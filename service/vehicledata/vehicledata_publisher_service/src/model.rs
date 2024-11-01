// Copyright 2024 Google LLC

use harry_vehicle_data_grpc::vehicledata_grpc_service::*;

use harry_vehicle_data_grpc::vehicle_tire::Location;
use harry_vehicle_data_grpc::vehicledata::Gear;
use harry_vehicle_data_grpc::vehicledata::Telltale;
use harry_vehicle_data_grpc::vehicledata::VehicleSpeedTopic;
use oem_harry_vehicle_messages_catalog_v1::vehicle_tire::TirePressure;

use oem_harry_vehicle_messages_catalog_v1::vehicledata::CurrentGear;
use oem_harry_vehicle_messages_catalog_v1::vehicledata::VehicleSpeed;

use thiserror::Error;

use oem_harry_vehicle_messages_catalog_v1::vehicledata::TellTaleStatus;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error parsing the request {0}")]
    Protocol(String),
    #[error("Cannot deliver the request {0}")]
    Internal(String),
}

/// Converts the GRPC PublishCurrentGearRequest to an SDV Data tunnel message.
pub fn current_gear_request_to_message(
    req: PublishCurrentGearRequest,
) -> Result<CurrentGear, Error> {
    let gear = req
        .gear
        .gear
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to gear {:?}", err)))?;

    let mut message = CurrentGear::new();
    message.gear = match gear {
        Gear::P => oem_harry_vehicle_messages_catalog_v1::vehicledata::Gear::P,
        Gear::R => oem_harry_vehicle_messages_catalog_v1::vehicledata::Gear::R,
        Gear::N => oem_harry_vehicle_messages_catalog_v1::vehicledata::Gear::N,
        Gear::D => oem_harry_vehicle_messages_catalog_v1::vehicledata::Gear::D,
    }
    .into();
    Ok(message)
}

/// Converts the GRPC PublishVehicleSpeedRequest to an SDV Data tunnel message.
pub fn vehicle_speed_request_to_message(
    req: PublishVehicleSpeedRequest,
) -> Result<(VehicleSpeedTopic, VehicleSpeed), Error> {
    let topic = req
        .topic
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to vehicle speed topic {:?}", err)))?;

    let mut message = VehicleSpeed::new();
    message.speed = req.speed.speed;
    Ok((topic, message))
}

/// Converts the GRPC PublishTelltaleStatusRequest to an SDV Data tunnel message.
pub fn telltale_status_request_to_message(
    req: PublishTelltaleStatusRequest,
) -> Result<(Telltale, TellTaleStatus), Error> {
    let variant: Telltale = req
        .topic
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to telltale topic {:?}", err)))?;

    let mut message = TellTaleStatus::new();
    message.alert = req.status.alert;
    Ok((variant, message))
}

/// Converts the GRPC PublishTirePressureRequest to an SDV Data tunnel message.
pub fn tire_pressure_request_to_message(
    req: PublishTirePressureRequest,
) -> Result<(Location, TirePressure), Error> {
    let topic = req
        .topic
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to tire pressure topic {:?}", err)))?;
    let mut message = TirePressure::new();
    message.pressure = req.pressure.pressure;
    Ok((topic, message))
}
