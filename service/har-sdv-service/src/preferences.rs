// Copyright 2024 Google LLC

use crate::HashMapTopicMapper;
use crate::SdvToHarMapper;
use har_grpc_services::vehicledata_grpc::VehicleDataServiceClient;
use log::debug;
use log::info;
use log::warn;
use sdvgenerated_har_preferences_client::har_preferences_client::HarPreferencesClient;
use sdvgenerated_rpc_sdv_user_preferences_view_change_notifier::interface::ChangeNotifier;
use std::sync::Arc;
use user_preferences_api::change_notifier::{OnSettingsChangeRequest, OnSettingsChangeResponse};
use user_preferences_api::setting::Setting;
use user_preferences_utils::print_settings;

use crate::send_data_blocking;
use grpcio::ClientDuplexSender;
use har_grpc_services::vehicledata::VehicleData;
use std::sync::Mutex;
use user_preferences_api::setting::setting::Value;
use user_preferences_api::settings_group::SettingsGroupId;
use protobuf::MessageField;
use user_preferences_api::user_preferences_management_service::SubscribeToSettingsChangeAndGetSettingsRequest;
use sdvgenerated_har_preferences_client::sdv_user_preferences_user_preferences_management_service_interface::UserPreferencesManagementService;
use user_preferences_api::setting::SettingAndConstraints;

pub(crate) const SETTINGS_GROUP_NAME: &str = "HAR";
pub(crate) const HAR_PREFERENCES_SERVICE_FQIN: &str = "HarPreferencesService";

pub fn create_har_user_preferences_client(
    client: VehicleDataServiceClient,
    data_mapper: Arc<SdvToHarMapper<HashMapTopicMapper>>,
) -> HarPreferencesClient {
    let preferences_view = Arc::new(HarPreferencesView::new(client, data_mapper));
    let har_preferences_client = HarPreferencesClient::new();
    har_preferences_client
        .sdv_user_preferences_view_change_notifier_server
        .register_service_interface(preferences_view);

    let user_preferences_management_client = har_preferences_client
        .get_sdv_user_preferences_user_preferences_management_service_client(
            "UserPreferencesServer",
        );

    info!("Subscribing to Har preferences changes");
    let response = user_preferences_management_client
        .subscribe_to_settings_change_and_get_settings(
            SubscribeToSettingsChangeAndGetSettingsRequest {
                settings_group_id: MessageField::some(SettingsGroupId {
                    service_fqin: HAR_PREFERENCES_SERVICE_FQIN.to_string(),
                    name: SETTINGS_GROUP_NAME.to_string(),
                    ..Default::default()
                }),
                caller_fqin: String::from("HarPreferencesClient"),
                ..Default::default()
            },
        )
        .expect("Failed to perform request to subscribe to settings changes");

    info!("Subscribed to Har preferences changes.");

    if let Some(error) = response.error.into_option() {
        panic!("Failed to subscribe to settings change: {:?}", error);
    }
    info!("Current HAR settings");
    print_settings_and_constraints(&response.current_settings);
    har_preferences_client
}

pub fn print_settings_and_constraints(setting_and_constraints: &[SettingAndConstraints]) {
    setting_and_constraints.iter().for_each(|setting_and_constraint| {
        let setting: Setting = setting_and_constraint.setting.clone().unwrap();

        let value = option_to_string(&setting.value);
        let constraints = option_to_string(&setting_and_constraint.constraints);

        info!("Key {:<15}, Value: {:<10}, Constraint: {:<10?}", setting.key, value, constraints)
    });
}

fn option_to_string<T: std::fmt::Debug>(val: &Option<T>) -> String {
    match val {
        Some(x) => format!("{:?}", x),
        None => String::from("None"),
    }
}

struct HarPreferencesView {
    client: VehicleDataServiceClient,
    data_mapper: Arc<SdvToHarMapper<HashMapTopicMapper>>,
}

impl HarPreferencesView {
    pub fn new(
        client: VehicleDataServiceClient,
        data_mapper: Arc<SdvToHarMapper<HashMapTopicMapper>>,
    ) -> Self {
        Self { client, data_mapper }
    }
}

impl ChangeNotifier for HarPreferencesView {
    fn on_settings_change(
        &self,
        request: OnSettingsChangeRequest,
    ) -> sdvmiddleware_rpc::RpcResult<OnSettingsChangeResponse> {
        if request.settings_group_id.is_some() {
            debug!(
                "The following settings were changed in settings group {}/{}",
                request.settings_group_id.service_fqin, request.settings_group_id.name
            );
            print_settings(&request.settings);

            if request.settings_group_id.name == SETTINGS_GROUP_NAME {
                match self.client.receive_vehicle_data() {
                    Ok((vehicle_data_sender, _vehicle_data_receiver)) => {
                        let vehicle_data_sender = Arc::new(Mutex::new(vehicle_data_sender));
                        send_to_harry(vehicle_data_sender, &request.settings, &self.data_mapper);
                    }
                    Err(err) => {
                        // TODO: No panic!
                        panic!("Failed to call Vehicle data api. Err: {:?}", err);
                    }
                }
            }
        }

        Ok(OnSettingsChangeResponse::new())
    }
}

fn send_to_harry(
    sender: Arc<Mutex<ClientDuplexSender<VehicleData>>>,
    settings: &Vec<Setting>,
    mapper: &Arc<SdvToHarMapper<HashMapTopicMapper>>,
) {
    for setting in settings {
        let key = setting.key.clone();
        if let Some(setting_value) = &setting.value {
            if let Some(vehicle_data) = to_vehicle_data(mapper, key, setting_value) {
                send_data_blocking(sender.clone(), vehicle_data);
            }
        }
    }
}

fn to_vehicle_data(
    mapper: &Arc<SdvToHarMapper<HashMapTopicMapper>>,
    key: String,
    setting_value: &Value,
) -> Option<VehicleData> {
    match setting_value {
        Value::Bool(value) => Some(mapper.map_bool(key.clone(), *value)),
        Value::Float(value) => Some(mapper.map_f32(key.clone(), *value)),
        Value::Int32(value) => Some(mapper.map_i32(key.clone(), *value)),
        Value::Int64(_value) => {
            warn!("Int64 type is not supported.");
            None
        }
        Value::Blob(_value) => {
            warn!("Blob type is not supported.");
            None
        }
        Value::Enum(value) => {
            // Mapping the Enum raw int value to i32 to preserve generics.
            // Converting it back to the correct enum representation is the
            // responsilibty of the receiver of this value.
            Some(mapper.map_i32(key.clone(), *value))
        }
        _ => {
            warn!("Unknown value type for {}", &key);
            None
        }
    }
}
