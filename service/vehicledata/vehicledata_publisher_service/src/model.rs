// Copyright 2024 Google LLC

use harry_vehicle_data_grpc::vehicledata_grpc_service::*;

use harry_vehicle_data_grpc::vehicle_tire::Location;
use harry_vehicle_data_grpc::vehicledata::CurrentGearTopic;
use harry_vehicle_data_grpc::vehicledata::Gear;
use harry_vehicle_data_grpc::vehicledata::Telltale;
use harry_vehicle_data_grpc::vehicledata::VehicleSpeedTopic;
use oem_harry_vehicle_messages_catalog::vehicle_tire::TirePressure;
use oem_harry_vehicle_messages_catalog::vehicledata::CurrentGear;
use oem_harry_vehicle_messages_catalog::vehicledata::TellTaleStatus;
use oem_harry_vehicle_messages_catalog::vehicledata::VehicleSpeed;
use sdvgenerated::topics::publisher::CurrentGearTopics;
use sdvgenerated::topics::publisher::TellTaleStatusTopics;
use sdvgenerated::topics::publisher::TirePressureTopics;
use sdvgenerated::topics::publisher::VehicleSpeedTopics;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("SDV middleware error: {0:?}")]
    Sdv(sdv_middleware_pubsub::error::Error),
    #[error("Error parsing the request {0}")]
    Protocol(String),
    #[error("Cannot deliver the request {0}")]
    Internal(String),
}

impl From<sdv_middleware_pubsub::error::Error> for Error {
    fn from(err: sdv_middleware_pubsub::error::Error) -> Self {
        Error::Sdv(err)
    }
}

/// Converts the GRPC PublishCurrentGearRequest to an SDV Data tunnel message.
pub fn current_gear_request_to_message(
    req: PublishCurrentGearRequest,
) -> Result<(CurrentGearTopics, CurrentGear), Error> {
    let topic = req
        .topic
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to gear topic {:?}", err)))
        .map(|topic| match topic {
            CurrentGearTopic::GEAR => CurrentGearTopics::GEAR,
        })?;

    let gear = req
        .gear
        .gear
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to gear {:?}", err)))
        .map(|gear| match gear {
            Gear::P => oem_harry_vehicle_messages_catalog::vehicledata::Gear::P,
            Gear::R => oem_harry_vehicle_messages_catalog::vehicledata::Gear::R,
            Gear::N => oem_harry_vehicle_messages_catalog::vehicledata::Gear::N,
            Gear::D => oem_harry_vehicle_messages_catalog::vehicledata::Gear::D,
        })?;

    let mut message = CurrentGear::new();
    message.gear = gear.into();
    Ok((topic, message))
}

/// Converts the GRPC PublishVehicleSpeedRequest to an SDV Data tunnel message.
pub fn vehicle_speed_request_to_message(
    req: PublishVehicleSpeedRequest,
) -> Result<(VehicleSpeedTopics, VehicleSpeed), Error> {
    let topic = req
        .topic
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to vehicle speed topic {:?}", err)))
        .map(|topic| match topic {
            VehicleSpeedTopic::VEHICLE_SPEED => VehicleSpeedTopics::VEHICLE_SPEED,
            VehicleSpeedTopic::MAX_SPEED => VehicleSpeedTopics::MAX_SPEED,
            VehicleSpeedTopic::SPEEDLIMIT => VehicleSpeedTopics::SPEEDLIMIT,
        })?;

    let mut message = VehicleSpeed::new();
    message.speed = req.speed.speed;
    Ok((topic, message))
}

/// Converts the GRPC PublishTelltaleStatusRequest to an SDV Data tunnel message.
pub fn telltale_status_request_to_message(
    req: PublishTelltaleStatusRequest,
) -> Result<(TellTaleStatusTopics, TellTaleStatus), Error> {
    let topic = req
        .topic
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to telltale topic {:?}", err)))
        .map(|telltale| match telltale {
            Telltale::OIL_PRESSURE => TellTaleStatusTopics::OIL_PRESSURE,
            Telltale::ENGINE_TEMP => TellTaleStatusTopics::ENGINE_TEMP,
            Telltale::CHECK_ENGINE => TellTaleStatusTopics::CHECK_ENGINE,
            Telltale::CHARGING_FAILURE => TellTaleStatusTopics::CHARGING_FAILURE,
            Telltale::SEATBELT_DRIVER => TellTaleStatusTopics::SEATBELT_DRIVER,
            Telltale::SEATBELT_PASSENGER => TellTaleStatusTopics::SEATBELT_PASSENGER,
            Telltale::LOW_TIRE_PRESSURE => TellTaleStatusTopics::LOW_TIRE_PRESSURE,
            Telltale::AIRBAG => TellTaleStatusTopics::AIRBAG,
            Telltale::ABS => TellTaleStatusTopics::ABS,
            Telltale::BRAKE => TellTaleStatusTopics::BRAKE,
            Telltale::TRACTION => TellTaleStatusTopics::TRACTION,
            Telltale::FOG_LIGHTS => TellTaleStatusTopics::FOG_LIGHTS,
            Telltale::PARK_LIGHTS => TellTaleStatusTopics::PARK_LIGHTS,
            Telltale::HIBEAM => TellTaleStatusTopics::HIBEAM,
            Telltale::LOWBEAM => TellTaleStatusTopics::LOWBEAM,
            Telltale::TURN_SIGNAL_LEFT => TellTaleStatusTopics::TURN_SIGNAL_LEFT,
            Telltale::TURN_SIGNAL_RIGHT => TellTaleStatusTopics::TURN_SIGNAL_RIGHT,
            Telltale::ADAS => TellTaleStatusTopics::ADAS,
            Telltale::MAX_SPEED_DISPLAYED => TellTaleStatusTopics::MAX_SPEED_DISPLAYED,
            Telltale::SPEED_LIMIT_DISPLAYED => TellTaleStatusTopics::SPEED_LIMIT_DISPLAYED,
            Telltale::EMERGENCY_LIGHT => TellTaleStatusTopics::EMERGENCY_LIGHT,
        })?;

    let mut message = TellTaleStatus::new();
    message.alert = req.status.alert;
    Ok((topic, message))
}

/// Converts the GRPC PublishTirePressureRequest to an SDV Data tunnel message.
pub fn tire_pressure_request_to_message(
    req: PublishTirePressureRequest,
) -> Result<(TirePressureTopics, TirePressure), Error> {
    let topic = req
        .topic
        .enum_value()
        .map_err(|err| Error::Protocol(format!("Cannot map to tire pressure topic {:?}", err)))
        .map(|tire_pressure| match tire_pressure {
            Location::FRONT_LEFT => TirePressureTopics::FRONT_LEFT,
            Location::FRONT_RIGHT => TirePressureTopics::FRONT_RIGHT,
            Location::REAR_LEFT => TirePressureTopics::REAR_LEFT,
            Location::REAR_RIGHT => TirePressureTopics::REAR_RIGHT,
            Location::FIFTH_WHEEL => TirePressureTopics::FIFTH_WHEEL,
        })?;
    let mut message = TirePressure::new();
    message.pressure = req.pressure.pressure;
    Ok((topic, message))
}
