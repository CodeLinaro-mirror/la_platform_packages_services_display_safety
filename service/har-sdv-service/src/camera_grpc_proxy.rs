// Copyright 2024 Google LLC

use crate::common::GrpcProxyServerToken;
use futures_util::FutureExt;
use futures_util::TryFutureExt;
use grpcio::ChannelBuilder;
use grpcio::Environment;
use grpcio::ResourceQuota;
use grpcio::ServerBuilder;
use grpcio::ServerCredentials;
use har_grpc_services::driverui::*;
use har_grpc_services::driverui_grpc::create_driver_ui_service;
use har_grpc_services::driverui_grpc::DriverUiService;
use har_grpc_services::driverui_grpc::DriverUiServiceClient;
use log::{error, trace, warn};
use std::sync::Arc;

/// A simple GRPC based proxy solution for the DriverUI GRPC service.
/// Will start a GRPC server and connect to a Client using
/// the same RPC definition and dispatch all requests
pub struct CameraServiceGrpcProxy {
    server_address: String,
}

impl CameraServiceGrpcProxy {
    /// Creates a new instance.
    /// * `server_address`: The address of the proxy server to start.
    pub fn new(server_address: String) -> Self {
        Self { server_address }
    }

    /// Starts the proxy server
    /// * `env`: The server runtime environment
    /// * `channel_to_har`: The GRPC channel to HAR.
    /// * returns the server token that can be used to stop the server.
    pub fn run(
        &self,
        env: Arc<Environment>,
        channel_to_har: ::grpcio::Channel,
    ) -> Result<GrpcProxyServerToken, String> {
        let rpc_client = DriverUiServiceClient::new(channel_to_har);

        // Create server
        let quota =
            ResourceQuota::new(Some("CameraServiceGrpcProxyQuota")).resize_memory(1024 * 1024);
        let server_ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

        let service = create_driver_ui_service(CameraServiceServer { rpc_client });
        let mut server = ServerBuilder::new(env.clone())
            .register_service(service)
            .channel_args(server_ch_builder.build_args())
            .build()
            .unwrap();
        server
            .add_listening_port(self.server_address.as_str(), ServerCredentials::insecure())
            .map_err(|err| format!("Error adding listening port. {:?}", err))?;
        server.start();
        Ok(GrpcProxyServerToken(server))
    }
}

#[derive(Clone)]
struct CameraServiceServer {
    rpc_client: DriverUiServiceClient,
}

impl DriverUiService for CameraServiceServer {
    fn heartbeat(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: HeartbeatRequest,
        sink: ::grpcio::UnarySink<HeartbeatResponse>,
    ) {
        trace!("Received heart beat request to send over to HAR: {:?}", &req);
        match self.rpc_client.heartbeat(&req) {
            Ok(response) => {
                trace!("Received heartbeat response {:?}", &response);
                let future = sink
                    .success(response)
                    .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e))
                    .map(|_| ());
                ctx.spawn(future);
            }
            Err(err) => {
                warn!("Error dispatching {:?}: {:?}", req, err);
            }
        }
    }
}
