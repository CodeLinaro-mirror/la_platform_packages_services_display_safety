// Copyright 2024 Google LLC
//! Contains common functionality for the orchestrator tests.
//! See system/software_defined_vehicle/core_services/orchestration/tests/common/test_common.rs

use binder::Status;
use google_sdv_identity_aidl::aidl::google::sdv::identity::ServiceFqin::ServiceFqin;
use google_sdv_lifecycle::aidl::google::sdv::lifecycle::ILifecycleManager::{
    BpLifecycleManager, ILifecycleManager,
};
use google_sdv_lifecycle::aidl::google::sdv::lifecycle::IServiceBundleState::IServiceBundleState;
use google_sdv_lifecycle::aidl::google::sdv::lifecycle::ResponseCode::ResponseCode;
use log::debug;
use log::warn;
use std::fmt::Debug;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const SDV_SERVICE_BUNDLE_PACKAGE: &str = "com.sdv.google.display_safety.service_bundle_apex";
const SDV_SERVICE_BUNDLE_VM: &str = "local-vm";
const SDV_SERVICE_BUNDLE_INSTANCE: &str = "instance-1";
const SDV_DEFAULT_TEST_TIMEOUT_SECONDS: u64 = 30;

/// Returns the vehicle data publisher service bundle fqin
pub fn get_vehicle_data_publisher_service_bundle() -> ServiceFqin {
    ServiceFqin {
        sdvVmName: SDV_SERVICE_BUNDLE_VM.to_owned(),
        sdvPackageName: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        serviceBundleName: "HarSdvVehicleDataPublisherServiceBundle".to_owned(),
        serviceInstanceName: SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}

/// Returns the vehicle data publisher service bundle fqin
pub fn get_fake_vehicle_data_publisher_service_bundle() -> ServiceFqin {
    ServiceFqin {
        sdvVmName: "local-vm".to_owned(),
        sdvPackageName: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        serviceBundleName: "HarSdvFakeVehicleDataPublisherServiceBundle".to_owned(),
        serviceInstanceName: SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}

/// Returns the HAR-SDV user preferences (base) service bundle fqin
pub fn get_har_sdv_user_preferences_service_bundle() -> ServiceFqin {
    ServiceFqin {
        sdvVmName: "local-vm".to_owned(),
        sdvPackageName: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        serviceBundleName: "HarUserPreferencesServiceBundle".to_owned(),
        serviceInstanceName: SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}

/// Returns the HAR user preferences service bundle fqin
pub fn get_har_preferences_service_bundle() -> ServiceFqin {
    ServiceFqin {
        sdvVmName: "local-vm".to_owned(),
        sdvPackageName: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        serviceBundleName: "HarUserPreferencesServiceBundle".to_owned(),
        serviceInstanceName: SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}

/// Returns the HAR-SDV service bundle fqin
pub fn get_har_sdv_service_bundle() -> ServiceFqin {
    ServiceFqin {
        sdvVmName: "local-vm".to_owned(),
        sdvPackageName: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        serviceBundleName: "HarSdvServiceBundle".to_owned(),
        serviceInstanceName: SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}

/// Returns the default timeout for SDV service bundle tests.
/// This timeout should be long enough to have SDV services
/// start after boot.
pub fn default_sdv_test_timeout() -> Duration {
    Duration::from_secs(SDV_DEFAULT_TEST_TIMEOUT_SECONDS)
}

/// Gets lifecycle manager binder service handler
pub fn get_lifecycle_manager() -> binder::Strong<dyn ILifecycleManager> {
    let descriptor =
        <BpLifecycleManager as ILifecycleManager>::get_descriptor().to_owned() + "/default";
    let expect_str = "Unable to find {descriptor}";
    binder::wait_for_interface(&descriptor).expect(expect_str)
}

/// Asserts that a service bundle with the given service_fqin is in created state.
pub fn assert_service_is_created(service_fqin: &ServiceFqin, timeout: Duration) {
    assert_poll_condition(
        format!("Service bundle did not change state to created: {:?}", service_fqin),
        timeout,
        || {
            let result = get_lifecycle_manager().getServiceBundleState(service_fqin);
            match result {
                Ok(IServiceBundleState::CREATED) => Some(Ok::<(), ()>(())),
                _ => None,
            }
        },
    );
}

/// Asserts that a service bundle with the given service_fqin starts.
pub fn assert_service_is_started(service_fqin: &ServiceFqin, timeout: Duration) {
    assert_poll_condition(
        format!("Service bundle did not change state to started: {:?}", service_fqin),
        timeout,
        || {
            let result = get_lifecycle_manager().getServiceBundleState(service_fqin);
            match result {
                Ok(IServiceBundleState::STARTED) => Some(Ok::<(), ()>(())),
                _ => None,
            }
        },
    );
}

/// Asserts that a service bundle with the given fqin shutsdown (to destroy the service).
pub fn assert_service_is_destroyed(service_fqin: &ServiceFqin, timeout: Duration) {
    assert_poll_condition(
        format!("Service bundle did not change state to destroyed: {:?}", service_fqin),
        timeout,
        || {
            let result = get_lifecycle_manager().getServiceBundleState(service_fqin);
            // In current LM logic, after service bundle stops, it deregisters it and removes it from internal states.
            // So we need to wait until the service returns SERVICE_NOT_FOUND error.
            match result {
                Err(status)
                    if status
                        == Status::new_service_specific_error_str::<String>(
                            ResponseCode::SERVICE_NOT_FOUND.0,
                            None,
                        ) =>
                {
                    Some(Ok::<(), ()>(()))
                }
                Err(status) => {
                    warn!("Unexpected error during service status check: {:?}", status);
                    None
                }
                _ => None,
            }
        },
    );
}

/// Waits for a property value to become the given `expected_value`.
pub fn assert_property_value(property: &str, expected_value: &str, timeout: Duration) {
    assert_poll_condition(
        format!("Waiting for property: {:?}={:?} failed", property, expected_value),
        timeout,
        || {
            // Get the property value.
            if let Ok(Some(current_value)) = rustutils::system_properties::read(property) {
                if current_value == expected_value {
                    Some(Ok::<(), ()>(()))
                } else {
                    None
                }
            } else {
                None
            }
        },
    );
}

/// Polls the `condition`` for the given `duration`.
/// The condition might return None to flag it's not ready to give a result.
/// The result can be `Ok(())` or an `Err(ERR)`.
pub fn assert_poll_condition<ERR: Debug, FN: Fn() -> Option<Result<(), ERR>>>(
    message: String,
    duration: Duration,
    condition: FN,
) {
    let start = Instant::now();
    while start.elapsed() < duration {
        if let Some(result) = condition() {
            result.unwrap_or_else(|_| panic!("{}", message));
            return;
        } else {
            debug!("Not ready yet: {:?}", message);
            thread::sleep(Duration::from_millis(500));
        }
    }
    panic!("{}", message);
}
