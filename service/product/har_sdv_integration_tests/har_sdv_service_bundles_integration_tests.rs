// Copyright 2024 Google LLC

//! Tests that verify the required service bundles were started.

#[cfg(test)]
mod tests {
    use har_sdv_orch_tests_common::test_common::*;

    #[test]
    fn test_vehicle_data_publisher_service_bundle_running() {
        // vehicle_data_publisher_service_bundle must be running after boot.
        assert_service_is_started(
            &get_vehicle_data_publisher_service_bundle(),
            default_sdv_test_timeout(),
        );
    }

    #[test]
    fn test_fake_vehicle_data_publisher_service_bundle_running() {
        // fake_vehicle_data_publisher_service_bundle must be running after boot.
        assert_service_is_started(
            &get_fake_vehicle_data_publisher_service_bundle(),
            default_sdv_test_timeout(),
        );
    }

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
