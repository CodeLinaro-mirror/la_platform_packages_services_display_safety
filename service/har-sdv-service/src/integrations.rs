use crate::mapper::HashMapTopicMapper;
use crate::mapper::SdvToHarMapper;
use crate::mapper::TopicMapper;
use grpcio::ClientDuplexSender;
use har_grpc_services::vehicledata::VehicleData;
use log::warn;
use std::sync::Arc;
use std::sync::Mutex;

use crate::common::CAMERA_RPC_SERVER_PORT;
use crate::common::DRIVERUI_RPC_SERVER_PORT;
use crate::create_har_user_preferences_client;
use futures_util::SinkExt;
use google_sdv_sd_common_aidl::aidl::google::sdv::sd_common::ServiceFqin::ServiceFqin;
use google_sdv_sd_common_aidl::aidl::google::sdv::sd_common::ServiceIdentity::PublicKey::PublicKey;
use grpcio::ChannelBuilder;
use grpcio::Environment;
use grpcio::WriteFlags;
use har_grpc_services::vehicledata_grpc::VehicleDataServiceClient;
use har_sdv_rpc::sdv_service_discovery::register_service;
use log::info;
use oem_harry_vehicle_messages_catalog::vehicle_tire::TirePressure;
use oem_harry_vehicle_messages_catalog::vehicledata::CurrentGear;
use oem_harry_vehicle_messages_catalog::vehicledata::Gear;
use oem_harry_vehicle_messages_catalog::vehicledata::TellTaleStatus;
use oem_harry_vehicle_messages_catalog::vehicledata::VehicleSpeed;
use sdv_middleware_pubsub::EnumTopic;
use sdvgenerated::harry_vehicle_data_subscriber::HarryVehicleDataSubscriber;
use sdvgenerated::harry_vehicle_data_subscriber::HarryVehicleDataSubscriberCallbacks;
use sdvgenerated::topics::subscriber;
use sdvgenerated_har_preferences_client::har_preferences_client::HarPreferencesClient;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

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

    // A set of SDV user preferences mapped to HAR events.
    map.add("TEMPERATURE_UNITS", "sdv.preferences.temperature_units");
    map.add("DISTANCE_UNITS", "sdv.preferences.distance_units");
    map
}

/// Subscribes to SDV vehicle data and sends all events to HAR.
pub fn create_sdv_data_service(
    vehicle_data_sender: Arc<Mutex<ClientDuplexSender<VehicleData>>>,
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

pub(crate) fn send_data_blocking<T>(sender: Arc<Mutex<ClientDuplexSender<T>>>, data: T) {
    if let Ok(mut sender) = sender.lock() {
        let result = futures::executor::block_on(sender.send((data, WriteFlags::default())));
        if result.is_err() {
            warn!("Error dispatching data: {:?}", result);
        }
    }
}

/// Registers the DriverUI GRPC proxy server.
pub(crate) fn register_har_sdv_driverui_proxy() -> Result<(), String> {
    let publickey = PublicKey { value: *b"HARSDVGATEWAY-7890123456_______\0" };

    let fqin = ServiceFqin {
        vm_name: "".to_string(),
        package_name: "android.sdv.displaysafety".to_string(),
        service_name: "DriverUIService".to_string(),
        instance_name: "default".to_string(),
    };

    register_service(
        &publickey,
        &fqin,
        /* custom-metadata: */ "".as_bytes().to_vec(),
        DRIVERUI_RPC_SERVER_PORT,
    )
    .map_err(|err| format!("Failed to register HAR-SDV DriverUI proxy service: {:?}", err))
}

pub(crate) fn register_camera_proxy() -> Result<(), String> {
    let camera_fqin = ServiceFqin {
        vm_name: "".to_string(),
        package_name: "android.sdv.displaysafety".to_string(),
        service_name: "CameraService".to_string(),
        instance_name: "default".to_string(),
    };

    let camera_publickey = PublicKey { value: *b"HARSDVGATEWAY-CAMERA-7890123456\0" };
    register_service(
        &camera_publickey,
        &camera_fqin,
        /* custom-metadata: */ "".as_bytes().to_vec(),
        CAMERA_RPC_SERVER_PORT,
    )
    .map_err(|err| format!("Failed to register Camera proxy service: {:?}", err))
}

pub fn proxy_vehicledata_to_qnx(
    ip: &str,
    grpc_env: Arc<Environment>,
    mapper: Arc<SdvToHarMapper<HashMapTopicMapper>>,
) -> JoinHandle<()> {
    info!("Starting QNX data service");
    let qnx_rpc_server_address = ip.to_owned() + ":50051";
    // Start SDV Data tunnel services.
    // TODO: handle errors, use a different transport
    let ch = ChannelBuilder::new(grpc_env)
        .initial_reconnect_backoff(Duration::from_millis(10))
        .max_reconnect_backoff(Duration::from_millis(50))
        .connect(&qnx_rpc_server_address);
    let vehicle_data_client = VehicleDataServiceClient::new(ch);
    thread::spawn(move || {
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
    })
}

pub fn proxy_vehicledata_to_harry(
    grpc_env: Arc<Environment>,
    mapper: Arc<SdvToHarMapper<HashMapTopicMapper>>,
) -> JoinHandle<()> {
    // TODO: handle errors, use a different transport
    let ch = ChannelBuilder::new(grpc_env)
        .initial_reconnect_backoff(Duration::from_millis(10))
        .max_reconnect_backoff(Duration::from_millis(50))
        // TODO: Extract IP/port to common.
        .connect("127.0.0.1:50051");
    let vehicle_data_client = VehicleDataServiceClient::new(ch);
    let mapper_cloned = mapper.clone();
    thread::spawn(move || {
        // TODO: Do this in some sort of a loop to make sure it never stops
        match vehicle_data_client.receive_vehicle_data() {
            Ok((vehicle_data_sender, _vehicle_data_receiver)) => {
                // Connection is established
                let vehicle_data_sender = Arc::new(Mutex::new(vehicle_data_sender));
                let mut sdv_service =
                    create_sdv_data_service(vehicle_data_sender.clone(), mapper_cloned);
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
    })
}

pub fn proxy_user_preferences_to_harry(
    grpc_env: Arc<Environment>,
    mapper: Arc<SdvToHarMapper<HashMapTopicMapper>>,
) -> HarPreferencesClient {
    // Start SDV Data tunnel services for User Prefs to HAR.
    // TODO: handle errors, use a different transport
    let ch = ChannelBuilder::new(grpc_env)
        .initial_reconnect_backoff(Duration::from_millis(10))
        .max_reconnect_backoff(Duration::from_millis(50))
        .connect("127.0.0.1:50051");
    let vehicle_data_client = VehicleDataServiceClient::new(ch);

    // Setup user preferences
    let prefs_client =
        create_har_user_preferences_client(vehicle_data_client.clone(), mapper.clone());

    info!("HAR User Preferences services started.");
    prefs_client
}
