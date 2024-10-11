// Copyright 2024 Google LLC
use std::thread;
use std::time::Duration;

/// Wait for the required service bundles to start up.
pub fn wait_for_sdv_services_ready(_timeout: Duration) -> Result<(), String> {
    // Sleeping 5 seconds as a workaronud.
    // TODO(369515367): Remove this workaround and wait for the service using Lifecycle API calls,
    // or use a better solution.
    thread::sleep(Duration::from_secs(5));
    Ok(())
}
