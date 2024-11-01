// Copyright 2023 Google LLC

use crate::HashMapTopicMapper;

/// Creates the mapper.
pub fn create_topic_map() -> HashMapTopicMapper {
    let mut map = HashMapTopicMapper::new();

    map.add("TirePressure.REAR_LEFT", "tire_pressure_rear_left");
    map.add("TirePressure.FRONT_RIGHT", "tire_pressure_front_right");
    map.add("TirePressure.REAR_RIGHT", "tire_pressure_rear_right");
    map.add("TirePressure.FRONT_LEFT", "tire_pressure_front_left");
    map.add("TirePressure.FIFTH_WHEEL", "tire_pressure_fifth_wheel");
    map.add("VehicleSpeed.VEHICLE_SPEED", "vehicle_speed");
    map.add("VehicleSpeed.SPEEDLIMIT", "speed_limit");
    map.add("VehicleSpeed.MAX_SPEED", "max_speed");
    map.add("CurrentGear", "vehicle_gear");
    map.add("TellTaleStatus.PARK_LIGHTS", "park_lights");
    map.add("TellTaleStatus.ADAS", "adas");
    map.add("TellTaleStatus.FOG_LIGHTS", "fog_lights");
    map.add("TellTaleStatus.TRACTION", "traction");
    map.add("TellTaleStatus.SEATBELT_PASSENGER", "seatbelt_passenger");
    map.add("TellTaleStatus.CHECK_ENGINE", "check_engine");
    map.add("TellTaleStatus.OIL_PRESSURE", "oil_pressure");
    map.add("TellTaleStatus.BRAKE", "brake");
    map.add("TellTaleStatus.LOWBEAM", "lowbeam");
    map.add("TellTaleStatus.SEATBELT_DRIVER", "seatbelt_driver");
    map.add("TellTaleStatus.CHARGING_FAILURE", "charging_failure");
    map.add("TellTaleStatus.MAX_SPEED_DISPLAYED", "max_speed_displayed");
    map.add("TellTaleStatus.ENGINE_TEMP", "engine_temp");
    map.add("TellTaleStatus.AIRBAG", "airbag");
    map.add("TellTaleStatus.EMERGENCY_LIGHT", "emergency_light");
    map.add("TellTaleStatus.ABS", "abs");
    map.add("TellTaleStatus.HIBEAM", "hibeam");
    map.add("TellTaleStatus.TURN_SIGNAL_RIGHT", "turn_signal_right");
    map.add("TellTaleStatus.LOW_TIRE_PRESSURE", "low_tire_pressure");
    map.add("TellTaleStatus.TURN_SIGNAL_LEFT", "turn_signal_left");
    map.add("TellTaleStatus.SPEED_LIMIT_DISPLAYED", "speed_limit_displayed");

    map
}
