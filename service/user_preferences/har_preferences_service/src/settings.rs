// Copyright 2023 Google LLC

use har_grpc_services::preferences::DistanceUnit;
use har_grpc_services::preferences::TemperatureUnit;
use protobuf::Enum;
use protobuf::{EnumOrUnknown, MessageField};
use std::collections::HashMap;
use user_preferences_api::setting::setting::Value;
use user_preferences_api::setting::setting_and_constraints::Constraints;
use user_preferences_api::setting::EnumConstraints;
use user_preferences_api::setting::{
    Setting, SettingAndConstraints, SettingDefinition, SettingKind,
};

// TODO(b/291979113): Extract to a shared module.
// When changing these constants, also change them in:
//    har-sdv-service/src/preferences.rs,
//    user_preferences/har_preferences_admin/src/main.rs
pub(crate) const SETTINGS_GROUP_NAME: &str = "HAR";
pub(crate) const DISTANCE_UNITS: &str = "DISTANCE_UNITS";
pub(crate) const TEMPERATURE_UNITS: &str = "TEMPERATURE_UNITS";

pub type SettingsMap = HashMap<String, Value>;

pub fn default_settings() -> SettingsMap {
    setting_definitions()
        .into_iter()
        .map(|(key, setting_definition)| {
            (
                key,
                setting_definition.setting_and_constraints.unwrap().setting.unwrap().value.unwrap(),
            )
        })
        .collect()
}

pub fn setting_definitions() -> HashMap<String, SettingDefinition> {
    let mut settings: HashMap<String, SettingDefinition> = HashMap::new();

    settings.insert(
        DISTANCE_UNITS.to_string(),
        SettingDefinition {
            kind: EnumOrUnknown::new(SettingKind::PER_USER),
            setting_and_constraints: MessageField::some(SettingAndConstraints {
                setting: MessageField::some(Setting {
                    key: DISTANCE_UNITS.to_string(),
                    value: Some(Value::Enum(DistanceUnit::MILES.value())),
                    ..Default::default()
                }),
                constraints: Some(Constraints::EnumConstraints(EnumConstraints {
                    possible_values: vec![
                        DistanceUnit::KILOMETERS.value(),
                        DistanceUnit::MILES.value(),
                    ],
                    ..Default::default()
                })),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    settings.insert(
        TEMPERATURE_UNITS.to_string(),
        SettingDefinition {
            kind: EnumOrUnknown::new(SettingKind::PER_USER),
            setting_and_constraints: MessageField::some(SettingAndConstraints {
                setting: MessageField::some(Setting {
                    key: TEMPERATURE_UNITS.to_string(),
                    value: Some(Value::Enum(TemperatureUnit::FAHRENHEIT.value())),
                    ..Default::default()
                }),
                constraints: Some(Constraints::EnumConstraints(EnumConstraints {
                    possible_values: vec![
                        TemperatureUnit::CELSIUS.value(),
                        TemperatureUnit::FAHRENHEIT.value(),
                    ],
                    ..Default::default()
                })),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    settings
}
