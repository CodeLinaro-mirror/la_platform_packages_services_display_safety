// Copyright 2023 Google LLC

use crate::common::GrpcProxyServerToken;
use futures_util::FutureExt;
use futures_util::TryFutureExt;
use grpcio::ChannelBuilder;
use grpcio::Environment;
use grpcio::ResourceQuota;
use grpcio::ServerBuilder;
use grpcio::ServerCredentials;
use har_grpc_services::driverui::document_switched_response::Status as DocumentSwitchedResponseStatus;
use har_grpc_services::driverui::document_updated_response::Status as DocumentUpdatedResponseStatus;
use har_grpc_services::driverui::heartbeat_request::Source;
use har_grpc_services::driverui::heartbeat_response::Status as HeartbeatResponseStatus;
use har_grpc_services::driverui::*;
use har_grpc_services::driverui_grpc::create_driver_ui_service;
use har_grpc_services::driverui_grpc::DriverUiService;
use har_grpc_services::driverui_grpc::DriverUiServiceClient;
use log::info;
use log::{error, trace, warn};
use sdv::status::SdvStatus;
use sdv::status::SdvStatusCode;
use std::sync::Arc;
use tracing::instrument;

// TODO(b/396229429): Remove this when MW Streaming is supported.
const MAX_MESSAGE_SIZE_BYTES: usize = 100_000_000;

/// A simple GRPC based proxy solution for the DriverUI GRPC service.
/// Will start a GRPC server and connect to a Client using
/// the same RPC definition and dispatch all requests
// Deprecated
pub struct DriverUiGrpcProxy {
    server_address: String,
}

