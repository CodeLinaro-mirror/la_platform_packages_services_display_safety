// Copyright 2024 Google LLC

use grpcio::{RpcContext, UnarySink};
use harry_vehicle_data_grpc::vehicledata_grpc_service::*;
use harry_vehicle_data_grpc::vehicledata_grpc_service_grpc::*;

use crate::model::current_gear_request_to_message;
use crate::model::telltale_status_request_to_message;
use crate::model::tire_pressure_request_to_message;
use crate::model::vehicle_speed_request_to_message;
use crate::Error;
use crate::Error::Internal;
use crate::Error::Protocol;
use crate::Error::Sdv;
use futures::FutureExt;
use futures::TryFutureExt;
use log::error;
use log::warn;
use sdvgenerated::harry_vehicle_data_publisher::HarryVehicleDataPublisher;
use std::sync::Arc;
use std::sync::Mutex;

/// Implements a GRPC server and publishes all incoming requests as topics to the SDV Data Tunnel.
#[derive(Clone)]
pub struct VehicleDataGrpcServer {
    service: Arc<Mutex<HarryVehicleDataPublisher>>,
}

impl SdvVehicleDataGrpc for VehicleDataGrpcServer {
    fn publish_vehicle_speed(
        &mut self,
        ctx: RpcContext<'_>,
        req: PublishVehicleSpeedRequest,
        sink: UnarySink<PublishVehicleDataResponse>,
    ) {
        let status = match self.service.lock() {
            Ok(mut service) => {
                vehicle_speed_request_to_message(req).and_then(|(topic, message)| {
                    service.vehicle_speed_publish(&topic, &message).map_err(|err| err.into())
                })
            }
            Err(err) => {
                error!("Error locking service mutex: {:?}", err);
                Err(Error::Internal("Lock error".into()))
            }
        };
        self.send_to_sdv(ctx, sink, Self::convert_result_to_status(status));
    }

    fn publish_telltale_status(
        &mut self,
        ctx: RpcContext<'_>,
        req: PublishTelltaleStatusRequest,
        sink: UnarySink<PublishVehicleDataResponse>,
    ) {
        let status = match self.service.lock() {
            Ok(mut service) => {
                telltale_status_request_to_message(req).and_then(|(topic, message)| {
                    service.tell_tale_status_publish(&topic, &message).map_err(|err| err.into())
                })
            }
            Err(err) => {
                error!("Error locking service mutex: {:?}", err);
                Err(Error::Internal("Lock error".into()))
            }
        };
        self.send_to_sdv(ctx, sink, Self::convert_result_to_status(status));
    }

    fn publish_current_gear(
        &mut self,
        ctx: RpcContext<'_>,
        req: PublishCurrentGearRequest,
        sink: UnarySink<PublishVehicleDataResponse>,
    ) {
        let status = match self.service.lock() {
            Ok(mut service) => current_gear_request_to_message(req).and_then(|(topic, message)| {
                service.current_gear_publish(&topic, &message).map_err(|err| err.into())
            }),
            Err(err) => {
                error!("Error locking service mutex: {:?}", err);
                Err(Error::Internal("Lock error".into()))
            }
        };
        self.send_to_sdv(ctx, sink, Self::convert_result_to_status(status));
    }

    fn publish_tire_pressure(
        &mut self,
        ctx: RpcContext<'_>,
        req: PublishTirePressureRequest,
        sink: UnarySink<PublishVehicleDataResponse>,
    ) {
        let status = match self.service.lock() {
            Ok(mut service) => {
                tire_pressure_request_to_message(req).and_then(|(topic, message)| {
                    service.tire_pressure_publish(&topic, &message).map_err(|err| err.into())
                })
            }
            Err(err) => {
                error!("Error locking service mutex: {:?}", err);
                Err(Error::Internal("Lock error".into()))
            }
        };
        self.send_to_sdv(ctx, sink, Self::convert_result_to_status(status));
    }
}

impl VehicleDataGrpcServer {
    /// Creates a new GRPC server.
    ///
    /// * `service`: The SDV service to use to dispatch messages to.
    pub fn new(service: HarryVehicleDataPublisher) -> Self {
        Self { service: Arc::new(Mutex::new(service)) }
    }

    fn send_to_sdv(
        &self,
        ctx: RpcContext<'_>,
        sink: UnarySink<PublishVehicleDataResponse>,
        status: PublishVehicleDataResponseStatus,
    ) {
        let response = PublishVehicleDataResponse { status: status.into(), ..Default::default() };
        let future =
            sink.success(response).map_err(move |e| error!("failed to reply: {:?}", e)).map(|_| ());
        ctx.spawn(future);
    }

    fn convert_result_to_status(status: Result<(), Error>) -> PublishVehicleDataResponseStatus {
        match status {
            Err(Sdv(err)) => {
                warn!("Error publishing data to SDV DT: {:?}", err);
                PublishVehicleDataResponseStatus::STATUS_SDV_ERROR
            }
            Err(Protocol(err)) => {
                warn!("Error parsing request: {:?}", err);
                PublishVehicleDataResponseStatus::STATUS_PROTOCOL_ERROR
            }
            Err(Internal(err)) => {
                warn!("Internal error: {:?}", err);
                PublishVehicleDataResponseStatus::STATUS_NOT_DELIVERED
            }
            Ok(()) => PublishVehicleDataResponseStatus::STATUS_OK,
        }
    }
}
