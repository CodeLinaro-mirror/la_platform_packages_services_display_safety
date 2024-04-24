// Copyright 2024 Google LLC

//! This is a copy of the SDV User Preferences Service implementation.
// Keeping it as close to the original as possible.
// This service might be removed at a later step if we can reuse the SDV
// implementation or keep it to add our customized logic.
// See original at http://ac/system/software_defined_vehicle/automotive_services/samples/user_preferences/user_preferences_service/

mod macros;
mod setting;
mod settings_group;
mod settings_group_id;
mod user_preferences_service_impl;
mod vehicle_configuration;

use log::info;
use sdvgenerated_har_user_preferences_server::user_preferences_server::UserPreferencesServer;
use std::sync::Arc;
use user_preferences_service_impl::UserPreferencesServiceImpl;

fn main() {
    sdv_log::init_logger("har_user_preferences").unwrap_or_else(|error| match &error {
        sdv_log::LoggerError::AlreadyInitializedError(_) => {
            // Only inform error, not panic
            log::info!("{}", error)
        }
        _ => panic!("{}", error),
    });

    let user_preferences_impl = Arc::new(UserPreferencesServiceImpl::new());
    let service = UserPreferencesServer::new();

    service
        .sdv_user_preferences_user_preferences_registry_service_server
        .register_service_interface(user_preferences_impl.clone());
    service
        .sdv_user_preferences_user_preferences_admin_service_server
        .register_service_interface(user_preferences_impl.clone());
    service
        .sdv_user_preferences_user_preferences_management_service_server
        .register_service_interface(user_preferences_impl.clone());

    info!("UserPreferencesServer service bundle started.");
    loop {
        std::thread::park();
    }
}
