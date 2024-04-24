// Copyright 2024 Google LLC

//! HAR Preferences Admin is a sample app that changes user preferences.
//! It is for demonstration purposes only. On a production vehicle,
//! it is likely IVI that initiates settings changes.

use har_grpc_services::preferences::DistanceUnit;
use har_grpc_services::preferences::TemperatureUnit;
use log::error;
use log::info;
use protobuf::MessageField;
use std::sync::Arc;
use user_preferences_api::setting::setting::Value;
use user_preferences_api::setting::Setting;
use user_preferences_api::settings_group::SettingsGroupId;
use user_preferences_api::user_preferences_management_service::RequestSettingsChangeRequest;

use user_preferences_api::user::user::UserFlags;
use user_preferences_api::user::User;
use user_preferences_api::user_preferences_admin_service::CreateUserRequest;
use sdvgenerated_har_preferences_admin::har_preferences_admin::HarPreferencesAdmin;
use sdvgenerated_har_preferences_admin::sdv_user_preferences_user_preferences_management_service_interface::UserPreferencesManagementService;
use protobuf::Enum;
use sdvgenerated_har_preferences_admin::sdv_user_preferences_user_preferences_admin_service_interface::UserPreferencesAdminService;
use user_preferences_api::user_preferences_admin_service::SelectUserRequest;

pub(crate) const SETTINGS_GROUP_NAME: &str = "HAR";
pub(crate) const HAR_PREFERENCES_SERVICE_FQIN: &str = "HarPreferencesService";
pub(crate) const DISTANCE_UNITS: &str = "DISTANCE_UNITS";
pub(crate) const TEMPERATURE_UNITS: &str = "TEMPERATURE_UNITS";

// This is a utility to change preferences or users.
// On a production system, we expect the IVI system to make these changes.

fn main() -> Result<(), String> {
    sdv_log::init_logger("har_preferences_admin").unwrap_or_else(|error| match &error {
        sdv_log::LoggerError::AlreadyInitializedError(_) => {
            // Only inform error, not panic
            log::info!("{}", error)
        }
        _ => panic!("{}", error),
    });

    let har_preferences_service = HarPreferencesAdmin::new();

    let user_preferences_admin_client = har_preferences_service
        .get_sdv_user_preferences_user_preferences_admin_service_client("UserPreferencesServer");

    let user_preferences_management_client = har_preferences_service
        .get_sdv_user_preferences_user_preferences_management_service_client(
            "UserPreferencesServer",
        );

    // user_id = 1 is reserved, using the next available ID to create a new user.
    let user_id = 2_i32;
    let response = user_preferences_admin_client
        .create_user(CreateUserRequest {
            user: MessageField::some(User {
                id: user_id,
                flags: UserFlags::DRIVER.value() | UserFlags::EPHEMERAL.value(),
                ..Default::default()
            }),
            ..Default::default()
        })
        .expect("Failed to perform request to create user");
    if let Some(error) = response.error.into_option() {
        error!("Failed to create user {}: {:?}", user_id, error);
    } else {
        info!("Successfully created user {}\n", user_id);
    }

    let response = user_preferences_admin_client
        .select_user(SelectUserRequest { user_id, ..Default::default() })
        .expect("Failed to perform request to select user");
    if let Some(error) = response.error.into_option() {
        error!("Failed to select user {}: {:?}", user_id, error);
    } else {
        info!("Successfully selected user: {}", user_id);
    }

    // Set to kilometer
    info!("Adjusting Distance Units to Kilometers");
    change_settings(
        &user_preferences_management_client,
        SETTINGS_GROUP_NAME.to_string(),
        DISTANCE_UNITS.to_string(),
        Value::Enum(DistanceUnit::KILOMETERS.value()),
    )
    .map_err(|error| format!("Error changing settings: {:?}", error))?;

    // Set to celsius
    info!("Adjusting Temperature Units to Celsius");
    change_settings(
        &user_preferences_management_client,
        SETTINGS_GROUP_NAME.to_string(),
        TEMPERATURE_UNITS.to_string(),
        Value::Enum(TemperatureUnit::CELSIUS.value()),
    )
    .map_err(|error| format!("Error changing settings: {:?}", error))?;
    info!("Successfully set user preferences");
    Ok(())
}

fn change_settings<ManagementClientType: UserPreferencesManagementService>(
    management_client: &Arc<ManagementClientType>,
    setting_group_name: String,
    settings_key: String,
    value: Value,
) -> Result<(), String> {
    let response = management_client
        .request_settings_change(RequestSettingsChangeRequest {
            settings_group_id: MessageField::some(SettingsGroupId {
                service_fqin: HAR_PREFERENCES_SERVICE_FQIN.to_string(),
                name: setting_group_name,
                ..Default::default()
            }),
            settings: vec![Setting { key: settings_key, value: Some(value), ..Default::default() }],
            ..Default::default()
        })
        .map_err(|err| format!("Failed to request setting change: {:?}", err))?;
    if let Some(error) = response.error.into_option() {
        return Err(format!("Failed to perform setting change {:?}", error));
    }

    Ok(())
}
