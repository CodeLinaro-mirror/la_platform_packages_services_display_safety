// Copyright 2024 Google LLC

//! Utilities for HAR-SDV service bundles.

use google_sdv_identity_aidl::aidl::google::sdv::identity::ServiceFqin::ServiceFqin;
use google_sdv_lifecycle::aidl::google::sdv::lifecycle::ILifecycleManager::BpLifecycleManager;
use google_sdv_lifecycle::aidl::google::sdv::lifecycle::ILifecycleManager::ILifecycleManager;
use google_sdv_lifecycle::aidl::google::sdv::lifecycle::IServiceBundleState::IServiceBundleState;
use google_sdv_sd_common_aidl::binder;
use log::info;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const SDV_SERVICE_BUNDLE_PACKAGE: &str = "com.sdv.google.display_safety.service_bundle_apex";
const SDV_SERVICE_BUNDLE_VM: &str = "local-vm";
const SDV_SERVICE_BUNDLE_INSTANCE: &str = "instance-1";

/// Gets lifecycle manager binder service handler
pub fn get_lifecycle_manager() -> binder::Strong<dyn ILifecycleManager> {
    let descriptor =
        <BpLifecycleManager as ILifecycleManager>::get_descriptor().to_owned() + "/default";
    let expect_str = "Unable to find {descriptor}";
    binder::wait_for_interface(&descriptor).expect(expect_str)
}

/// Returns the HAR user preferences service bundle fqin
pub fn get_har_preferences_service_bundle_fqin() -> ServiceFqin {
    ServiceFqin {
        sdvVmName: SDV_SERVICE_BUNDLE_VM.to_owned(),
        sdvPackageName: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        serviceBundleName: "HarUserPreferencesServiceBundle".to_owned(),
        serviceInstanceName: SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}

/// Returns the HAR-SDV user preferences (base) service bundle fqin
pub fn get_har_sdv_user_preferences_service_bundle() -> ServiceFqin {
    ServiceFqin {
        sdvVmName: "local-vm".to_owned(),
        sdvPackageName: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        serviceBundleName: "HarSdvUserPreferencesServiceBundle".to_owned(),
        serviceInstanceName: SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}

/// Polls the `condition`` for the given `duration`.
/// The condition might return None to flag it's not ready to give a result.
/// The result can be `Ok(())` or an `Err(ERR)`.
pub fn wait_for_condition(
    message: String,
    duration: Duration,
    condition: impl Fn() -> Option<Result<(), String>>,
) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < duration {
        if let Some(result) = condition() {
            info!("Service found.");
            return result.map_err(|err| format!("Error {:?}: {:?}", message, err));
        } else {
            info!("Not ready yet: {:?}", message);
            thread::sleep(Duration::from_millis(500));
        }
    }
    info!("Service timeout: {:?}", message);
    Err(format!("Timeout: {:?}", message))
}

/// Waits for the service bundle with the given service_fqin starts.
pub fn wait_for_service_started(
    service_fqin: &ServiceFqin,
    timeout: Duration,
) -> Result<(), String> {
    wait_for_condition(
        format!("Service bundle did not change state to started: {:?}", service_fqin),
        timeout,
        || {
            let result = get_lifecycle_manager().getServiceBundleState(service_fqin);
            match result {
                Ok(IServiceBundleState::STARTED) => Some(Ok::<(), String>(())),
                _ => None,
            }
        },
    )
}
