// Copyright 2024 Google LLC

use har_grpc_services::vehicledata::vehicle_data;
use har_grpc_services::vehicledata::vehicle_data::Data::*;
use har_grpc_services::vehicledata::VehicleData;
use log::info;
use std::fmt::{Debug, Display};

const DEFAULT_TAG: &str = "CI_TEST_INFO";

/// Logs events using a stable format that can be used during testing.
#[derive(Copy, Clone, Debug)]
pub struct TestLog;

/// A basic implementation to log events we expect in tests.
// TODO(365843614): Implement a way to enable/disable this to avoid log spam.
impl TestLog {
    /// Call when the application state was changed due to an action.
    pub fn info(state_name: &String, value: &impl Debug, message: &(impl Display + ?Sized)) {
        info!("{}: {state_name:?}={value:?}; {}", DEFAULT_TAG, message);
    }

    /// Prints a log after processing a vehicle data fragment.
    pub fn on_vehicle_data_processed(vehicle_data: &VehicleData) {
        if let Some(value) = vehicle_data.data.as_ref() {
            Self::info(&vehicle_data.name, &vehicle_data_to_debug(value), "processed");
        }
    }
}

fn vehicle_data_to_debug(data: &vehicle_data::Data) -> Box<dyn Debug> {
    match data {
        DataBool(value) => Box::new(value.dataBool),
        DataI32(value) => Box::new(value.dataI32),
        DataU32(value) => Box::new(value.dataU32),
        DataF32(value) => Box::new(value.dataF32),
        DataF64(value) => Box::new(value.dataF64),
        DataU8(value) => Box::new(value.dataU8),
        DataString(value) => Box::new(value.dataString.clone()),
        DataBinary(value) => Box::new(value.dataBinary.clone()),
        other => Box::new(format!("Unexpected data: {:?}", other)),
    }
}