impl DriverUiGrpcProxy {
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
    ) -> GrpcProxyServerToken {
        // Create client. Client needs a dedicated env otherwise it deadlocks.
        let rpc_client = DriverUiServiceClient::new(channel_to_har);

        // Create server
        let quota = ResourceQuota::new(Some("DriverUiGrpcProxyQuota")).resize_memory(1024 * 1024);
        let server_ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

        let service = create_driver_ui_service(DriverUiServer { rpc_client });
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

// DriverUI SDV-RPC to HAR's DriverUI GRPC Adapter.
pub struct DriverUiSdvRpcProxy {
    rpc_client: DriverUiServiceClient,
}

impl DriverUiSdvRpcProxy {
    /// Creates a new instance.
    /// - `channel_to_har`: The GRPC channel to the HAR app.
    pub fn new(channel_to_har: ::grpcio::Channel) -> DriverUiSdvRpcProxy {
        DriverUiSdvRpcProxy { rpc_client: DriverUiServiceClient::new(channel_to_har) }
    }
}

#[allow(non_snake_case)]
#[async_trait::async_trait]
impl com_sdv_google_display_safety_driver_ui_service_rpc::Interface for DriverUiSdvRpcProxy {
    #[instrument(skip_all, name = "heartbeat_driverui")]
    async fn Heartbeat(
        &self,
        _caller_id: sdv::comms::id::ServiceFqin,
        request: &oem_harry_vehicle_messages_catalog_v1::driverui::HeartbeatRequest,
    ) -> sdv::status::SdvResult<oem_harry_vehicle_messages_catalog_v1::driverui::HeartbeatResponse>
    {
        trace!("Received heart beat request to send over to HAR: {:?}", &request);
        match self.rpc_client.heartbeat(&HeartbeatRequest {
            uptime: request.uptime,
            source: match request.source.enum_value().map_err(|err| {
                error!("Error converting heart beat request source: {:?}", err);
                SdvStatus::new(SdvStatusCode::InvalidArgument)
            })? {
                oem_harry_vehicle_messages_catalog_v1::driverui::heartbeat_request::Source::SOURCE_UNKNOWN => Source::SOURCE_UNKNOWN,
                oem_harry_vehicle_messages_catalog_v1::driverui::heartbeat_request::Source::SOURCE_ANDROID => Source::SOURCE_ANDROID,
                oem_harry_vehicle_messages_catalog_v1::driverui::heartbeat_request::Source::SOURCE_INSTRUMENT_CLUSTER => Source::SOURCE_INSTRUMENT_CLUSTER,
                oem_harry_vehicle_messages_catalog_v1::driverui::heartbeat_request::Source::SOURCE_CAMERA_SERVICE => Source::SOURCE_CAMERA_SERVICE,
            }.into(),
            ..Default::default()
        }) {
            Ok(response) => {
                trace!("Received heartbeat response {:?}", &response);
                Ok(oem_harry_vehicle_messages_catalog_v1::driverui::HeartbeatResponse {
                    status: match response.status.enum_value().map_err(|err| {
                        error!("Error converting heart beat response status: {:?}", err);
                        SdvStatus::new(SdvStatusCode::InvalidArgument)
                    })? {
                        HeartbeatResponseStatus::STATUS_UNKNOWN => oem_harry_vehicle_messages_catalog_v1::driverui::heartbeat_response::Status::STATUS_UNKNOWN,
                        HeartbeatResponseStatus::STATUS_OK => oem_harry_vehicle_messages_catalog_v1::driverui::heartbeat_response::Status::STATUS_OK,
                        HeartbeatResponseStatus::STATUS_PROCESSING_ERROR => oem_harry_vehicle_messages_catalog_v1::driverui::heartbeat_response::Status::STATUS_PROCESSING_ERROR,
                    }.into(),
                    ..Default::default()
                })
            }
            Err(err) => {
                warn!("Error dispatching Heartbeat {:?}: {:?}", request, err);
                Err(SdvStatus::with_message(SdvStatusCode::Internal, format!("{:?}", err)))
            }
        }
    }

    #[instrument(skip_all)]
    async fn DocumentSwitched(
        &self,
        _caller_id: sdv::comms::id::ServiceFqin,
        request: &oem_harry_vehicle_messages_catalog_v1::driverui::DocumentSwitchedRequest,
    ) -> sdv::status::SdvResult<
        oem_harry_vehicle_messages_catalog_v1::driverui::DocumentSwitchedResponse,
    > {
        match self.rpc_client.document_switched(&DocumentSwitchedRequest {
            document_id: request.document_id.clone(),
            ..Default::default()
        }) {
            Ok(response) => {
                Ok(oem_harry_vehicle_messages_catalog_v1::driverui::DocumentSwitchedResponse {
                    status: match response.status.enum_value().map_err(|err| {
                        error!("Error converting document switch response status: {:?}", err);
                        SdvStatus::new(SdvStatusCode::InvalidArgument)
                    })? {
                        DocumentSwitchedResponseStatus::STATUS_UNKNOWN => oem_harry_vehicle_messages_catalog_v1::driverui::document_switched_response::Status::STATUS_UNKNOWN,
                        DocumentSwitchedResponseStatus::STATUS_OK => oem_harry_vehicle_messages_catalog_v1::driverui::document_switched_response::Status::STATUS_OK,
                        DocumentSwitchedResponseStatus::STATUS_PROCESSING_ERROR => oem_harry_vehicle_messages_catalog_v1::driverui::document_switched_response::Status::STATUS_PROCESSING_ERROR,
                    }.into(),
                    ..Default::default()
                })
            }
            Err(err) => {
                warn!("Error dispatching DocumentSwitched {:?}: {:?}", request, err);
                Err(SdvStatus::with_message(SdvStatusCode::Internal, format!("{:?}", err)))
            }
        }
    }

    #[instrument(skip_all)]
    async fn DocumentUpdated(
        &self,
        _caller_id: sdv::comms::id::ServiceFqin,
        request: &oem_harry_vehicle_messages_catalog_v1::driverui::DocumentUpdatedRequest,
    ) -> sdv::status::SdvResult<
        oem_harry_vehicle_messages_catalog_v1::driverui::DocumentUpdatedResponse,
    > {
        match self.rpc_client.document_updated(&DocumentUpdatedRequest {
            document_id: request.document_id.clone(),
            serialized_document: request.serialized_document.clone(),
            ..Default::default()
        }) {
            Ok(response) => {
                Ok(oem_harry_vehicle_messages_catalog_v1::driverui::DocumentUpdatedResponse {
                    status: match response.status.enum_value().map_err(|err| {
                        error!("Error converting document updated response status: {:?}", err);
                        SdvStatus::new(SdvStatusCode::InvalidArgument)
                    })? {
                        DocumentUpdatedResponseStatus::STATUS_UNKNOWN => oem_harry_vehicle_messages_catalog_v1::driverui::document_updated_response::Status::STATUS_UNKNOWN,
                        DocumentUpdatedResponseStatus::STATUS_OK => oem_harry_vehicle_messages_catalog_v1::driverui::document_updated_response::Status::STATUS_OK,
                        DocumentUpdatedResponseStatus::STATUS_PROCESSING_ERROR => oem_harry_vehicle_messages_catalog_v1::driverui::document_updated_response::Status::STATUS_PROCESSING_ERROR,
                    }.into(),
                    ..Default::default()
                })
            }
            Err(err) => {
                warn!("Error dispatching DocumentUpdated {:?}: {:?}", request, err);
                Err(SdvStatus::with_message(SdvStatusCode::Internal, format!("{:?}", err)))
            }
        }
    }

    #[instrument(skip_all)]
    async fn DesignTokenUpdate(
        &self,
        _caller_id: sdv::comms::id::ServiceFqin,
        request: &oem_harry_vehicle_messages_catalog_v1::driverui::DesignTokenUpdateRequest,
    ) -> sdv::status::SdvResult<
        oem_harry_vehicle_messages_catalog_v1::driverui::DesignTokenUpdateResponse,
    > {
        match self.rpc_client.design_token_update(&DesignTokenUpdateRequest {
            theme: request.theme.clone(),
            variable_mode: request.variable_mode.clone(),
            ..Default::default()
        }) {
            Ok(_response) => {
                Ok(oem_harry_vehicle_messages_catalog_v1::driverui::DesignTokenUpdateResponse {
                    ..Default::default()
                })
            }
            Err(err) => {
                warn!("Error dispatching DesignTokenUpdate {:?}: {:?}", request, err);
                Err(SdvStatus::with_message(SdvStatusCode::Internal, format!("{:?}", err)))
            }
        }
    }

    #[instrument(skip_all)]
    async fn LocaleUpdate(
        &self,
        _caller_id: sdv::comms::id::ServiceFqin,
        request: &oem_harry_vehicle_messages_catalog_v1::driverui::LocaleUpdateRequest,
    ) -> sdv::status::SdvResult<oem_harry_vehicle_messages_catalog_v1::driverui::LocaleUpdateResponse>
    {
        match self.rpc_client.locale_update(&LocaleUpdateRequest {
            language_tag: request.language_tag.clone(),
            ..Default::default()
        }) {
            Ok(_response) => {
                Ok(oem_harry_vehicle_messages_catalog_v1::driverui::LocaleUpdateResponse {
                    ..Default::default()
                })
            }
            Err(err) => {
                warn!("Error dispatching LocaleUpdate {:?}: {:?}", request, err);
                Err(SdvStatus::with_message(SdvStatusCode::Internal, format!("{:?}", err)))
            }
        }
    }

    #[instrument(skip_all)]
    fn init_server_options(&self) -> sdv::mw::ServerOptions {
        // Increasing RPC message limit to allow document updates.
        sdv::mw::ServerOptions::builder()
            .max_request_size(Some(MAX_MESSAGE_SIZE_BYTES))
            .timeout_in_sec(30)
            .build()
            .expect("Cannot build server options")
    }
}

#[derive(Clone)]
struct DriverUiServer {
    rpc_client: DriverUiServiceClient,
}

impl DriverUiService for DriverUiServer {
    #[instrument(skip_all)]
    fn heartbeat(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: HeartbeatRequest,
        sink: ::grpcio::UnarySink<HeartbeatResponse>,
    ) {
        info!("Received heart beat request to send over to HAR: {:?}", &req);
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
    #[instrument(skip_all)]
    fn document_switched(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: DocumentSwitchedRequest,
        sink: ::grpcio::UnarySink<DocumentSwitchedResponse>,
    ) {
        info!("Received document_switched: {:?}", &req);
        match self.rpc_client.document_switched(&req) {
            Ok(response) => {
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
    #[instrument(skip_all)]
    fn document_updated(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: DocumentUpdatedRequest,
        sink: ::grpcio::UnarySink<DocumentUpdatedResponse>,
    ) {
        info!("Received document_updated");
        match self.rpc_client.document_updated(&req) {
            Ok(response) => {
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
    #[instrument(skip_all)]
    fn design_token_update(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: DesignTokenUpdateRequest,
        sink: ::grpcio::UnarySink<DesignTokenUpdateResponse>,
    ) {
        info!("Received design_token_update: {:?}", &req);
        match self.rpc_client.design_token_update(&req) {
            Ok(response) => {
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
    #[instrument(skip_all)]
    fn locale_update(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: LocaleUpdateRequest,
        sink: ::grpcio::UnarySink<LocaleUpdateResponse>,
    ) {
        info!("Received locale_update: {:?}", &req);
        match self.rpc_client.locale_update(&req) {
            Ok(response) => {
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
