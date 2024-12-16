// Copyright 2024 Google LLC

//! Tests the system has booted and services started.

#[cfg(test)]
mod tests {
    use har_sdv_orch_tests_common::test_common::*;

    /// Confirms the HAR binary started and set a property.
    #[test]
    fn test_harry_app_grpc_server_started() {
        // Property to set after the GRPC service was started.
        assert_property_value(
            "vendor.harplatform.grpc.started",
            "true",
            default_sdv_test_timeout(),
        );
    }
}
