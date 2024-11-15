// Copyright 2024 Google LLC

//! Utilities for HAR-SDV service bundles.

use sdv::comms::id::ServiceFqin;

pub mod async_service_bundle;

const SDV_SERVICE_BUNDLE_PACKAGE: &str = "com.sdv.google.display_safety";
const SDV_SERVICE_BUNDLE_VM: &str = "local-vm";
const DEFAULT_SDV_SERVICE_BUNDLE_INSTANCE: &str = "default";
const DRIVERUI_SERVICE: &str = "DriverUIService";
const CAMERA_SERVICE: &str = "CameraService";

/// Returns the HAR-SDV DrierUI proxy service fqin
pub fn get_har_sdv_driverui_service_fqin() -> ServiceFqin {
    ServiceFqin::builder()
        .sdv_vm_name(SDV_SERVICE_BUNDLE_VM.to_owned())
        .sdv_package_name(SDV_SERVICE_BUNDLE_PACKAGE.to_owned())
        .service_bundle_name(DRIVERUI_SERVICE.to_owned())
        .service_instance_name(DEFAULT_SDV_SERVICE_BUNDLE_INSTANCE.to_owned())
        .build()
        .expect("Invalid FQIN")
}

/// Returns the HAR-SDV Camera proxy service fqin
pub fn get_har_sdv_camera_service_fqin() -> ServiceFqin {
    ServiceFqin::builder()
        .sdv_vm_name(SDV_SERVICE_BUNDLE_VM.to_owned())
        .sdv_package_name(SDV_SERVICE_BUNDLE_PACKAGE.to_owned())
        .service_bundle_name(CAMERA_SERVICE.to_owned())
        .service_instance_name(DEFAULT_SDV_SERVICE_BUNDLE_INSTANCE.to_owned())
        .build()
        .expect("Invalid FQIN")
}
