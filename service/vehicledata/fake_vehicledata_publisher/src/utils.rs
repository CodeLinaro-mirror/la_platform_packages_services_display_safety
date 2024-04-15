// Copyright 2023 Google LLC

use core::time::Duration;
use harry_vehicle_data_grpc::vehicle_tire::Location;
use harry_vehicle_data_grpc::vehicle_tire::TirePressure;
use harry_vehicle_data_grpc::vehicledata::CurrentGearTopic;
use harry_vehicle_data_grpc::vehicledata::Gear;
use harry_vehicle_data_grpc::vehicledata::Telltale;
use harry_vehicle_data_grpc::vehicledata::VehicleSpeedTopic;
use harry_vehicle_data_grpc::vehicledata::*;
use harry_vehicle_data_grpc::vehicledata_grpc_service::*;
use harry_vehicle_data_grpc::vehicledata_grpc_service_grpc::SdvVehicleDataGrpcClient;
use log::info;
use log::trace;
use log::warn;
use protobuf::MessageField;
use std::fmt::Debug;

// TODO: Find a way to get these from the GRPC obj.
const TALLTALE_TOPIC_VALUES: &[Telltale] = &[
    Telltale::OIL_PRESSURE,
    Telltale::ENGINE_TEMP,
    Telltale::CHECK_ENGINE,
    Telltale::CHARGING_FAILURE,
    Telltale::SEATBELT_DRIVER,
    Telltale::SEATBELT_PASSENGER,
    Telltale::LOW_TIRE_PRESSURE,
    Telltale::AIRBAG,
    Telltale::ABS,
    Telltale::BRAKE,
    Telltale::TRACTION,
    Telltale::FOG_LIGHTS,
    Telltale::PARK_LIGHTS,
    Telltale::HIBEAM,
    Telltale::LOWBEAM,
    Telltale::TURN_SIGNAL_LEFT,
    Telltale::TURN_SIGNAL_RIGHT,
    Telltale::ADAS,
    Telltale::MAX_SPEED_DISPLAYED,
    Telltale::SPEED_LIMIT_DISPLAYED,
    Telltale::EMERGENCY_LIGHT,
];

pub enum Task {
    ChangeSpeed(ChangeSpeed),
    SetTelltale(SetTelltale),
    SetAllTelltales(bool),
    SetTirePressure(SetTirePressure),
    SetGear(SetGear),
    Delay(Duration),
}

impl Task {
    pub fn change_speed(topic: VehicleSpeedTopic, from: i32, to: i32, duration: Duration) -> Self {
        Task::ChangeSpeed(ChangeSpeed { topic, from, to, duration })
    }

    pub fn set_telltale_alert(topic: Telltale, alert: bool) -> Self {
        Task::SetTelltale(SetTelltale { topic, alert })
    }

    pub fn set_all_telltales_alert(alert: bool) -> Self {
        Task::SetAllTelltales(alert)
    }

    pub fn set_tire_pressure(topic: Location, pressure: i32) -> Self {
        Task::SetTirePressure(SetTirePressure { topic, pressure })
    }

    pub fn set_gear(gear: Gear) -> Self {
        Task::SetGear(SetGear { gear })
    }

    pub fn delay(duration: Duration) -> Self {
        Task::Delay(duration)
    }
}

pub struct ChangeSpeed {
    pub topic: VehicleSpeedTopic,
    pub from: i32,
    pub to: i32,
    pub duration: Duration,
}

pub struct SetTelltale {
    pub topic: Telltale,
    pub alert: bool,
}

pub struct SetTirePressure {
    pub topic: Location,
    pub pressure: i32,
}

pub struct SetGear {
    pub gear: Gear,
}

pub struct StepsBuilder {
    pub steps: Vec<Task>,
}

impl StepsBuilder {
    pub fn new() -> Self {
        StepsBuilder { steps: Vec::new() }
    }

    pub fn and_then(mut self, task: Task) -> Self {
        self.steps.push(task);
        self
    }

    pub fn build(self) -> Vec<Task> {
        self.steps
    }
}

pub async fn play_steps(service: &mut SdvVehicleDataGrpcClient, steps: Vec<Task>) {
    for step in steps {
        match step {
            Task::ChangeSpeed(t) => {
                info!("Changing speed from {} to {} in {:?}", t.from, t.to, t.duration);
                let delta = (t.from - t.to).abs();
                let wait_duration = t.duration.div_f32(delta as _);

                let range: Box<dyn Iterator<Item = i32>> = if t.from <= t.to {
                    Box::new(t.from..=t.to)
                } else {
                    Box::new((t.to..=t.from).rev())
                };
                for speed in range {
                    let message = PublishVehicleSpeedRequest {
                        topic: t.topic.into(),
                        speed: MessageField::some(VehicleSpeed {
                            speed: speed as u32,
                            ..Default::default()
                        }),
                        ..Default::default()
                    };
                    log_result(&message, service.publish_vehicle_speed(&message));
                    tokio::time::sleep(wait_duration).await;
                }
            }
            Task::SetTelltale(t) => {
                info!("Setting telltale {:?} to {}", t.topic, t.alert);

                let message = PublishTelltaleStatusRequest {
                    topic: t.topic.into(),
                    status: MessageField::some(TellTaleStatus {
                        alert: t.alert,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                log_result(&message, service.publish_telltale_status(&message));
            }
            Task::SetAllTelltales(alert) => {
                info!("Setting all telltales to {:?}", alert);

                for topic in TALLTALE_TOPIC_VALUES.iter() {
                    let message = PublishTelltaleStatusRequest {
                        topic: (*topic).into(),
                        status: MessageField::some(TellTaleStatus { alert, ..Default::default() }),
                        ..Default::default()
                    };
                    log_result(&message, service.publish_telltale_status(&message));
                }
            }
            Task::SetTirePressure(t) => {
                info!("Setting tire pressure {:?} to {}", t.topic, t.pressure);

                let message = PublishTirePressureRequest {
                    topic: t.topic.into(),
                    pressure: MessageField::some(TirePressure {
                        pressure: t.pressure as u32,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                log_result(&message, service.publish_tire_pressure(&message));
            }
            Task::SetGear(t) => {
                info!("Setting gear to {:?} to {:?}", CurrentGearTopic::GEAR, t.gear);

                let message = PublishCurrentGearRequest {
                    topic: CurrentGearTopic::GEAR.into(),
                    gear: MessageField::some(CurrentGear {
                        gear: t.gear.into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                log_result(&message, service.publish_current_gear(&message));
            }
            Task::Delay(duration) => {
                info!("Waiting {:?}", duration);
                tokio::time::sleep(duration).await;
            }
        }
    }
}

fn log_result<Ok: Debug, Err: Debug, Message: Debug>(message: &Message, result: Result<Ok, Err>) {
    if let Err(err) = result {
        warn!("Error publishing {:?}: {:?}", message, err);
    } else {
        trace!("Published {:?}", message);
    }
}
