// Copyright 2024 Google LLC

//! Vehicle data publisher service (reference implementation).
//!
//! This is an SDV Data Tunnel based vehicle data publisher,
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
use log::warn;
use sdvgenerated::harry_vehicle_data_publisher::HarryVehicleDataPublisher;
use sdvgenerated::harry_vehicle_data_publisher::HarryVehicleDataPublisherCallbacks;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

mod grpc_server;
mod model;

const GRPC_ADDRESS: &str = "0.0.0.0:7002";

fn main() -> Result<(), ()> {
    // Create SDV DT topics
    let mut service = HarryVehicleDataPublisher::new(HarryVehicleDataPublisherCallbacks {});
    if let Err(err) = service.start() {
        warn!("Service starting failed: {:?}", err);
        return Err(());
    }
    info!("SDV Data tunnel connected.");

    // Start GRPC server
    let _grpc_server = run_grpc_server(GRPC_ADDRESS.to_string(), service);
    info!("RPC server running.");

    // Loop forever, we never want to intentionally exit this process.
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

/// Starts the GRPC server.
/// * `server_address`: The server address.
/// * `sdv_service`: To SDV service to use.
/// * returns the server token that can be used to stop the server.
fn run_grpc_server(server_address: String, sdv_service: HarryVehicleDataPublisher) -> Server {
    let env = Arc::new(EnvBuilder::new().build());

    // Create server
    let quota = ResourceQuota::new(Some("VehicleDataPublisherService")).resize_memory(1024 * 1024);
    let server_ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

    let service =
        create_sdv_vehicle_data_grpc(VehicleDataGrpcServer::new(Arc::new(Mutex::new(sdv_service))));
    let mut server = ServerBuilder::new(env.clone())
        .register_service(service)
        .channel_args(server_ch_builder.build_args())
        .build()
        .unwrap();
    server.add_listening_port(server_address.as_str(), ServerCredentials::insecure()).unwrap();
    server.start();
    server
}
