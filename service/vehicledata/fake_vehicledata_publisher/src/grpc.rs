// Copyright 2023 Google LLC

use core::time::Duration;
use grpcio::ChannelBuilder;
use grpcio::EnvBuilder;
use harry_vehicle_data_grpc::vehicledata_grpc_service_grpc::SdvVehicleDataGrpcClient;
use std::sync::Arc;

/// Creates the control client for vehicle data events.
/// - `client_address`: The address of the GRPC server.
pub fn create_grpc_client(client_address: String) -> SdvVehicleDataGrpcClient {
    let client_ch = ChannelBuilder::new(Arc::new(EnvBuilder::new().build()))
        .initial_reconnect_backoff(Duration::from_millis(10))
        .max_reconnect_backoff(Duration::from_millis(50))
        .connect(client_address.as_str());
    SdvVehicleDataGrpcClient::new(client_ch)
}
