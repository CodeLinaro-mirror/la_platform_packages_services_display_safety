// Copyright 2024 Google LLC

//! A simple stub program to simulate harry-app.
//! It will start the listening services and log all RPC calls.

use crate::test_log::TestLog;
use futures::executor::block_on;
use futures::FutureExt;
use futures::SinkExt;
use futures::StreamExt;
use futures::TryFutureExt;
use grpcio::ChannelBuilder;
use grpcio::DuplexSink;
use grpcio::EnvBuilder;
use grpcio::Environment;
use grpcio::RequestStream;
use grpcio::ResourceQuota;
use grpcio::RpcContext;
use grpcio::Server;
use grpcio::ServerBuilder;
use grpcio::ServerCredentials;
use grpcio::WriteFlags;
use har_grpc_services::driverui::DesignTokenUpdateRequest;
use har_grpc_services::driverui::DesignTokenUpdateResponse;
use har_grpc_services::driverui::DocumentSwitchedRequest;
use har_grpc_services::driverui::DocumentSwitchedResponse;
use har_grpc_services::driverui::DocumentUpdatedRequest;
use har_grpc_services::driverui::DocumentUpdatedResponse;
use har_grpc_services::driverui::HeartbeatRequest;
use har_grpc_services::driverui::HeartbeatResponse;
use har_grpc_services::driverui::LocaleUpdateRequest;
use har_grpc_services::driverui::LocaleUpdateResponse;
use har_grpc_services::driverui_grpc::create_driver_ui_service;
use har_grpc_services::driverui_grpc::DriverUiService;
use har_grpc_services::vehicledata::VehicleData;
use har_grpc_services::vehicledata::VehicleDataStreamResponse;
use har_grpc_services::vehicledata_grpc::create_vehicle_data_service;
use har_grpc_services::vehicledata_grpc::VehicleDataService;
use har_tracing_common::har_tracing::HarTracing;
use log::error;
use log::info;
use log::warn;
use std::sync::Arc;
use tokio::time::sleep;
use tokio::time::Duration;
use tracing::instrument;

mod test_log;

// Property to set after the GRPC service was started.
const HARRY_GRPC_STARTED_PROPERTY: &str = "vendor.harplatform.grpc.started";

#[derive(Clone)]
struct StubHarryServerBuilder {
    server_address: String,
}
impl HarTracing for StubHarryServerBuilder {}

impl StubHarryServerBuilder {
    /// Creates a new stub server instance.
    pub fn new(server_address: String) -> Self {
        Self { server_address }
    }

    /// Starts the server
    /// - `env`: The runtime environment.
    #[instrument(skip_all)]
    pub fn start_server(
        &self,
        env: Arc<Environment>,
        ports: &[u32],
    ) -> Result<HarryStubServerToken, String> {
        let vehicle_data_service = create_vehicle_data_service(StubHarryServer {});
        let driverui_service = create_driver_ui_service(StubHarryServer {});

        // Create server
        let quota = ResourceQuota::new(Some("StubHarryServerQuota")).resize_memory(1024 * 1024);
        let server_ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

        let mut server = ServerBuilder::new(env.clone())
            .register_service(vehicle_data_service)
            .register_service(driverui_service)
            .channel_args(server_ch_builder.build_args())
            .build()
            .map_err(|e| format!("Error creating server: {:?}", e))?;
        for port in ports {
            info!("Listening on {}:{}", self.server_address, port);
            server
                .add_listening_port(
                    format!("{}:{}", self.server_address, port),
                    ServerCredentials::insecure(),
                )
                .map_err(|e| format!("Error setting up listening port: {:?}", e))?;
        }

        server.start();
        Ok(HarryStubServerToken(server))
    }
}

#[derive(Clone)]
struct StubHarryServer {}

