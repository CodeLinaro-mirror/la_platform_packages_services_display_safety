// Copyright 2024 Google LLC

//! Vehicle data publisher service bundle (reference implementation).
//!
//! This is an SDV Data Tunnel based vehicle data publisher in a service bundle,
//! exposing a GRPC server to control the data sent to the data tunnel.
//!
//! This is a reference implementation: a vehicle would probably receive
//! vehicle data from the vehicle hardware.

use crate::grpc_server::VehicleDataGrpcServer;
use async_trait::async_trait;
use grpcio::ChannelBuilder;
use grpcio::EnvBuilder;
use grpcio::ResourceQuota;
use grpcio::Server;
use grpcio::ServerBuilder;
use grpcio::ServerCredentials;
use har_sdv_service_bundle_common::async_service_bundle::AsyncServiceBundle;
use har_sdv_service_bundle_common::async_service_bundle::AsyncServiceBundleLauncher;
use harry_vehicle_data_grpc::vehicle_tire::Location;
use harry_vehicle_data_grpc::vehicledata::Telltale;
use harry_vehicle_data_grpc::vehicledata::VehicleSpeedTopic;
use harry_vehicle_data_grpc::vehicledata_grpc_service_grpc::*;
use log::info;
use oem_harry_vehicle_messages_catalog_v1::vehicle_tire::TirePressure;
use oem_harry_vehicle_messages_catalog_v1::vehicledata::CurrentGear;
use oem_harry_vehicle_messages_catalog_v1::vehicledata::TellTaleStatus;
use oem_harry_vehicle_messages_catalog_v1::vehicledata::VehicleSpeed;
use sdv::mw::Communicate;
use sdv::mw::Publisher;
use sdv::status::SdvStatus;
use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_vehicle_data_publisher::publisher::Variant;
use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_vehicle_data_publisher::HarSdvVehicleDataPublisher as PublisherService;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

mod grpc_server;
mod model;

const GRPC_ADDRESS: &str = "0.0.0.0:7002";

/// Service bundle struct definition.
pub struct HarSdvVehicleDataPublisher {
    comms: Arc<dyn Communicate>,
}

/// Contains the publishers
#[derive(Clone)]
pub struct HarryVehicleDataPublishers {
    // telltales
    oil_pressure_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    engine_temp_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    check_engine_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    charging_failure_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    seatbelt_driver_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    seatbelt_passenger_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    low_tire_pressure_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    airbag_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    abs_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    brake_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    traction_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    fog_lights_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    park_lights_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    hibeam_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    lowbeam_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    turn_signal_left_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    turn_signal_right_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    adas_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    max_speed_displayed_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    speed_limit_displayed_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    emergency_light_publisher: Arc<Mutex<Publisher<TellTaleStatus>>>,
    // vehicle speed
    vehicle_speed_publisher: Arc<Mutex<Publisher<VehicleSpeed>>>,
    max_speed_publisher: Arc<Mutex<Publisher<VehicleSpeed>>>,
    speedlimit_publisher: Arc<Mutex<Publisher<VehicleSpeed>>>,
    // tire pressure
    front_left_publisher: Arc<Mutex<Publisher<TirePressure>>>,
    front_right_publisher: Arc<Mutex<Publisher<TirePressure>>>,
    rear_left_publisher: Arc<Mutex<Publisher<TirePressure>>>,
    rear_right_publisher: Arc<Mutex<Publisher<TirePressure>>>,
    fifth_wheel_publisher: Arc<Mutex<Publisher<TirePressure>>>,
    // current gear
    gear_publisher: Arc<Mutex<Publisher<CurrentGear>>>,
}

impl HarryVehicleDataPublishers {
    /// Publish a vehicle speed update.
    pub async fn vehicle_speed_publish(&self, speed_type: VehicleSpeedTopic, speed: VehicleSpeed) {
        let publisher = match speed_type {
            VehicleSpeedTopic::VEHICLE_SPEED => &self.vehicle_speed_publisher,
            VehicleSpeedTopic::MAX_SPEED => &self.max_speed_publisher,
            VehicleSpeedTopic::SPEEDLIMIT => &self.speedlimit_publisher,
        };
        Self::publish_locked(publisher.clone(), speed).await;
    }

