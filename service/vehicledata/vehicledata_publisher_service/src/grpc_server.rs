// Copyright 2024 Google LLC

use grpcio::{RpcContext, UnarySink};
use harry_vehicle_data_grpc::vehicledata_grpc_service::*;
use harry_vehicle_data_grpc::vehicledata_grpc_service_grpc::*;

use crate::model::current_gear_request_to_message;
use crate::model::telltale_status_request_to_message;
use crate::model::tire_pressure_request_to_message;
use crate::model::vehicle_speed_request_to_message;
use crate::model::Error;
use crate::HarryVehicleDataPublishers;
use log::error;
use log::info;
use log::warn;
use std::sync::Arc;
use tracing::instrument;

/// Implements a GRPC server and publishes all incoming requests as topics to the SDV Data Tunnel.
#[derive(Clone)]
pub struct VehicleDataGrpcServer {
    service: Arc<HarryVehicleDataPublishers>,
}

impl SdvVehicleDataGrpc for VehicleDataGrpcServer {
    #[instrument(skip_all)]
    fn publish_vehicle_speed(
        &mut self,
        ctx: RpcContext<'_>,
        req: PublishVehicleSpeedRequest,
        sink: UnarySink<PublishVehicleDataResponse>,
    ) {
        info!("publish_vehicle_speed {:?}", req);
        let service = self.service.clone();
        match vehicle_speed_request_to_message(req) {
            Ok((status, message)) => ctx.spawn(async move {
                service.vehicle_speed_publish(status, message).await;
                Self::send_response(sink, Self::convert_result_to_status(Ok(())))
                    .await
                    .unwrap_or_else(|err| error!("Error sending GRPC response: {:?}", err));
            }),
            Err(err) => ctx.spawn(async move {
                Self::send_response(
                    sink,
                    Self::convert_result_to_status(Err(format!(
                        "Error converting GRPC message: {:?}",
                        err
                    ))),
                )
                .await
                .unwrap_or_else(|err| error!("Error sending GRPC error response: {:?}", err));
            }),
        };
    }

    #[instrument(skip_all)]
    fn publish_telltale_status(
        &mut self,
        ctx: RpcContext<'_>,
        req: PublishTelltaleStatusRequest,
        sink: UnarySink<PublishVehicleDataResponse>,
    ) {
        info!("publish_telltale_status {:?}", req);
        let service = self.service.clone();

        match telltale_status_request_to_message(req) {
            Ok((status, message)) => ctx.spawn(async move {
                service.tell_tale_status_publish(status, message).await;
                Self::send_response(sink, Self::convert_result_to_status(Ok(())))
                    .await
                    .unwrap_or_else(|err| error!("Error sending GRPC response: {:?}", err));
            }),
            Err(err) => ctx.spawn(async move {
                Self::send_response(
                    sink,
                    Self::convert_result_to_status(Err(format!(
                        "Error converting GRPC message: {:?}",
                        err
                    ))),
                )
                .await
                .unwrap_or_else(|err| error!("Error sending GRPC error response: {:?}", err));
            }),
        };
    }

    #[instrument(skip_all)]
    fn publish_current_gear(
        &mut self,
        ctx: RpcContext<'_>,
        req: PublishCurrentGearRequest,
        sink: UnarySink<PublishVehicleDataResponse>,
    ) {
        info!("publish_current_gear {:?}", req);
        let service = self.service.clone();

        match current_gear_request_to_message(req) {
            Ok(message) => ctx.spawn(async move {
                service.current_gear_publish(message).await;
                Self::send_response(sink, Self::convert_result_to_status(Ok(())))
                    .await
                    .unwrap_or_else(|err| error!("Error sending GRPC response: {:?}", err));
            }),
            Err(err) => ctx.spawn(async move {
                Self::send_response(
                    sink,
                    Self::convert_result_to_status(Err(format!(
                        "Error converting GRPC message: {:?}",
                        err
                    ))),
                )
                .await
                .unwrap_or_else(|err| error!("Error sending GRPC error response: {:?}", err));
            }),
        };
    }

    #[instrument(skip_all)]
    fn publish_tire_pressure(
        &mut self,
        ctx: RpcContext<'_>,
        req: PublishTirePressureRequest,
        sink: UnarySink<PublishVehicleDataResponse>,
    ) {
        info!("publish_tire_pressure {:?}", req);
        let service = self.service.clone();

        match tire_pressure_request_to_message(req) {
            Ok((location, message)) => ctx.spawn(async move {
                service.tire_pressure_publish(location, message).await;
                Self::send_response(sink, Self::convert_result_to_status(Ok(())))
                    .await
                    .unwrap_or_else(|err| error!("Error sending GRPC response: {:?}", err));
            }),
            Err(err) => ctx.spawn(async move {
                Self::send_response(
                    sink,
                    Self::convert_result_to_status(Err(format!(
                        "Error converting GRPC message: {:?}",
                        err
                    ))),
                )
                .await
                .unwrap_or_else(|err| error!("Error sending GRPC error response: {:?}", err));
            }),
        };
    }
}

impl VehicleDataGrpcServer {
    /// Creates a new GRPC server.
    ///
    /// * `service`: The SDV service to use to dispatch messages to.
    pub fn new(service: HarryVehicleDataPublishers) -> Self {
        Self { service: Arc::new(service) }
    }

    #[instrument(skip_all)]
    async fn send_response(
        sink: UnarySink<PublishVehicleDataResponse>,
        status: PublishVehicleDataResponseStatus,
    ) -> Result<(), Error> {
        let response = PublishVehicleDataResponse { status: status.into(), ..Default::default() };
        sink.success(response)
            .await
            .map_err(move |e| Error::Internal(format!("failed to reply: {:?}", e)))
            .map(|_| ())
    }

    #[instrument(skip_all)]
    fn convert_result_to_status(status: Result<(), String>) -> PublishVehicleDataResponseStatus {
        match status {
            Err(err) => {
                warn!("Internal error: {:?}", err);
                PublishVehicleDataResponseStatus::STATUS_NOT_DELIVERED
            }
            Ok(()) => PublishVehicleDataResponseStatus::STATUS_OK,
        }
    }
}
