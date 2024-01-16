// Copyright 2023 Google LLC

//! This SDV service proxies between SDV and HAR.

use oem_harry_vehicle_messages_catalog::vehicle_tire::TirePressure;
use oem_harry_vehicle_messages_catalog::vehicledata::CurrentGear;
use oem_harry_vehicle_messages_catalog::vehicledata::Gear;
use oem_harry_vehicle_messages_catalog::vehicledata::TellTaleStatus;
use oem_harry_vehicle_messages_catalog::vehicledata::VehicleSpeed;
use sdv_middleware_pubsub::EnumTopic;
use sdvgenerated::harry_vehicle_data_subscriber::HarryVehicleDataSubscriber;
use sdvgenerated::harry_vehicle_data_subscriber::HarryVehicleDataSubscriberCallbacks;
use sdvgenerated::topics::subscriber;

use grpcio::ClientDuplexSender;
use grpcio::{ChannelBuilder, EnvBuilder};
use har_grpc_services::vehicledata::VehicleDataFragment;
use har_grpc_services::vehicledata_grpc::VehicleDataServiceClient;
use std::sync::Arc;
use std::sync::Mutex;

use crate::mapper::HashMapTopicMapper;
use crate::mapper::SdvToHarMapper;
use crate::mapper::TopicMapper;
use core::time::Duration;
use futures_util::SinkExt;
use grpcio::WriteFlags;
use log::info;
use log::warn;

mod mapper;

#[tokio::main]
async fn main() -> Result<(), ()> {
    // Allow time for Harry to start GRPC.
    // TODO: Use the property trigger once it is fixed (sepolicy missing).
    info!("HAR SDV proxy service starting");
    tokio::time::sleep(Duration::from_secs(1)).await;

    sdv_log::init_logger("har_sdv_service").unwrap_or_else(|error| match &error {
        sdv_log::LoggerError::AlreadyInitializedError(_) => {
            // Only inform error, not panic
            log::info!("{}", error)
        }
        _ => panic!("{}", error),
    });

    let env = Arc::new(EnvBuilder::new().build());

    let mapper = Arc::new(SdvToHarMapper::new(create_topic_map()));

    // TODO: handle errors, use a different transport
    let ch = ChannelBuilder::new(env)
        .initial_reconnect_backoff(Duration::from_millis(10))
        .max_reconnect_backoff(Duration::from_millis(50))
        .connect("127.0.0.1:50051");
    let vehicle_data_client = VehicleDataServiceClient::new(ch);
    // TODO: Do this in some sort of a loop to make sure it never stops
    match vehicle_data_client.receive_vehicle_data() {
        Ok((vehicle_data_sender, _vehicle_data_receiver)) => {
            // Connection is established
            let vehicle_data_sender = Arc::new(Mutex::new(vehicle_data_sender));
            let mut sdv_service =
                create_sdv_data_service(vehicle_data_sender.clone(), mapper.clone());
            let _ = sdv_service.start();
            sdv_service.join();
            // TODO: receive response from server (vehicle_data_receiver)
            // TODO: join or close channels.
        }
        Err(err) => {
            // TODO: No panic!
            panic!("Failed to call Vehicle data api. Err: {:?}", err);
        }
    }
    info!("HAR SDV proxy completed");
    Ok(())
}

fn send_data_blocking<T>(sender: Arc<Mutex<ClientDuplexSender<T>>>, data: T) {
    info!("Sending");
    if let Ok(mut sender) = sender.lock() {
        let result = futures::executor::block_on(sender.send((data, WriteFlags::default())));
        info!("Sent with result: {:?}", result);
    }
}

