// Copyright 2024 Google LLC

//! Implements SDV Service discovery registration and lookup.

// Implementation is based on:
// system/software_defined_vehicle/core_services/service_discovery/sdv_sd_agent/tests/add_service_flow_test.rs

use anyhow::{anyhow, Result};
use google_sdv_rpc_aidl::aidl::google::sdv::rpc::IRpcAgent::{BpRpcAgent, IRpcAgent};
use google_sdv_sd_aidl::aidl::google::sdv::sd::IIdentityManager::{
    BpIdentityManager, IIdentityManager,
};
use google_sdv_sd_aidl::aidl::google::sdv::sd::IServiceManager::{
    BpServiceManager, IServiceManager,
};
use google_sdv_sd_aidl::aidl::google::sdv::sd::ServiceDefinition::TypedMetadata::TypedMetadata as ServiceTypedMetadata;
use google_sdv_sd_common_aidl::aidl::google::sdv::sd_common::GenericMetadata::GenericMetadata;
use google_sdv_sd_common_aidl::aidl::google::sdv::sd_common::ServiceFqin::ServiceFqin as InternalServiceFqin;
use google_sdv_sd_common_aidl::aidl::google::sdv::sd_common::ServiceIdentity::PublicKey::PublicKey;
use log::{debug, trace};
use sdv::comms::id::ServiceFqin;

/// Size in bytes of the X25519 Service Public Key.
pub const SERVICE_PUBLIC_KEY_SIZE_BYTES: usize = 32;

fn get_service_manager() -> Result<binder::Strong<dyn IServiceManager>> {
    let descriptor =
        <BpServiceManager as IServiceManager>::get_descriptor().to_owned() + "/default";
    Ok(binder::get_interface(&descriptor)?)
}

fn get_identity_manager() -> Result<binder::Strong<dyn IIdentityManager>> {
    let descriptor =
        <BpIdentityManager as IIdentityManager>::get_descriptor().to_owned() + "/default";
    Ok(binder::get_interface(&descriptor)?)
}

fn get_rpc_agent() -> Result<binder::Strong<dyn IRpcAgent>> {
    let descriptor = <BpRpcAgent as IRpcAgent>::get_descriptor().to_owned() + "/default";
    Ok(binder::get_interface(&descriptor)?)
}

/// Registers a service with SDV Service Discovery and SDV RPC as an RPC server
/// listening for client connections on a listening port
/// * `publickey`: The public key to pass to SDV SD.
/// * `fqin`: The service descriptor.
/// * `custom_data`: Any custom metadata that can be retrieved from the server.
/// * `listening_port`: The listening port to register for the service.
pub fn register_service(
    publickey: [u8; SERVICE_PUBLIC_KEY_SIZE_BYTES],
    fqin: &ServiceFqin,
    custom_data: Vec<u8>,
    listening_port: i32,
) -> Result<()> {
    let identity = get_identity_manager()?;
    // Calling directly into the SDV Comms private APIs to register an
    // arbitrary FQIN and port. This needs conversion.
    let fqin = InternalServiceFqin {
        vm_name: fqin.get_sdv_vm_name().to_string(),
        package_name: fqin.get_sdv_package_name().to_string(),
        service_name: fqin.get_service_bundle_name().to_string(),
        instance_name: fqin.get_service_instance_name().to_string(),
    };
    let _service_identity = identity.createIdentity(&PublicKey { value: publickey }, &fqin)?;

    let service = get_service_manager()?;
    let metadata = ServiceTypedMetadata { version: 1 };
    let app_metadata = GenericMetadata { value_holder: custom_data };

    // Register service in app
    let token = service.registerService(&metadata, &app_metadata)?;
    debug!("Registered service. Token: {:?}", token);

    let rpc_agent = get_rpc_agent()?;
    rpc_agent.registerServicePort(&token, listening_port)?;

    Ok(())
}

/// Tries to find a service that is an RPC server using SDV Service Discovery and SDV RPC.
/// * `fqin`: The service descriptor.
/// * returns a connection string using the "host:port" format.
pub fn find_service(fqin: &ServiceFqin) -> Result<String> {
    // Calling directly into the SDV Comms private APIs to register an
    // arbitrary FQIN and port. This needs conversion.
    let fqin = InternalServiceFqin {
        vm_name: fqin.get_sdv_vm_name().to_string(),
        package_name: fqin.get_sdv_package_name().to_string(),
        service_name: fqin.get_service_bundle_name().to_string(),
        instance_name: fqin.get_service_instance_name().to_string(),
    };
    let service_manager = get_service_manager()?;
    let vm_name: Option<&str> =
        if fqin.vm_name.is_empty() { None } else { Some(fqin.vm_name.as_str()) };
    let instance_name =
        if fqin.instance_name.is_empty() { None } else { Some(fqin.instance_name.as_str()) };
    let package_name = fqin.package_name.as_str();
    let service_name = fqin.service_name.as_str();

    debug!("Looking up service for vm: {vm_name:?}, package: {package_name}, service: {service_name}, instance: {instance_name:?}");
    let found_services =
        service_manager.listServices(vm_name, package_name, service_name, instance_name)?;

    trace!("Service list result: {:?}", found_services);
    // We are just taking the first found service for simplicity
    let service_definition = found_services.into_iter().next().ok_or_else(|| {
        anyhow!("Service not found. vm: {vm_name:?}, package: {package_name}, service: {service_name}, instance: {instance_name:?}")
    })?;

    let rpc_agent = get_rpc_agent()?;
    Ok(rpc_agent.getServerAddress(&service_definition.identity)?)
}
