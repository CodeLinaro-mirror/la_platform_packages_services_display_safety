// Copyright 2023 Google LLC

//! HAR-SDV-service connects to SDV Comms, Data tunnel and delivers events to HAR.

use crate::camera_grpc_proxy::CameraServiceGrpcProxy;
use crate::driverui_grpc_proxy::DriverUiGrpcProxy;
use crate::integrations::proxy_user_preferences_to_harry;
use crate::integrations::proxy_vehicledata_to_harry;
use crate::integrations::proxy_vehicledata_to_qnx;
use crate::integrations::register_camera_proxy;
use crate::integrations::register_har_sdv_driverui_proxy;
use crate::mapper::SdvToHarMapper;
use crate::preferences::create_har_user_preferences_client;
use core::time::Duration;
use grpcio::EnvBuilder;
use integrations::create_topic_map;
use log::info;
use rustutils::system_properties;
use sdv_service_utils_stub::wait_for_sdv_services_ready;
use std::panic;
use std::sync::Arc;

use crate::common::CAMERA_RPC_CLIENT_ADDRESS;
use crate::common::CAMERA_RPC_SERVER_PORT;
use crate::common::DRIVERUI_RPC_CLIENT_ADDRESS;
use crate::common::DRIVERUI_RPC_SERVER_HOST;
use crate::common::DRIVERUI_RPC_SERVER_PORT;

mod camera_grpc_proxy;
mod common;
mod driverui_grpc_proxy;
mod integrations;
mod mapper;
mod preferences;
mod sdv_service_utils_stub;

const PRODUCT_HAR_SAFETY_MONITOR_IP: &str = "vendor.harplatform.safety_monitor";

// Deprecated: This service moved to SDV service bundles.
// This implementation is no longer in-use on APEX-based systems.
fn main() -> Result<(), ()> {
    // Make sure dependent services are running.
    wait_for_sdv_services_ready(Duration::from_secs(30)).expect("SDV services failed to start");

    sdv_log::init_logger("har_sdv_service").unwrap();

    info!("HAR SDV service starting");

    info!("SDV Service registered: {:?}", register_har_sdv_driverui_proxy());

    let env = Arc::new(EnvBuilder::new().build());
    let mapper = Arc::new(SdvToHarMapper::new(create_topic_map()));

    // Run the proxy between HAR and DriverUI
    let driverui_rpc_proxy = DriverUiGrpcProxy::new(
        format!("{}:{}", DRIVERUI_RPC_SERVER_HOST, DRIVERUI_RPC_SERVER_PORT),
        DRIVERUI_RPC_CLIENT_ADDRESS.to_string(),
    );
    let mut proxy_server = driverui_rpc_proxy.run(env.clone());
    info!("Cluster app GRPC dispatcher running.");

    info!("SDV camera proxy service registered: {:?}", register_camera_proxy());

    // Run another proxy between HAR and IVI Camera Service.
    let camera_rpc_proxy = CameraServiceGrpcProxy::new(
        format!("{}:{}", DRIVERUI_RPC_SERVER_HOST, CAMERA_RPC_SERVER_PORT),
        CAMERA_RPC_CLIENT_ADDRESS.to_string(),
    );
    let mut camera_proxy_server = camera_rpc_proxy.run(env.clone());
    info!("Camera service GRPC dispatcher running.");

    // The QNX IP address has to be hardcoded.  Set as a system property.
    let handle_qnx = match system_properties::read(PRODUCT_HAR_SAFETY_MONITOR_IP) {
        Ok(value) => {
            value.as_ref().map(|ip| proxy_vehicledata_to_qnx(ip, env.clone(), mapper.clone()))
        }

        Err(e) => {
            panic!("Could not fetch Safety Monitor IP property. Err: {:?}", e);
        }
    };

    info!("Starting HAR data service");

    // Start SDV Data tunnel services to HAR.
    let handle_sdv = proxy_vehicledata_to_harry(env.clone(), mapper.clone());

    let mut handles = vec![handle_sdv];
    if let Some(handle_qnx) = handle_qnx {
        handles.push(handle_qnx);
    }

    // Start SDV Data tunnel services for User Prefs to HAR.
    let _prefs_client = proxy_user_preferences_to_harry(env, mapper);

    // Join&unwrap all to see any issues. Note: the order matters here, so we might be
    // waiting on something that works while other have already failed.
    // TODO: Move back to async futures and join_all.
    for handle in handles {
        handle.join().expect("One of the threads failed.");
    }

    proxy_server.shutdown();
    camera_proxy_server.shutdown();
    info!("HAR SDV service completed");
    Ok(())
}