fn create_sdv_data_service(
    vehicle_data_sender: Arc<Mutex<ClientDuplexSender<VehicleDataFragment>>>,
    data_mapper: Arc<SdvToHarMapper<impl TopicMapper + Sync + Send + 'static>>,
) -> HarryVehicleDataSubscriber {
    // Vehicle speed callback
    let sender = vehicle_data_sender.clone();
    let mapper = data_mapper.clone();
    let vehicle_speed_cb = Box::new(
        move |topic: &subscriber::VehicleSpeedTopics,
              message: sdv_middleware_pubsub::Result<VehicleSpeed>| {
            if let Ok(message) = message {
                send_data_blocking(
                    sender.clone(),
                    mapper.map_i32(topic.get_name().to_string(), message.speed as _),
                );
            } else {
                warn!("ERROR on {}: {:?}!", topic.get_name(), message);
            }
        },
    );

    // Current gear callback
    let sender = vehicle_data_sender.clone();
    let mapper = data_mapper.clone();
    let current_gear_cb = Box::new(
        move |topic: &subscriber::CurrentGearTopics,
              message: sdv_middleware_pubsub::Result<CurrentGear>| {
            if let Ok(message) = message {
                // TODO: Using u8 for gear. Harry expects a string, but no string callbacks implemented yet.
                let gear: String = match message.gear.unwrap() {
                    Gear::P => 'P'.into(),
                    Gear::R => 'R'.into(),
                    Gear::N => 'N'.into(),
                    Gear::D => 'D'.into(),
                };
                send_data_blocking(
                    sender.clone(),
                    mapper.map_string(topic.get_name().to_string(), gear),
                );
            } else {
                warn!("ERROR on {}: {:?}!", topic.get_name(), message);
            }
        },
    );

    // Tire pressure callback
    let sender = vehicle_data_sender.clone();
    let mapper = data_mapper.clone();
    let tire_pressure_cb = Box::new(
        move |topic: &subscriber::TirePressureTopics,
              message: sdv_middleware_pubsub::Result<TirePressure>| {
            if let Ok(message) = message {
                send_data_blocking(
                    sender.clone(),
                    mapper.map_u32(topic.get_name().to_string(), message.pressure),
                );
            } else {
                warn!("ERROR on {}: {:?}!", topic.get_name(), message);
            }
        },
    );

    // Telltales callback
    let sender = vehicle_data_sender.clone();
    let mapper = data_mapper.clone();
    let tell_tale_status_cb = Box::new(
        move |topic: &subscriber::TellTaleStatusTopics,
              message: sdv_middleware_pubsub::Result<TellTaleStatus>| {
            if let Ok(message) = message {
                send_data_blocking(
                    sender.clone(),
                    mapper.map_bool(topic.get_name().to_string(), message.alert),
                );
            } else {
                warn!("ERROR on {}: {:?}!", topic.get_name(), message);
            }
        },
    );

    let callbacks = HarryVehicleDataSubscriberCallbacks {
        vehicle_speed_cb,
        current_gear_cb,
        tire_pressure_cb,
        tell_tale_status_cb,
    };

    HarryVehicleDataSubscriber::new(callbacks)
}

/// Creates the mapper.
pub fn create_topic_map() -> HashMapTopicMapper {
    let mut map = HashMapTopicMapper::new();

    map.add("har.vehicledata.TirePressure.REAR_LEFT", "tire_pressure_rear_left");
    map.add("har.vehicledata.TirePressure.FRONT_RIGHT", "tire_pressure_front_right");
    map.add("har.vehicledata.TirePressure.REAR_RIGHT", "tire_pressure_rear_right");
    map.add("har.vehicledata.TirePressure.FRONT_LEFT", "tire_pressure_front_left");
    map.add("har.vehicledata.TirePressure.FIFTH_WHEEL", "tire_pressure_fifth_wheel");
    map.add("har.vehicledata.VehicleSpeed.VEHICLE_SPEED", "vehicle_speed");
    map.add("har.vehicledata.VehicleSpeed.SPEEDLIMIT", "speed_limit");
    map.add("har.vehicledata.VehicleSpeed.MAX_SPEED", "max_speed");
    map.add("har.vehicledata.CurrentGear.GEAR", "vehicle_gear");
    map.add("har.vehicledata.TellTaleStatus.PARK_LIGHTS", "park_lights");
    map.add("har.vehicledata.TellTaleStatus.ADAS", "adas");
    map.add("har.vehicledata.TellTaleStatus.FOG_LIGHTS", "fog_lights");
    map.add("har.vehicledata.TellTaleStatus.TRACTION", "traction");
    map.add("har.vehicledata.TellTaleStatus.SEATBELT_PASSENGER", "seatbelt_passenger");
    map.add("har.vehicledata.TellTaleStatus.CHECK_ENGINE", "check_engine");
    map.add("har.vehicledata.TellTaleStatus.OIL_PRESSURE", "oil_pressure");
    map.add("har.vehicledata.TellTaleStatus.BRAKE", "brake");
    map.add("har.vehicledata.TellTaleStatus.LOWBEAM", "lowbeam");
    map.add("har.vehicledata.TellTaleStatus.SEATBELT_DRIVER", "seatbelt_driver");
    map.add("har.vehicledata.TellTaleStatus.CHARGING_FAILURE", "charging_failure");
    map.add("har.vehicledata.TellTaleStatus.MAX_SPEED_DISPLAYED", "max_speed_displayed");
    map.add("har.vehicledata.TellTaleStatus.ENGINE_TEMP", "engine_temp");
    map.add("har.vehicledata.TellTaleStatus.AIRBAG", "airbag");
    map.add("har.vehicledata.TellTaleStatus.EMERGENCY_LIGHT", "emergency_light");
    map.add("har.vehicledata.TellTaleStatus.ABS", "abs");
    map.add("har.vehicledata.TellTaleStatus.HIBEAM", "hibeam");
    map.add("har.vehicledata.TellTaleStatus.TURN_SIGNAL_RIGHT", "turn_signal_right");
    map.add("har.vehicledata.TellTaleStatus.LOW_TIRE_PRESSURE", "low_tire_pressure");
    map.add("har.vehicledata.TellTaleStatus.TURN_SIGNAL_LEFT", "turn_signal_left");
    map.add("har.vehicledata.TellTaleStatus.SPEED_LIMIT_DISPLAYED", "speed_limit_displayed");

    map
}