    /// Publish a vehicle telltale update
    pub async fn tell_tale_status_publish(&self, telltale: Telltale, status: TellTaleStatus) {
        let publisher = match telltale {
            Telltale::OIL_PRESSURE => &self.oil_pressure_publisher,
            Telltale::ENGINE_TEMP => &self.engine_temp_publisher,
            Telltale::CHECK_ENGINE => &self.check_engine_publisher,
            Telltale::CHARGING_FAILURE => &self.charging_failure_publisher,
            Telltale::SEATBELT_DRIVER => &self.seatbelt_driver_publisher,
            Telltale::SEATBELT_PASSENGER => &self.seatbelt_passenger_publisher,
            Telltale::LOW_TIRE_PRESSURE => &self.low_tire_pressure_publisher,
            Telltale::AIRBAG => &self.airbag_publisher,
            Telltale::ABS => &self.abs_publisher,
            Telltale::BRAKE => &self.brake_publisher,
            Telltale::TRACTION => &self.traction_publisher,
            Telltale::FOG_LIGHTS => &self.fog_lights_publisher,
            Telltale::PARK_LIGHTS => &self.park_lights_publisher,
            Telltale::HIBEAM => &self.hibeam_publisher,
            Telltale::LOWBEAM => &self.lowbeam_publisher,
            Telltale::TURN_SIGNAL_LEFT => &self.turn_signal_left_publisher,
            Telltale::TURN_SIGNAL_RIGHT => &self.turn_signal_right_publisher,
            Telltale::ADAS => &self.adas_publisher,
            Telltale::MAX_SPEED_DISPLAYED => &self.max_speed_displayed_publisher,
            Telltale::SPEED_LIMIT_DISPLAYED => &self.speed_limit_displayed_publisher,
            Telltale::EMERGENCY_LIGHT => &self.emergency_light_publisher,
        };
        Self::publish_locked(publisher.clone(), status).await;
    }

    /// Publish a gear update.
    pub async fn current_gear_publish(&self, gear: CurrentGear) {
        Self::publish_locked(self.gear_publisher.clone(), gear).await;
    }

    /// Publish a tire pressure update
    pub async fn tire_pressure_publish(&self, location: Location, pressure: TirePressure) {
        let publisher = match location {
            Location::FRONT_LEFT => &self.front_left_publisher,
            Location::FRONT_RIGHT => &self.front_right_publisher,
            Location::REAR_LEFT => &self.rear_left_publisher,
            Location::REAR_RIGHT => &self.rear_right_publisher,
            Location::FIFTH_WHEEL => &self.fifth_wheel_publisher,
        };
        Self::publish_locked(publisher.clone(), pressure).await;
    }

    async fn publish_locked<T: protobuf::Message>(publisher: Arc<Mutex<Publisher<T>>>, message: T) {
        let publisher = publisher.lock().await;
        publisher.publish(&message).expect("Cannot publish message");
    }
}

// Register the new service bundle.
sdv_lifecycle_client::register_service_bundle!(AsyncServiceBundle<HarSdvVehicleDataPublisher>);

#[async_trait]
impl AsyncServiceBundleLauncher for HarSdvVehicleDataPublisher {
    fn new(comms: Arc<dyn Communicate>) -> Self {
        HarSdvVehicleDataPublisher { comms }
    }

