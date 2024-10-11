// Copyright 2024 Google LLC
use std::thread;
use std::time::Duration;

/// Wait for the required service bundles to start up.
pub fn wait_for_sdv_services_ready(timeout: Duration) -> Result<(), String> {
    // This implementation is only meant to be used for the obsolete
    // HAR-SDV integration that is not using Service Bundles.
    // Service bundle status can be queried, but the APIs area only
    // available on the product partition.
    thread::sleep(timeout);
    Ok(())
}
