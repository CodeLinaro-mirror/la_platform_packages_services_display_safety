// Copyright 2024 Google LLC

use futures::executor::block_on;
use futures_util::FutureExt;
use futures_util::TryFutureExt;
use grpcio::ChannelBuilder;
use grpcio::EnvBuilder;
use grpcio::Environment;
use grpcio::ResourceQuota;
use grpcio::Server;
use grpcio::ServerBuilder;
use grpcio::ServerCredentials;
use har_grpc_services::driverui::*;
use har_grpc_services::driverui_grpc::create_harry_grpc_service;
use har_grpc_services::driverui_grpc::HarryGrpcService;
use har_grpc_services::driverui_grpc::HarryGrpcServiceClient;
use log::{error, trace, warn};
use std::sync::Arc;
use std::time::Duration;

/// A simple GRPC based proxy solution for the DriverUI GRPC service.
/// Will start a GRPC server and connect to a Client using
/// the same RPC definition and dispatch all requests
pub struct CameraServiceGrpcProxy {
    server_address: String,
    client_address: String,
}

/// A token for a running server. Dropping this will stop the server.
pub struct GrpcProxyServerToken(Server);

impl GrpcProxyServerToken {
    /// Shutdown the server
    pub fn shutdown(mut self) {
        let _ = block_on(self.0.shutdown());
    }
}

impl CameraServiceGrpcProxy {
    /// Creates a new instance.
    /// * `server_address`: The address of the proxy server to start.
    /// * `client_address`: The address of the remote server to proxy to.
    pub fn new(server_address: String, client_address: String) -> Self {
        Self { server_address, client_address }
    }

    /// Starts the proxy server * `env`: The server runtime environment * returns the server token that can be used to stop the server.
    pub fn run(&self, env: Arc<Environment>) -> GrpcProxyServerToken {
        // Create client. Client needs a dedicated env otherwise it deadlocks.
        let client_ch = ChannelBuilder::new(Arc::new(EnvBuilder::new().build()))
            .initial_reconnect_backoff(Duration::from_millis(10))
            .max_reconnect_backoff(Duration::from_millis(50))
            .connect(self.client_address.as_str());
        let rpc_client = HarryGrpcServiceClient::new(client_ch);

        // Create server
        let quota =
            ResourceQuota::new(Some("CameraServiceGrpcProxyQuota")).resize_memory(1024 * 1024);
        let server_ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

        let service = create_harry_grpc_service(CameraServiceServer { rpc_client });
        let mut server = ServerBuilder::new(env.clone())
            .register_service(service)
            .channel_args(server_ch_builder.build_args())
            .build()
            .unwrap();
        server
            .add_listening_port(self.server_address.as_str(), ServerCredentials::insecure())
            .unwrap();
        server.start();
        GrpcProxyServerToken(server)
    }
}

#[derive(Clone)]
struct CameraServiceServer {
    rpc_client: HarryGrpcServiceClient,
}

impl HarryGrpcService for CameraServiceServer {
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