    async fn launch(self, cancellation_token: CancellationToken) -> Result<(), SdvStatus> {
        info!("HAR-SDV Vehicle data publisher starting.");
        let mut publisher_service = PublisherService::new(self.comms.clone())
            .await
            .expect("Cannot create telltale manager.");

        let oil_pressure_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::OIL_PRESSURE)
                .expect("OIL_PRESSURE publisher is already taken"),
        ));
        let engine_temp_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::ENGINE_TEMP)
                .expect("ENGINE_TEMP publisher is already taken"),
        ));
        let check_engine_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::CHECK_ENGINE)
                .expect("CHECK_ENGINE publisher is already taken"),
        ));
        let charging_failure_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::CHARGING_FAILURE)
                .expect("CHARGING_FAILURE publisher is already taken"),
        ));
        let seatbelt_driver_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::SEATBELT_DRIVER)
                .expect("SEATBELT_DRIVER publisher is already taken"),
        ));
        let seatbelt_passenger_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::SEATBELT_PASSENGER)
                .expect("SEATBELT_PASSENGER publisher is already taken"),
        ));
        let low_tire_pressure_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::LOW_TIRE_PRESSURE)
                .expect("LOW_TIRE_PRESSURE publisher is already taken"),
        ));
        let airbag_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::AIRBAG)
                .expect("AIRBAG publisher is already taken"),
        ));
        let abs_publisher = Arc::new(Mutex::new(
            publisher_service.take_publisher(Variant::ABS).expect("ABS publisher is already taken"),
        ));
        let brake_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::BRAKE)
                .expect("BRAKE publisher is already taken"),
        ));
        let traction_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::TRACTION)
                .expect("TRACTION publisher is already taken"),
        ));
        let fog_lights_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::FOG_LIGHTS)
                .expect("FOG_LIGHTS publisher is already taken"),
        ));
        let park_lights_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::PARK_LIGHTS)
                .expect("PARK_LIGHTS publisher is already taken"),
        ));
        let hibeam_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::HIBEAM)
                .expect("HIBEAM publisher is already taken"),
        ));
        let lowbeam_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::LOWBEAM)
                .expect("LOWBEAM publisher is already taken"),
        ));
        let turn_signal_left_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::TURN_SIGNAL_LEFT)
                .expect("TURN_SIGNAL_LEFT publisher is already taken"),
        ));
        let turn_signal_right_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::TURN_SIGNAL_RIGHT)
                .expect("TURN_SIGNAL_RIGHT publisher is already taken"),
        ));
        let adas_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::ADAS)
                .expect("ADAS publisher is already taken"),
        ));
        let max_speed_displayed_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::MAX_SPEED_DISPLAYED)
                .expect("MAX_SPEED_DISPLAYED publisher is already taken"),
        ));
        let speed_limit_displayed_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::SPEED_LIMIT_DISPLAYED)
                .expect("SPEED_LIMIT_DISPLAYED publisher is already taken"),
        ));
        let emergency_light_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::EMERGENCY_LIGHT)
                .expect("EMERGENCY_LIGHT publisher is already taken"),
        ));

        let vehicle_speed_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::VEHICLE_SPEED)
                .expect("VEHICLE_SPEED publisher is already taken"),
        ));
        let max_speed_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::MAX_SPEED)
                .expect("MAX_SPEED publisher is already taken"),
        ));
        let speedlimit_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::SPEEDLIMIT)
                .expect("SPEEDLIMIT publisher is already taken"),
        ));

        let front_left_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::FRONT_LEFT)
                .expect("FRONT_LEFT publisher is already taken"),
        ));
        let front_right_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::FRONT_RIGHT)
                .expect("FRONT_RIGHT publisher is already taken"),
        ));
        let rear_left_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::REAR_LEFT)
                .expect("REAR_LEFT publisher is already taken"),
        ));
        let rear_right_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::REAR_RIGHT)
                .expect("REAR_RIGHT publisher is already taken"),
        ));
        let fifth_wheel_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::FIFTH_WHEEL)
                .expect("FIFTH_WHEEL publisher is already taken"),
        ));

        let gear_publisher = Arc::new(Mutex::new(
            publisher_service
                .take_publisher(Variant::UNIQUE)
                .expect("GEAR publisher is already taken"),
        ));

        let publishers = HarryVehicleDataPublishers {
            // telltales
            oil_pressure_publisher,
            engine_temp_publisher,
            check_engine_publisher,
            charging_failure_publisher,
            seatbelt_driver_publisher,
            seatbelt_passenger_publisher,
            low_tire_pressure_publisher,
            airbag_publisher,
            abs_publisher,
            brake_publisher,
            traction_publisher,
            fog_lights_publisher,
            park_lights_publisher,
            hibeam_publisher,
            lowbeam_publisher,
            turn_signal_left_publisher,
            turn_signal_right_publisher,
            adas_publisher,
            max_speed_displayed_publisher,
            speed_limit_displayed_publisher,
            emergency_light_publisher,
            // vehicle speed
            vehicle_speed_publisher,
            max_speed_publisher,
            speedlimit_publisher,
            // tire pressure
            front_left_publisher,
            front_right_publisher,
            rear_left_publisher,
            rear_right_publisher,
            fifth_wheel_publisher,
            // current gear
            gear_publisher,
        };
        info!("HAR-SDV Vehicle data publisher GRPC server is starting.");
        let mut grpc_server = create_grpc_server(GRPC_ADDRESS.to_string(), publishers);
        let _grpc_server = grpc_server.start();
        info!("GRPC Server started.");
        // waiting for cancellation
        cancellation_token.cancelled().await;
        grpc_server.shutdown();
        info!("HAR-SDV Vehicle data publisher completed.");
        Ok(())
    }
}

/// Creates the GRPC server for receiving vehicle data send request.
/// This simulates receiving data from the vehicle hardware.
/// * `server_address`: The server address.
/// * `sdv_service`: To SDV service to use.
/// * returns the server
fn create_grpc_server(server_address: String, sdv_service: HarryVehicleDataPublishers) -> Server {
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
