#![allow(missing_docs)]
// Copyright 2023 Google LLC

use core::time::Duration;
use log::info;
use log::warn;
use oem_harry_vehicle_messages_catalog::vehicledata::Gear;
use sdvgenerated::harry_vehicle_data_publisher::HarryVehicleDataPublisher;
use sdvgenerated::harry_vehicle_data_publisher::HarryVehicleDataPublisherCallbacks;
use sdvgenerated::topics::publisher::TellTaleStatusTopics;
use sdvgenerated::topics::publisher::TirePressureTopics;
use sdvgenerated::topics::publisher::VehicleSpeedTopics;
use utils::*;

mod utils;

#[tokio::main]
async fn main() -> Result<(), ()> {
    info!("HAR-SDV service running.");

    let mut service = HarryVehicleDataPublisher::new(HarryVehicleDataPublisherCallbacks {});
    if let Err(err) = service.start() {
        warn!("Service starting failed: {:?}", err);
        return Err(());
    }

    loop {
        let steps = StepsBuilder::new()
            // Driver enters vehicle. Car in Parked mode.
            .and_then(Task::set_gear(Gear::P))
            // Set tire pressure
            .and_then(Task::set_tire_pressure(TirePressureTopics::REAR_LEFT, 38i32))
            .and_then(Task::set_tire_pressure(TirePressureTopics::REAR_RIGHT, 38i32))
            .and_then(Task::set_tire_pressure(TirePressureTopics::FRONT_LEFT, 38i32))
            .and_then(Task::set_tire_pressure(TirePressureTopics::FRONT_RIGHT, 38i32))
            // Add a delay so we can see the initial state too
            .and_then(Task::delay(Duration::from_secs(1)))
            // All telltales light up briefly, turn off.
            .and_then(Task::set_all_telltales_alert(true))
            .and_then(Task::delay(Duration::from_secs(1)))
            .and_then(Task::set_all_telltales_alert(false))
            // Seatbelt warning is on.  Seatbelt warning turns off after
            // a couple of seconds indicating driver put on seatbelt.
            .and_then(Task::set_telltale_alert(TellTaleStatusTopics::SEATBELT_DRIVER, true))
            .and_then(Task::delay(Duration::from_secs(4)))
            .and_then(Task::set_telltale_alert(TellTaleStatusTopics::SEATBELT_DRIVER, false))
            // Gear shifts to Reverse for about 5 seconds
            .and_then(Task::set_gear(Gear::R))
            .and_then(Task::change_speed(
                VehicleSpeedTopics::VEHICLE_SPEED,
                0i32,
                5i32,
                Duration::from_secs(3),
            ))
            .and_then(Task::change_speed(
                VehicleSpeedTopics::VEHICLE_SPEED,
                5i32,
                0i32,
                Duration::from_secs(2),
            ))
            .and_then(Task::delay(Duration::from_secs(1)))
            // Gear shifts to Drive
            .and_then(Task::set_gear(Gear::D))
            // Speed goes 0mph to about 40mph gradually in about 5 seconds
            .and_then(Task::change_speed(
                VehicleSpeedTopics::VEHICLE_SPEED,
                0i32,
                40i32,
                Duration::from_secs(5),
            ))
            // Left Indicator turns on for about 3 to 5 seconds
            .and_then(Task::set_telltale_alert(TellTaleStatusTopics::TURN_SIGNAL_LEFT, true))
            .and_then(Task::change_speed(
                VehicleSpeedTopics::VEHICLE_SPEED,
                40i32,
                20i32,
                Duration::from_secs(5),
            ))
            // Turn completes
            .and_then(Task::set_telltale_alert(TellTaleStatusTopics::TURN_SIGNAL_LEFT, false))
            // Speed increases gradually from 20mph to about 40 mph
            .and_then(Task::change_speed(
                VehicleSpeedTopics::VEHICLE_SPEED,
                20i32,
                40i32,
                Duration::from_secs(5),
            ))
            // Right Indicator turns on for about 3 to 5 seconds (4 sec)
            .and_then(Task::set_telltale_alert(TellTaleStatusTopics::TURN_SIGNAL_RIGHT, true))
            .and_then(Task::change_speed(
                VehicleSpeedTopics::VEHICLE_SPEED,
                40i32,
                20i32,
                Duration::from_secs(4),
            ))
            // Turn completes
            .and_then(Task::set_telltale_alert(TellTaleStatusTopics::TURN_SIGNAL_RIGHT, false))
            // Speed goes up to about 40mph gradually in about 5 seconds
            .and_then(Task::change_speed(
                VehicleSpeedTopics::VEHICLE_SPEED,
                20i32,
                40i32,
                Duration::from_secs(5),
            ))
            // Speed goes down to 0mph in about 5 seconds
            .and_then(Task::change_speed(
                VehicleSpeedTopics::VEHICLE_SPEED,
                20i32,
                0i32,
                Duration::from_secs(5),
            ))
            // Gear shifts to Park
            .and_then(Task::set_gear(Gear::P))
            // Driver removes seatbelt.  Seatbelt warning comes on.
            .and_then(Task::set_telltale_alert(TellTaleStatusTopics::SEATBELT_DRIVER, true))
            // Done
            .build();

        play_steps(&mut service, steps).await;
        // Wait a little and start again.
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
