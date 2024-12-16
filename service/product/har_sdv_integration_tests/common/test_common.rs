// Copyright 2024 Google LLC

//! Common utils for testing.

use log::debug;
use std::fmt::Debug;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const SDV_DEFAULT_TEST_TIMEOUT_SECONDS: u64 = 30;

/// Returns the default timeout for SDV service bundle tests.
/// This timeout should be long enough to have SDV services
/// start after boot.
pub fn default_sdv_test_timeout() -> Duration {
    Duration::from_secs(SDV_DEFAULT_TEST_TIMEOUT_SECONDS)
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
