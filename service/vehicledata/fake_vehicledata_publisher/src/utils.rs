// Copyright 2023 Google LLC

use core::time::Duration;
use log::info;
use oem_harry_vehicle_messages_catalog::vehicle_tire::TirePressure;
use oem_harry_vehicle_messages_catalog::vehicledata::CurrentGear;
use oem_harry_vehicle_messages_catalog::vehicledata::Gear;
use oem_harry_vehicle_messages_catalog::vehicledata::TellTaleStatus;
use oem_harry_vehicle_messages_catalog::vehicledata::VehicleSpeed;
use sdv_middleware_pubsub::EnumTopic;
use sdvgenerated::harry_vehicle_data_publisher::HarryVehicleDataPublisher;
use sdvgenerated::topics::publisher::CurrentGearTopics;
use sdvgenerated::topics::publisher::TellTaleStatusTopics;
use sdvgenerated::topics::publisher::TirePressureTopics;
use sdvgenerated::topics::publisher::VehicleSpeedTopics;

pub enum Task {
    ChangeSpeed(ChangeSpeed),
    SetTelltale(SetTelltale),
    SetAllTelltales(bool),
    SetTirePressure(SetTirePressure),
    SetGear(SetGear),
    Delay(Duration),
}

impl Task {
    pub fn change_speed(topic: VehicleSpeedTopics, from: i32, to: i32, duration: Duration) -> Self {
        Task::ChangeSpeed(ChangeSpeed { topic, from, to, duration })
    }

    pub fn set_telltale_alert(topic: TellTaleStatusTopics, alert: bool) -> Self {
        Task::SetTelltale(SetTelltale { topic, alert })
    }

    pub fn set_all_telltales_alert(alert: bool) -> Self {
        Task::SetAllTelltales(alert)
    }

    pub fn set_tire_pressure(topic: TirePressureTopics, pressure: i32) -> Self {
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
    pub topic: VehicleSpeedTopics,
    pub from: i32,
    pub to: i32,
    pub duration: Duration,
}

pub struct SetTelltale {
    pub topic: TellTaleStatusTopics,
    pub alert: bool,
}

pub struct SetTirePressure {
    pub topic: TirePressureTopics,
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

pub async fn play_steps(service: &mut HarryVehicleDataPublisher, steps: Vec<Task>) {
    for step in steps {
        match step {
            Task::ChangeSpeed(t) => {
                info!("Changing speed from {} to {} in {:?}", t.from, t.to, t.duration);
                let delta = (t.from - t.to).abs();
                let wait_duration = t.duration.div_f32(delta as _);
                let mut message = VehicleSpeed::new();

                let range: Box<dyn Iterator<Item = i32>> = if t.from <= t.to {
                    Box::new(t.from..=t.to)
                } else {
                    Box::new((t.to..=t.from).rev())
                };
                for speed in range {
                    message.speed = speed as u32;
                    if let Err(err) = service.vehicle_speed_publish(&t.topic, &message) {
                        info!("Error publishing {:?}", err);
                    } else {
                        info!("Published to '{}': {:?}", t.topic.get_name(), message);
                    }
                    tokio::time::sleep(wait_duration).await;
                }
            }
            Task::SetTelltale(t) => {
                info!("Setting telltale {} to {}", t.topic.get_name(), t.alert);
                let mut message = TellTaleStatus::new();
                message.alert = t.alert;
                if let Err(err) = service.tell_tale_status_publish(&t.topic, &message) {
                    info!("Error publishing {:?}", err);
                } else {
                    info!("Published to '{}': {:?}", t.topic.get_name(), message);
                }
            }
            Task::SetAllTelltales(alert) => {
                info!("Setting all telltales to {}", alert);
                let mut message = TellTaleStatus::new();
                message.alert = alert;

                for topic in TellTaleStatusTopics::iterator() {
                    if let Err(err) = service.tell_tale_status_publish(topic, &message) {
                        info!("Error publishing {:?}", err);
                    } else {
                        info!("Published to '{}': {:?}", topic.get_name(), message);
                    }
                }
            }
            Task::SetTirePressure(t) => {
                info!("Setting tire pressure {} to {}", t.topic.get_name(), t.pressure);
                let mut message = TirePressure::new();
                message.pressure = t.pressure as u32;
                if let Err(err) = service.tire_pressure_publish(&t.topic, &message) {
                    info!("Error publishing {:?}", err);
                } else {
                    info!("Published to '{}': {:?}", t.topic.get_name(), message);
                }
            }
            Task::SetGear(t) => {
                info!("Setting gear to {:?} to {:?}", CurrentGearTopics::GEAR.get_name(), t.gear);
                let mut message = CurrentGear::new();
                message.gear = t.gear.into();
                if let Err(err) = service.current_gear_publish(&CurrentGearTopics::GEAR, &message) {
                    info!("Error publishing {:?}", err);
                } else {
                    info!("Published to '{}': {:?}", CurrentGearTopics::GEAR.get_name(), message);
                }
            }
            Task::Delay(duration) => {
                info!("Waiting {:?}", duration);
                tokio::time::sleep(duration).await;
            }
        }
    }
}
