// Copyright 2023 Google LLC

//! HAR-SDV-service connects to SDV Comms, Data tunnel and delivers events to HAR.

use crate::camera_grpc_proxy::CameraServiceGrpcProxy;
use crate::common::CAMERA_RPC_CLIENT_ADDRESS;
use crate::common::CAMERA_RPC_SERVER_HOST;
use crate::common::CAMERA_RPC_SERVER_PORT;
use crate::common::DRIVERUI_RPC_SERVER_HOST;
use crate::common::DRIVERUI_RPC_SERVER_PORT;
use crate::common::HAR_DRIVERUI_RPC_CLIENT_ADDRESS;
use crate::common::HAR_VEHICLE_DATA_GRPC;
use crate::common::PRODUCT_HAR_SAFETY_MONITOR_IP;
use crate::common::QNX_VEHICLE_DATA_PORT;
use crate::driverui_grpc_proxy::DriverUiGrpcProxy;
use crate::driverui_grpc_proxy::DriverUiSdvRpcProxy;
use crate::integrations_v1::create_topic_map;
use crate::mapper::HashMapTopicMapper;
use crate::mapper::SdvToHarMapper;
use crate::observe::start_monitoring_all_data;
use async_trait::async_trait;
use futures::SinkExt;
use grpcio::ChannelBuilder;
use grpcio::EnvBuilder;
use grpcio::WriteFlags;
use har_grpc_services::vehicledata_grpc::VehicleDataServiceClient;
use har_sdv_service_bundle_common::async_service_bundle::AsyncServiceBundle;
use har_sdv_service_bundle_common::async_service_bundle::AsyncServiceBundleLauncher;
use log::info;
use log::trace;
use log::warn;
use oem_harry_vehicle_messages_catalog_v1::vehicledata::Gear;
use rustutils::system_properties;
use sdv::mw::Communicate;
use sdv::status::SdvStatus;
use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::HarSdvServiceBundle as SdvVehicleDataClient;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

mod camera_grpc_proxy;
mod common;
mod driverui_grpc_proxy;
mod integrations_v1;
mod mapper;
mod observe;

/// Service bundle struct definition.
pub struct HarSdvServiceBundle {
    comms: Arc<dyn Communicate>,
    driverui_rpc_proxy: DriverUiGrpcProxy,
    camera_rpc_proxy: CameraServiceGrpcProxy,
    qnx_address: Option<String>,
}

/// A message enum sent to Harry's Vehicle data server
#[derive(Debug, Clone)]
pub enum HarMessage {
    /// Tell Tale Status
    TellTaleStatus(String, bool),
    /// VehicleS peed
    VehicleSpeed(String, i32),
    /// Tire Pressure
    TirePressure(String, u32),
    /// Current Gear
    CurrentGear(String, Gear),
}

// Register the new service bundle.
sdv::lifecycle::register_service_bundle!(AsyncServiceBundle<HarSdvServiceBundle>);

#[async_trait]
impl AsyncServiceBundleLauncher for HarSdvServiceBundle {
    fn new(comms: Arc<dyn Communicate>) -> Self {
        let qnx_address = match system_properties::read(PRODUCT_HAR_SAFETY_MONITOR_IP) {
            Ok(Some(ip)) => Some(format!("{}:{}", ip, QNX_VEHICLE_DATA_PORT)),
            Ok(None) => {
                info!(
                    "QNX IP is not set by '{:?}'. QNX Proxy not started.",
                    PRODUCT_HAR_SAFETY_MONITOR_IP
                );
                // QNX Proxy won't be started.
                None
            }
            Err(e) => panic!("Could not fetch Safety Monitor IP property. Err: {:?}", e),
        };
        let driverui_rpc_proxy = DriverUiGrpcProxy::new(format!(
            "{}:{}",
            DRIVERUI_RPC_SERVER_HOST, DRIVERUI_RPC_SERVER_PORT
        ));
        let camera_rpc_proxy = CameraServiceGrpcProxy::new(format!(
            "{}:{}",
            CAMERA_RPC_SERVER_HOST, CAMERA_RPC_SERVER_PORT
        ));
        HarSdvServiceBundle { comms, driverui_rpc_proxy, camera_rpc_proxy, qnx_address }
    }

