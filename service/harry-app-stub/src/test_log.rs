// Copyright 2024 Google LLC

use har_grpc_services::driverui::heartbeat_request::Source;
use har_grpc_services::driverui::DesignTokenUpdateRequest;
use har_grpc_services::driverui::DocumentSwitchedRequest;
use har_grpc_services::driverui::DocumentUpdatedRequest;
use har_grpc_services::driverui::HeartbeatRequest;
use har_grpc_services::driverui::LocaleUpdateRequest;
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

    /// Prints a log after processing a heart beat message.
    pub fn on_driverui_heartbeat_processed(req: &HeartbeatRequest) {
        Self::info(
            &"DriverUI:heartbeat::source".to_string(),
            &match req.source.enum_value().unwrap_or(Source::SOURCE_UNKNOWN) {
                Source::SOURCE_UNKNOWN => "UNKNOWN",
                Source::SOURCE_ANDROID => "ANDROID",
                Source::SOURCE_INSTRUMENT_CLUSTER => "INSTRUMENT_CLUSTER",
                Source::SOURCE_CAMERA_SERVICE => "CAMERA_SERVICE",
            }
            .to_string(),
            &format!("processed, uptime={}", req.uptime),
        );
    }

    /// Prints a log after processing a document switched message.
    pub fn on_driverui_documentswitched_processed(req: &DocumentSwitchedRequest) {
        Self::info(
            &"DriverUI:documentswitched".to_string(),
            &req.document_id.to_string(),
            "processed",
        );
    }

    /// Prints a log after processing a document updated message.
    pub fn on_driverui_documentupdated_processed(req: &DocumentUpdatedRequest) {
        Self::info(
            &"DriverUI:documentupdated".to_string(),
            &req.document_id.to_string(),
            "processed",
        );
    }

    /// Prints a log after processing a design token update message.
    pub fn on_driverui_designtokenupdate_processed(req: &DesignTokenUpdateRequest) {
        Self::info(
            &"DriverUI:designtokenupdate".to_string(),
            &format!("theme={}, variable_mode={}", req.theme, req.variable_mode),
            "processed",
        );
    }

    /// Prints a log after processing a locale update message.
    pub fn on_driverui_localeupdate_processed(req: &LocaleUpdateRequest) {
        Self::info(
            &"DriverUI:localeupdate".to_string(),
            &req.language_tag.to_string(),
            "processed",
        );
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
