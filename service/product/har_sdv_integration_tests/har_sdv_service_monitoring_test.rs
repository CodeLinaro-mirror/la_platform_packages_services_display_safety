// Copyright 2024 Google LLC

//! Tests that verify the units monitoring can start.

#[cfg(test)]
mod tests {
    use oem_harry_vehicle_messages_catalog_v1::vehicle_tire::TirePressure;
    use oem_harry_vehicle_messages_catalog_v1::vehicledata::CurrentGear;
    use oem_harry_vehicle_messages_catalog_v1::vehicledata::TellTaleStatus;
    use oem_harry_vehicle_messages_catalog_v1::vehicledata::VehicleSpeed;
    use protobuf::Message;
    use sdv::mw::SdvComms;
    use sdv::mw::SubscribeOptions;
    use sdv_comms::id::ServiceFqin;
    use sdv_comms::ContextRef;
    use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::subscriber::Metadata;
    use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::subscriber::Variant;
    use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::HarSdvServiceBundle as SdvVehicleDataClient;
    use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::UnitType;
    use sdv_status_rs::SdvStatus;
    use std::fmt::Debug;
    use std::sync::Arc;
    use tokio::time::timeout;
    use tokio::time::Duration;

    #[tokio::test]
    pub async fn test_vehicle_data_publisher_service_bundle_running() {
        let fqin = ServiceFqin::builder()
            .sdv_vm_name("local-vm")
            .sdv_package_name("com.sdv.google_display_safety")
            .service_bundle_name("MonitoringTest")
            .service_instance_name("default")
            .build()
            .expect("Cannot create FQIN for test.");

        let context = ContextRef::create(fqin);
        let comms = Arc::new(SdvComms { context });
        let mut subscriber_service = match SdvVehicleDataClient::new(comms).await {
            Ok(service) => service,
            Err(e) => panic!("Error connecting to SDV: {e:?}"),
        };

        // call retry_monitor_all with a timeout
        let lookup_timeout = Duration::from_secs(20);
        timeout(lookup_timeout, retry_monitor_all(&mut subscriber_service))
            .await
            .expect("Failed to monitor SDV data.");
    }

    async fn retry_monitor_all(subscriber_service: &mut SdvVehicleDataClient) {
        while try_monitor_all(subscriber_service).await.is_err() {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    }

    async fn try_monitor_all(
        subscriber_service: &mut SdvVehicleDataClient,
    ) -> Result<(), SdvStatus> {
        try_start_monitoring(subscriber_service, Variant::<CurrentGear>::ALL_VARIANTS).await?;
        try_start_monitoring(subscriber_service, Variant::<TellTaleStatus>::ALL_VARIANTS).await?;
        try_start_monitoring(subscriber_service, Variant::<TirePressure>::ALL_VARIANTS).await?;
        try_start_monitoring(subscriber_service, Variant::<VehicleSpeed>::ALL_VARIANTS).await?;
        Ok(())
    }

    async fn try_start_monitoring<MessageType: Message + Debug + UnitType + Metadata>(
        subscriber_service: &mut SdvVehicleDataClient,
        variants: &[&'static Variant<MessageType>],
    ) -> Result<(), SdvStatus> {
        let options = SubscribeOptions::default();
        for variant in variants {
            subscriber_service.create_observer(variant, &options).await.map(|_| ())?
        }
        Ok(())
    }
}