impl VehicleDataService for StubHarryServer {
    #[instrument(skip_all)]
    fn receive_vehicle_data(
        &mut self,
        ctx: RpcContext,
        mut stream: RequestStream<VehicleData>,
        mut sink: DuplexSink<VehicleDataStreamResponse>,
    ) {
        info!("Vehicle data stub server received new connection");

        ctx.spawn(async move {
            while let Some(data) = stream.next().await {
                if let Ok(data) = data {
                    TestLog::on_vehicle_data_processed(&data);
                } else {
                    warn!("Error processing received vehicle data{:?}", data);
                }
                let response =
                    VehicleDataStreamResponse { message: "OK".to_string(), ..Default::default() };
                sink.send((response, WriteFlags::default()))
                    .map_err(move |e| error!("failed to reply: {:?}", e))
                    .map(|_| ())
                    .await;
            }
            if let Err(err) = sink.close().await {
                warn!("Error closing response channel: {:?}", err);
            }
        });

        info!("Vehicle data stub server completed request");
    }
}

impl DriverUiService for StubHarryServer {
    #[instrument(skip_all)]
    fn heartbeat(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: HeartbeatRequest,
        sink: ::grpcio::UnarySink<HeartbeatResponse>,
    ) {
        TestLog::on_driverui_heartbeat_processed(&req);

        ctx.spawn(
            sink.success(HeartbeatResponse::default())
                .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e))
                .map(|_| ()),
        );
    }

    #[instrument(skip_all)]
    fn document_switched(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: DocumentSwitchedRequest,
        sink: ::grpcio::UnarySink<DocumentSwitchedResponse>,
    ) {
        TestLog::on_driverui_documentswitched_processed(&req);
        ctx.spawn(
            sink.success(DocumentSwitchedResponse::default())
                .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e))
                .map(|_| ()),
        );
    }

    #[instrument(skip_all)]
    fn document_updated(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: DocumentUpdatedRequest,
        sink: ::grpcio::UnarySink<DocumentUpdatedResponse>,
    ) {
        TestLog::on_driverui_documentupdated_processed(&req);
        ctx.spawn(
            sink.success(DocumentUpdatedResponse::default())
                .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e))
                .map(|_| ()),
        );
    }

    #[instrument(skip_all)]
    fn design_token_update(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: DesignTokenUpdateRequest,
        sink: ::grpcio::UnarySink<DesignTokenUpdateResponse>,
    ) {
        TestLog::on_driverui_designtokenupdate_processed(&req);
        ctx.spawn(
            sink.success(DesignTokenUpdateResponse::default())
                .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e))
                .map(|_| ()),
        );
    }

    #[instrument(skip_all)]
    fn locale_update(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: LocaleUpdateRequest,
        sink: ::grpcio::UnarySink<LocaleUpdateResponse>,
    ) {
        TestLog::on_driverui_localeupdate_processed(&req);
        ctx.spawn(
            sink.success(LocaleUpdateResponse::default())
                .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e))
                .map(|_| ()),
        );
    }
}

/// A token for a running server. Dropping this will stop the server.
pub struct HarryStubServerToken(Server);

impl HarryStubServerToken {
    /// Shutdown the server
    pub fn shutdown(mut self) {
        info!("Server is shutting down");
        let _ = block_on(self.0.shutdown());
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    sdv_log::init_logger("harry-app").unwrap();
    info!("Harry-app stub started.");
    let address = "0.0.0.0";
    let ports = [50051, 7001, 8001];

    let env = Arc::new(EnvBuilder::new().build());

    let server_builder = StubHarryServerBuilder::new(address.to_string());
    let _server = server_builder.start_server(env, &ports)?;

    info!("Server started on {:?}", address);

    // Set the same property that the Harry-app would set after starting the GRPC server.
    // This will trigger init.rc to start other dependencies.
    if let Err(e) = rustutils::system_properties::write(HARRY_GRPC_STARTED_PROPERTY, "true") {
        log::error!("Error setting system property {}: {:?}", HARRY_GRPC_STARTED_PROPERTY, e);
    }

    // Loop forever, we never want to intentionally exit this process.
    loop {
        sleep(Duration::from_secs(10)).await;
    }
}
