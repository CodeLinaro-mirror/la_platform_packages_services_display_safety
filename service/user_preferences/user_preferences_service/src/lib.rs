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

/// Service bundle for the User Preferences Service.
pub struct HarSdvUserPreferencesServiceBundle {
    _context: ContextRef,
    sdv_service: UserPreferencesServer,
}

// Register the new service bundle.
sdv_lifecycle_client::register_service_bundle!(HarSdvUserPreferencesServiceBundle);

impl ServiceBundle for HarSdvUserPreferencesServiceBundle {
    /// Creates a new instance of the HarSdvUserPreferencesServiceBundle.
    /// Called when service bundle is created by the system.
    ///
    /// Context object is provided as a parameter that gives access to the
    /// communication stack APIs.
    fn new(_context: ContextRef) -> HarSdvUserPreferencesServiceBundle {
        sdv_log::init_logger("har_sdv_user_preferences_bundle").unwrap();
        info!("Creating service bundle.");
        // Initialize the SDV service.
        let user_preferences_impl = Arc::new(UserPreferencesServiceImpl::new());
        let sdv_service = UserPreferencesServer::new();

        sdv_service
            .sdv_user_preferences_user_preferences_registry_service_server
            .register_service_interface(user_preferences_impl.clone());
        sdv_service
            .sdv_user_preferences_user_preferences_admin_service_server
            .register_service_interface(user_preferences_impl.clone());
        sdv_service
            .sdv_user_preferences_user_preferences_management_service_server
            .register_service_interface(user_preferences_impl.clone());

        HarSdvUserPreferencesServiceBundle { _context, sdv_service }
    }

    /// Called when the service bundle is started by the system.
    fn on_start(&mut self) {
        self.sdv_service.start().unwrap_or_else(|err| panic!("Service starting failed: {:?}", err));
        info!("Service bundle started.");
    }

    /// Called when the service bundle is stopped by the system in preparation
    /// for shutdown or suspend to RAM/Disc.
    fn on_stop(&mut self) {
        info!("Service bundle stopped.");
    }
}
