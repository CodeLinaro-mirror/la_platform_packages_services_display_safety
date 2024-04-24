// Copyright 2023 Google LLC

use itertools::Itertools;
use protobuf::MessageField;
use user_preferences_api::error::Error;
use user_preferences_api::{setting::Setting, user_preferences_registry_service};
use log::info;
use sdvmiddleware_rpc_grpc::grpc_client_transport::GrpcClientTransport;
use user_preferences_api::user_controllable_service::{
    FactoryResetRequest, FactoryResetResponse, RequestSettingsChangeRequest,
    RequestSettingsChangeResponse,
};
use sdvgenerated_har_preferences_service::sdv_user_preferences_user_controllable_user_controllable_service_interface::UserControllableService;
use sdvgenerated_har_preferences_service::sdv_user_preferences_user_preferences_registry_service_interface::UserPreferencesRegistryService;
use sdvgenerated_rpc_sdv_user_preferences_user_preferences_registry_service::client::UserPreferencesRegistryServiceClient;
use sdvmiddleware_rpc::error::RpcError;
use crate::settings::default_settings;
use crate::settings::SettingsMap;
use std::sync::Mutex;
use std::sync::Arc;
use std::thread;

use crate::settings::SETTINGS_GROUP_NAME;
use crate::HAR_PREFERENCES_SERVICE_FQIN;

/// SDV Service that handles settings change requests.
/// Implements the UserControllableService trait from SDV user preferences.
pub(crate) struct UserControllablePreferencesServiceImpl {
    settings: Arc<Mutex<SettingsMap>>,
}

impl UserControllablePreferencesServiceImpl {
    pub fn new(settings: SettingsMap) -> Self {
        Self { settings: Arc::new(Mutex::new(settings)) }
    }

    fn client() -> UserPreferencesRegistryServiceClient {
        UserPreferencesRegistryServiceClient::new(Box::new(GrpcClientTransport::new(
            "UserPreferencesServer",
        )))
    }
}

impl UserControllableService for UserControllablePreferencesServiceImpl {
    fn request_settings_change(
        &self,
        request: RequestSettingsChangeRequest,
    ) -> Result<RequestSettingsChangeResponse, RpcError> {
        let settings = self.settings.clone();
        thread::spawn(move || {
            let mut settings = settings.lock().expect("Cannot lock settings");
            info!("Settings change request, simulating setting change delay\n\n");
            request.settings.iter().for_each(|setting| {
                settings.insert(setting.key.clone(), setting.value.clone().unwrap());
            });
            info!("Updated settings");
            settings.iter().for_each(|(key, value)| info!("Key {:<15} Value {:<10?}", key, value));

            let client = Self::client();
            let response = client
                .update_settings(user_preferences_registry_service::UpdateSettingsRequest {
                    group_name: request.group_name,
                    settings: request.settings,
                    caller_fqin: HAR_PREFERENCES_SERVICE_FQIN.to_string(),
                    ..Default::default()
                })
                .expect("Failed to perform GRPC to update settings");
            if response.error.is_some() {
                panic!("Failed update settings of current user {:#?}", response.error.unwrap());
            }
        });

        // This result only says the message was received.
        Ok(RequestSettingsChangeResponse::new())
    }

    fn factory_reset(
        &self,
        _request: FactoryResetRequest,
    ) -> Result<FactoryResetResponse, RpcError> {
        let default_settings = default_settings()
            .into_iter()
            .map(|(name, value)| Setting { key: name, value: Some(value), ..Default::default() })
            .collect_vec();

        let result = self
            .request_settings_change(RequestSettingsChangeRequest {
                group_name: SETTINGS_GROUP_NAME.to_string(),
                settings: default_settings,
                ..Default::default()
            })
            .unwrap();

        let mut response = FactoryResetResponse::new();

        if let Some(err) = result.error.into_option() {
            response.error = MessageField::some(Error {
                error_code: err.error_code,
                message: format!("Failed to perform factory reset: {}", err),
                ..Default::default()
            });
        } else {
            info!("\nPerformed factory reset");
        }
        Ok(response)
    }
}
