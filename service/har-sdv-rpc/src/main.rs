// Copyright 2024 Google LLC

//! Sample application to demonstrate the usage of SDV Service Discovery.

// Implementation is based on:
// system/software_defined_vehicle/core_services/service_discovery/sdv_sd_agent/tests/add_service_flow_test.rs

use crate::sdv_service_discovery::find_service;
use crate::sdv_service_discovery::register_service;
use anyhow::Result;
use log::info;
use sdv::comms::id::ServiceFqin;

mod sdv_service_discovery;

fn main() -> Result<()> {
    let publickey = *b"HARSDVGATEWAY-7890123456_______\0";

    let fqin = ServiceFqin::builder()
        .sdv_vm_name("".to_owned())
        .sdv_package_name("com.sdv.android.car.displaysafety".to_owned())
        .service_bundle_name("DriverUIServiceFoo".to_owned())
        .service_instance_name("default".to_owned())
        .build()
        .expect("Invalid FQIN");

    let listening_port = 11224;
    register_service(publickey, &fqin, "foo-bar-custom-data".as_bytes().to_vec(), listening_port)?;
    info!("Service registered.");

    info!("Now trying to retrieve the registration");
    let metadata = find_service(&fqin);
    info!("Got service address that was registered earlier: {:?}", metadata);
    Ok(())
}
