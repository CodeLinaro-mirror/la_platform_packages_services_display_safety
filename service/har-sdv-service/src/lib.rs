// Copyright 2023 Google LLC

//! HAR-SDV-service connects to SDV Comms, Data tunnel and delivers events to HAR.

use crate::integrations::register_camera_proxy;
use crate::integrations::register_har_sdv_driverui_proxy;
use crate::sdv_service_utils::wait_for_sdv_services_ready;

use log::info;
use std::time::Duration;

use crate::integrations::create_topic_map;
use crate::mapper::SdvToHarMapper;
use grpcio::EnvBuilder;
use std::sync::Arc;

use crate::camera_grpc_proxy::CameraServiceGrpcProxy;
use crate::common::GrpcProxyServerToken;
use crate::common::CAMERA_RPC_CLIENT_ADDRESS;
use crate::common::CAMERA_RPC_SERVER_PORT;
use crate::common::DRIVERUI_RPC_CLIENT_ADDRESS;
use crate::common::DRIVERUI_RPC_SERVER_HOST;
use crate::common::DRIVERUI_RPC_SERVER_PORT;
use crate::driverui_grpc_proxy::DriverUiGrpcProxy;
use crate::integrations::proxy_user_preferences_to_harry;
use crate::integrations::proxy_vehicledata_to_harry;
use crate::integrations::proxy_vehicledata_to_qnx;
use crate::mapper::HashMapTopicMapper;
use crate::preferences::create_har_user_preferences_client;
use grpcio::Environment;
use rustutils::system_properties;
use sdvgenerated_har_preferences_client::har_preferences_client::HarPreferencesClient;
use std::sync::Mutex;
use std::thread::JoinHandle;

mod camera_grpc_proxy;
mod common;
mod driverui_grpc_proxy;
mod integrations;
mod mapper;
mod preferences;
mod sdv_service_utils;

/// Service bundle struct definition.
pub struct HarSdvServiceBundle {
    _context: ContextRef,
    grpc_env: Arc<Environment>,
    qnx_ip: Option<String>,
    sdv_to_har_mapper: Arc<SdvToHarMapper<HashMapTopicMapper>>,
    driverui_rpc_proxy: DriverUiGrpcProxy,
    camera_rpc_proxy: CameraServiceGrpcProxy,
    running_services: Mutex<Option<RunningServices>>,
}

// No direct usage of the members yet, but we use them during drop.
#[allow(dead_code)]
struct RunningServices {
    driverui_service: GrpcProxyServerToken,
    camera_service: GrpcProxyServerToken,
    qnx_handle: Option<JoinHandle<()>>,
    user_prefs_client: HarPreferencesClient,
}

const PRODUCT_HAR_SAFETY_MONITOR_IP: &str = "product.harplatform.safety_monitor";

// Register the new service bundle.
sdv_lifecycle_client::register_service_bundle!(HarSdvServiceBundle);

impl ServiceBundle for HarSdvServiceBundle {
    /// Creates a new instance of the HarSdvServiceBundle.
    /// Called when service bundle is created by the system.
    ///
    /// Context object is provided as a parameter that gives access to the
    /// communication stack APIs.
    fn new(_context: ContextRef) -> HarSdvServiceBundle {
        info!("SDV Service registered: {:?}", register_har_sdv_driverui_proxy());
        info!("SDV camera proxy service registered: {:?}", register_camera_proxy());

        let env = Arc::new(EnvBuilder::new().build());
        let mapper = Arc::new(SdvToHarMapper::new(create_topic_map()));

        // Run the proxy between HAR and DriverUI
        let driverui_rpc_proxy = DriverUiGrpcProxy::new(
            format!("{}:{}", DRIVERUI_RPC_SERVER_HOST, DRIVERUI_RPC_SERVER_PORT),
            DRIVERUI_RPC_CLIENT_ADDRESS.to_string(),
        );

        // Run another proxy between HAR and IVI Camera Service.
        let camera_rpc_proxy = CameraServiceGrpcProxy::new(
            format!("{}:{}", DRIVERUI_RPC_SERVER_HOST, CAMERA_RPC_SERVER_PORT),
            CAMERA_RPC_CLIENT_ADDRESS.to_string(),
        );

        let qnx_ip = match system_properties::read(PRODUCT_HAR_SAFETY_MONITOR_IP) {
            Ok(Some(ip)) => Some(ip.to_string()),
            Ok(None) => {
                info!(
                    "QNX IP is not set by '{:?}'. QNX Proxy not started.",
                    PRODUCT_HAR_SAFETY_MONITOR_IP
                );
                // QNX Proxy won't be started.
                None
            }
            Err(e) => panic!("Could not fetch Safety Monitor IP property. Err: {:?}", e),
        };

        HarSdvServiceBundle {
            _context,
            qnx_ip,
            sdv_to_har_mapper: mapper,
            grpc_env: env,
            driverui_rpc_proxy,
            camera_rpc_proxy,
            running_services: Mutex::new(None),
        }
    }

    /// Called when the service bundle is started by the system.
    fn on_start(&mut self) {
        let _ = sdv_log::init_logger("har_sdv_service_sb");
        // Make sure dependent services are running.
        wait_for_sdv_services_ready(Duration::from_secs(30)).expect("SDV services failed to start");

        let driverui_service = self.driverui_rpc_proxy.run(self.grpc_env.clone());
        info!("Cluster app GRPC dispatcher running.");

        let camera_service = self.camera_rpc_proxy.run(self.grpc_env.clone());
        info!("Camera service GRPC dispatcher running.");

        // Start SDV Data tunnel services to QNX.
        let qnx_handle = if let Some(qnx_ip) = self.qnx_ip.as_ref() {
            Some(proxy_vehicledata_to_qnx(
                qnx_ip,
                self.grpc_env.clone(),
                self.sdv_to_har_mapper.clone(),
            ))
        } else {
            None
        };

        // Start SDV Data tunnel services to HAR.
        let _handle_sdv =
            proxy_vehicledata_to_harry(self.grpc_env.clone(), self.sdv_to_har_mapper.clone());

        // Start SDV Data tunnel services for User Prefs to HAR.
        let user_prefs_client =
            proxy_user_preferences_to_harry(self.grpc_env.clone(), self.sdv_to_har_mapper.clone());

        let mut running_services =
            self.running_services.lock().expect("Cannot lock running services.");
        running_services.replace(RunningServices {
            driverui_service,
            camera_service,
            qnx_handle,
            user_prefs_client,
        });
    }

    /// Called when the service bundle is stopped by the system in preparation
    /// for shutdown or suspend to RAM/Disc.
    fn on_stop(&mut self) {
        info!("Service bundle stopped.");
        let mut running_services =
            self.running_services.lock().expect("Cannot lock running services.");
        drop(running_services.take());
        // TODO: shut down user preferences and other threads properly.
    }
}
