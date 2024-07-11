// Copyright 2024 Google LLC

//! HAR Preferences Service declares the user preferences supported using
//! some of the SDV User preferences APIs.

use std::sync::Arc;
use sdvgenerated_har_preferences_service::sdv_user_preferences_user_preferences_registry_service_interface::UserPreferencesRegistryService;
use crate::user_controllable_preferences_impl::UserControllablePreferencesServiceImpl;
use crate::settings::setting_definitions;
use crate::settings::SETTINGS_GROUP_NAME;
use user_preferences_api::user_preferences_registry_service::RegisterSettingsRequest;
use sdvgenerated_har_preferences_service::har_preferences_service::HarPreferencesService;
use log::info;
use crate::settings::default_settings;

mod settings;
mod user_controllable_preferences_impl;

pub(crate) const HAR_PREFERENCES_SERVICE_FQIN: &str = "HarPreferencesService";

/// Starts the service.
pub fn main() {
    sdv_log::init_logger("har_preferences").unwrap();

    // Initialize user preferences
    let settings = default_settings();
    let preferences_impl = Arc::new(UserControllablePreferencesServiceImpl::new(settings));
    // Run the server
    let _preferences_service = start_user_preferences_service(preferences_impl.clone());

    loop {
        std::thread::park();
    }
}

fn start_user_preferences_service(
    preferences_impl: Arc<UserControllablePreferencesServiceImpl>,
) -> HarPreferencesService {
    // TODO: Implement persistency.
    info!("Starting user preferences services.");

    let preferences_service = HarPreferencesService::new();
    preferences_service
        .sdv_user_preferences_user_controllable_user_controllable_service_server
        .register_service_interface(preferences_impl);

    let register_settings_request = RegisterSettingsRequest {
        group_name: SETTINGS_GROUP_NAME.to_string(),
        version: String::from("1.0"),
        settings_definitions: setting_definitions().values().cloned().collect(),
        caller_fqin: HAR_PREFERENCES_SERVICE_FQIN.to_string(),
        ..Default::default()
    };
    info!(
        "Registering settings group with name: {},\nVersion: {}",
        register_settings_request.group_name, register_settings_request.version,
    );
    let registry_service_client = preferences_service
        .get_sdv_user_preferences_user_preferences_registry_service_client("UserPreferencesServer");
    let response = registry_service_client
        .register_settings(register_settings_request.clone())
        .expect("Failed to perform GRPC request to register default settings");
    if response.error.is_some() {
        panic!("Failed to register settings: {:#?}", response.error.unwrap());
    }
    info!("Registering user preferences completed. Preferences service ready.");
    preferences_service
}
