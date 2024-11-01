// Copyright 2024 Google LLC

//! Utilities for HAR-SDV service bundles.

use google_sdv_lifecycle::aidl::google::sdv::lifecycle::ILifecycleManager::BpLifecycleManager;
use google_sdv_lifecycle::aidl::google::sdv::lifecycle::ILifecycleManager::ILifecycleManager;
use google_sdv_sd_common_aidl::aidl::google::sdv::sd_common::ServiceFqin::ServiceFqin;
use google_sdv_sd_common_aidl::binder;

pub mod async_service_bundle;

const SDV_SERVICE_BUNDLE_PACKAGE: &str = "com.sdv.google.display_safety";
const SDV_SERVICE_BUNDLE_VM: &str = "local-vm";
const DEFAULT_SDV_SERVICE_BUNDLE_INSTANCE: &str = "default";
const DRIVERUI_SERVICE: &str = "DriverUIService";
const CAMERA_SERVICE: &str = "CameraService";

/// Gets lifecycle manager binder service handler
pub fn get_lifecycle_manager() -> binder::Strong<dyn ILifecycleManager> {
    let descriptor =
        <BpLifecycleManager as ILifecycleManager>::get_descriptor().to_owned() + "/default";
    let expect_str = "Unable to find {descriptor}";
    binder::wait_for_interface(&descriptor).expect(expect_str)
}

/// Returns the HAR-SDV DrierUI proxy service fqin
pub fn get_har_sdv_driverui_service_fqin() -> ServiceFqin {
    ServiceFqin {
        vm_name: SDV_SERVICE_BUNDLE_VM.to_owned(),
        package_name: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        service_name: DRIVERUI_SERVICE.to_owned(),
        instance_name: DEFAULT_SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}

/// Returns the HAR-SDV Camera proxy service fqin
pub fn get_har_sdv_camera_service_fqin() -> ServiceFqin {
    ServiceFqin {
        vm_name: SDV_SERVICE_BUNDLE_VM.to_owned(),
        package_name: SDV_SERVICE_BUNDLE_PACKAGE.to_owned(),
        service_name: CAMERA_SERVICE.to_owned(),
        instance_name: DEFAULT_SDV_SERVICE_BUNDLE_INSTANCE.to_owned(),
    }
}
