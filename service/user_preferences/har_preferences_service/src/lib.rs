// Copyright 2024 Google LLC

//! HAR Preferences Service declares the user preferences supported using
//! some of the SDV User preferences APIs.

use std::time::Duration;
use std::sync::Arc;
use sdvgenerated_har_preferences_service::sdv_user_preferences_user_preferences_registry_service_interface::UserPreferencesRegistryService;
use crate::user_controllable_preferences_impl::UserControllablePreferencesServiceImpl;
use crate::settings::setting_definitions;
use crate::settings::SETTINGS_GROUP_NAME;
use user_preferences_api::user_preferences_registry_service::RegisterSettingsRequest;
use sdvgenerated_har_preferences_service::har_preferences_service::HarPreferencesService;
use log::info;
use log::warn;
use crate::settings::default_settings;

mod settings;
mod user_controllable_preferences_impl;

pub(crate) const HAR_PREFERENCES_SERVICE_FQIN: &str = "HarPreferencesService";

/// Service bundle for the User Preferences Service.
pub struct HarUserPreferencesServiceBundle {
    _context: ContextRef,
    preferences_service: Option<HarPreferencesService>,
}

// Register the new service bundle.
sdv_lifecycle_client::register_service_bundle!(HarUserPreferencesServiceBundle);

impl ServiceBundle for HarUserPreferencesServiceBundle {
    /// Creates a new instance of the HarUserPreferencesServiceBundle.
    /// Called when service bundle is created by the system.
    ///
    /// Context object is provided as a parameter that gives access to the
    /// communication stack APIs.
    fn new(_context: ContextRef) -> HarUserPreferencesServiceBundle {
        sdv_log::init_logger("har_user_preferences_bundle")
            .unwrap_or_else(|err| warn!("Error during logger initialization: {:?}", err));
        info!("Creating service bundle.");

        HarUserPreferencesServiceBundle { _context, preferences_service: None }
    }

    /// Called when the service bundle is started by the system.
    fn on_start(&mut self) {
        info!("HarUserPreferencesServiceBundle starting.");
        // Sleeping 5 seconds as a workaronud to allow the base preferences to start first.
        // TODO(369515367): Remove this workaround and wait for the service using Lifecycle API calls,
        // or use a better solution.
        std::thread::sleep(Duration::from_secs(2));

        // Initialize the SDV service.
        let settings = default_settings();
        let preferences_impl = Arc::new(UserControllablePreferencesServiceImpl::new(settings));
        self.preferences_service.replace(start_user_preferences_service(preferences_impl));
        info!("Service bundle started.");
    }

    /// Called when the service bundle is stopped by the system in preparation
    /// for shutdown or suspend to RAM/Disc.
    fn on_stop(&mut self) {
        let _ = self.preferences_service.take();
        info!("Service bundle stopped.");
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