    async fn launch(self, cancellation_token: CancellationToken) -> Result<(), SdvStatus> {
        sdv_log::init_logger("har_sdv_svc_v1")
            .unwrap_or_else(|err| warn!("Error during logger initialization: {:?}", err));

        info!("Launching.");

        let ch_to_har_vehicle_data = ChannelBuilder::new(Arc::new(EnvBuilder::new().build()))
            .initial_reconnect_backoff(Duration::from_millis(10))
            .max_reconnect_backoff(Duration::from_millis(50))
            .connect(HAR_VEHICLE_DATA_GRPC);

        let ch_to_har_camera = ChannelBuilder::new(Arc::new(EnvBuilder::new().build()))
            .initial_reconnect_backoff(Duration::from_millis(10))
            .max_reconnect_backoff(Duration::from_millis(50))
            .connect(CAMERA_RPC_CLIENT_ADDRESS);

        let ch_to_har_driverui = ChannelBuilder::new(Arc::new(EnvBuilder::new().build()))
            .initial_reconnect_backoff(Duration::from_millis(10))
            .max_reconnect_backoff(Duration::from_millis(50))
            .connect(HAR_DRIVERUI_RPC_CLIENT_ADDRESS);

        // DriverUiSdvRpcProxy implements the SDV RPC server trait.
        let driverui_sdv_rpc: Arc<
            dyn com_sdv_google_display_safety_driver_ui_service_rpc::Interface,
        > = Arc::new(DriverUiSdvRpcProxy::new(ch_to_har_driverui.clone()));

        let mut subscriber_service =
            match SdvVehicleDataClient::new(self.comms.clone(), (driverui_sdv_rpc,)).await {
                Ok(service) => service,
                Err(e) => panic!("{e}"),
            };

        let (har_tx, har_rx) = mpsc::channel(32);
        // Create client to transfer vehicle data
        let har_vehicle_data_client = VehicleDataServiceClient::new(ch_to_har_vehicle_data);
        let mut har_vehicle_data_clients = vec![har_vehicle_data_client];
        // Initialize the optional QNX Vehicle Data proxy. (only available on QNX-based systems.)
        if let Some(qnx_address) = self.qnx_address.as_ref() {
            let ch = ChannelBuilder::new(Arc::new(EnvBuilder::new().build()))
                .initial_reconnect_backoff(Duration::from_millis(10))
                .max_reconnect_backoff(Duration::from_millis(50))
                .connect(qnx_address);

            let vehicle_data_client = VehicleDataServiceClient::new(ch);
            har_vehicle_data_clients.push(vehicle_data_client);
        }

        // Maps from SDV-relevant Strings to sendable HAR vehicle data messages.
        let mapper = Arc::new(SdvToHarMapper::new(create_topic_map()));

        let _sdv_vehicle_data_to_har_task = tokio::spawn(transfer_vehicle_data(
            har_vehicle_data_clients,
            har_rx,
            cancellation_token.clone(),
            mapper,
        ));

        // The timeout for each bunch of observed units.
        let lookup_timeout = Duration::from_secs(20);
        let mut subscriptions = timeout(
            lookup_timeout,
            start_monitoring_all_data(&mut subscriber_service, har_tx, cancellation_token.clone()),
        )
        .await
        .map_err(|err| {
            info!("Starting to monitor variants failed. {:?}", err);
            Err(sdv::status::SdvStatus::new(sdv::status::SdvStatusCode::Cancelled))
        })??;
        info!("Monitoring vehicle data started");

        // Start other RPC services
        let mut driverui_rpc_proxy_token =
            self.driverui_rpc_proxy.run(Arc::new(EnvBuilder::new().build()), ch_to_har_driverui);
        let camera_rpc_proxy_token =
            self.camera_rpc_proxy.run(Arc::new(EnvBuilder::new().build()), ch_to_har_camera).ok();

        info!("HAR-SDV Service started.");

        while let Some(res) = subscriptions.join_next().await {
            if let Err(err) = res {
                // One of the futures has completed with an error which indicates an abnormal behavior.
                // Initiating a resubscription could be performed at such a case.
                panic!("{err:?}");
            } else {
                info!("Task completed: {:?}", res);
            }
        }
        info!("All observers completed.");
        driverui_rpc_proxy_token.shutdown();
        if let Some(mut camera_rpc_proxy_token) = camera_rpc_proxy_token {
            camera_rpc_proxy_token.shutdown();
        }
        info!("RPC Services stopped.");
        Ok(())
    }
}

async fn transfer_vehicle_data(
    vehicle_data_clients: Vec<VehicleDataServiceClient>,
    mut har_rx: mpsc::Receiver<HarMessage>,
    cancellation_token: CancellationToken,
    mapper: Arc<SdvToHarMapper<HashMapTopicMapper>>,
) {
    let mut clients = Vec::new();
    // Create the non-async GRPC clients.
    for vehicle_data_client in vehicle_data_clients {
        // Need to keep GRPC service references open, otherwise they will be canceled.
        let client = tokio::task::spawn_blocking(move || {
            match vehicle_data_client.receive_vehicle_data() {
                Ok((vehicle_data_sender, vehicle_data_receiver)) => {
                    info!("Created GRPC channel to HAR");
                    (vehicle_data_sender, vehicle_data_receiver, vehicle_data_client)
                }
                Err(err) => {
                    // TODO(378910627): Avoid panic or implement crash handling for these services.
                    panic!("Failed to call Vehicle data api. Err: {:?}", err);
                }
            }
        })
        .await
        .expect("Error starting GRPC to HAR.");
        clients.push(client);
    }

    info!("Waiting for messages to send to HAR");
    loop {
        tokio::select! {
            message = har_rx.recv() => {
                if let Some(message) = message {
                    let vehicle_data_message = match message {
                        HarMessage::TellTaleStatus(key, value) => {
                            mapper.map_bool(key, value)
                        },
                        HarMessage::VehicleSpeed(key, value) => {
                            mapper.map_i32(key, value)
                        },
                        HarMessage::TirePressure(key, value) => {
                            mapper.map_u32(key, value)
                        },
                        HarMessage::CurrentGear(key, value) => {
                            mapper.map_string(key, match value {
                                Gear::P => "P",
                                Gear::R => "R",
                                Gear::N => "N",
                                Gear::D => "D",
                            }.to_string())
                        },
                    };
                    trace!("Sending: {:?}", vehicle_data_message);
                    for client in &mut clients {
                        let (ref mut sender, _receiver, _vehicle_data_client) = client;
                        if let Err(err) = sender.send((vehicle_data_message.clone(), WriteFlags::default())).await {
                            warn!("Error sending message to HAR: {:?}", err);
                        }
                    }
                } else {
                    warn!("GRPC Message channel closed.");
                    return;
                }
            },
            () = cancellation_token.cancelled() => {
                info!("GRPC to HAR canceled.");
                return;
            },
        };
    }
    // Clients and other references are dropped here, when returning.
}
