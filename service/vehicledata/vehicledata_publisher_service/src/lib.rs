// Copyright 2024 Google LLC

//! Vehicle data publisher service bundle (reference implementation).
//!
//! This is an SDV Data Tunnel based vehicle data publisher in a service bundle,
//! exposing a GRPC server to control the data sent to the data tunnel.
//!
//! This is a reference implementation: a vehicle would probably receive
//! vehicle data from the vehicle hardware.

use crate::grpc_server::VehicleDataGrpcServer;
use crate::model::Error;

use grpcio::ChannelBuilder;
use grpcio::EnvBuilder;
use grpcio::ResourceQuota;
use grpcio::Server;
use grpcio::ServerBuilder;
use grpcio::ServerCredentials;
use harry_vehicle_data_grpc::vehicledata_grpc_service_grpc::*;
use log::info;
use sdvgenerated::harry_vehicle_data_publisher::HarryVehicleDataPublisher;
use sdvgenerated::harry_vehicle_data_publisher::HarryVehicleDataPublisherCallbacks;
use std::sync::Arc;
use std::sync::Mutex;

mod grpc_server;
mod model;

const GRPC_ADDRESS: &str = "0.0.0.0:7002";

/// Service bundle struct definition.
pub struct HarSdvVehicleDataPublisherServiceBundle {
    _context: ContextRef,
    sdv_service: Arc<Mutex<HarryVehicleDataPublisher>>,
    grpc_server: Arc<Mutex<Server>>,
}

// Register the new service bundle.
sdv_lifecycle_client::register_service_bundle!(HarSdvVehicleDataPublisherServiceBundle);

impl ServiceBundle for HarSdvVehicleDataPublisherServiceBundle {
    /// Creates a new instance of the HarSdvVehicleDataPublisherServiceBundle.
    /// Called when service bundle is created by the system.
    ///
    /// Context object is provided as a parameter that gives access to the
    /// communication stack APIs.
    fn new(_context: ContextRef) -> HarSdvVehicleDataPublisherServiceBundle {
        let _ = sdv_log::init_logger("sdv_vehicledata_publisher_sb");
        info!("Creating service bundle.");
        // Initialize the SDV service.
        let sdv_service = Arc::new(Mutex::new(HarryVehicleDataPublisher::new(
            HarryVehicleDataPublisherCallbacks {},
        )));
        // Initialize the GRPC server
        let grpc_server =
            Arc::new(Mutex::new(create_grpc_server(GRPC_ADDRESS.to_string(), sdv_service.clone())));

        HarSdvVehicleDataPublisherServiceBundle { _context, sdv_service, grpc_server }
    }

    /// Called when the service bundle is started by the system.
    fn on_start(&mut self) {
        let mut sdv_service =
            self.sdv_service.lock().expect("Cannot lock the SDV service object.  ");
        sdv_service.start().unwrap_or_else(|err| panic!("Service starting failed: {:?}", err));
        info!("SDV Data tunnel started.");
        drop(sdv_service);

        let mut grpc_server = self.grpc_server.lock().expect("Cannot lock the GRPC server object.");
        grpc_server.start();
        info!("GRPC Server started.");
    }

    /// Called when the service bundle is stopped by the system in preparation
    /// for shutdown or suspend to RAM/Disc.
    fn on_stop(&mut self) {
        // Stop phase requires the service bundle to delete the dynamic resources
        // (sockets, files, etc) that were previously allocated in the [Self::on_start()] method.
    }
}

/// Called when the service bundle is destroyed by the system.
impl Drop for HarSdvVehicleDataPublisherServiceBundle {
    fn drop(&mut self) {
        // Static resources deallocation needs to be implemented in the drop method.
    }
}

/// Creates the GRPC server for receiving vehicle data send request.
/// This simulates receiving data from the vehicle hardware.
/// * `server_address`: The server address.
/// * `sdv_service`: To SDV service to use.
/// * returns the server
fn create_grpc_server(
    server_address: String,
    sdv_service: Arc<Mutex<HarryVehicleDataPublisher>>,
) -> Server {
    let env = Arc::new(EnvBuilder::new().build());

    // Create server
    let quota = ResourceQuota::new(Some("VehicleDataPublisherService")).resize_memory(1024 * 1024);
    let server_ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

    let service = create_sdv_vehicle_data_grpc(VehicleDataGrpcServer::new(sdv_service));
    let mut server = ServerBuilder::new(env.clone())
        .register_service(service)
        .channel_args(server_ch_builder.build_args())
        .build()
        .unwrap();
    server.add_listening_port(server_address.as_str(), ServerCredentials::insecure()).unwrap();
    server
}
