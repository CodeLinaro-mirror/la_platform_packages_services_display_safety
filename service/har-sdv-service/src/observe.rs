// Copyright 2024 Google LLC

use crate::HarMessage;
use futures::StreamExt;
use log::debug;
use log::info;
use log::warn;
use oem_harry_vehicle_messages_catalog_v1::vehicle_tire::TirePressure;
use oem_harry_vehicle_messages_catalog_v1::vehicledata::CurrentGear;
use oem_harry_vehicle_messages_catalog_v1::vehicledata::TellTaleStatus;
use oem_harry_vehicle_messages_catalog_v1::vehicledata::VehicleSpeed;
use protobuf::Message;
use sdv::status::SdvResult;
use sdv::status::SdvStatus;
use sdv::status::SdvStatusCode;
use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::subscriber::Metadata;
use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::subscriber::Variant;
use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::HarSdvServiceBundle as SdvVehicleDataClient;
use sdv_mw_rs_com_sdv_google_display_safety_har_sdv_service_bundle::UnitType;
use std::fmt::Debug;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

/// Starts monitoring the given unit variants and sends the message after conversion.
/// - `subscriber_service`: The SDV Client to use.
/// - `har_tx`: The channel to send data to.
/// - `cancellation_token`: A token that can be stored and used by tasks.
pub async fn start_monitoring_all_data(
    subscriber_service: &mut SdvVehicleDataClient,
    har_tx: mpsc::Sender<HarMessage>,
    cancellation_token: CancellationToken,
) -> Result<JoinSet<()>, SdvStatus> {
    let mut subscriptions = JoinSet::new();
    let retry_delay = Duration::from_millis(200);

    // Start monitoring gear and convert them to HarMessage
    start_monitoring_units(
        retry_delay,
        subscriber_service,
        Variant::<CurrentGear>::ALL_VARIANTS,
        &mut subscriptions,
        &|in_message: CurrentGear, variant| {
            HarMessage::CurrentGear(gear_to_string(variant), in_message.gear.unwrap())
        },
        har_tx.clone(),
        cancellation_token.clone(),
    )
    .await?;

    // Start monitoring tell tales and convert them to HarMessage
    start_monitoring_units(
        retry_delay,
        subscriber_service,
        Variant::<TellTaleStatus>::ALL_VARIANTS,
        &mut subscriptions,
        &|in_message: TellTaleStatus, variant| {
            HarMessage::TellTaleStatus(telltale_status_to_string(variant), in_message.alert)
        },
        har_tx.clone(),
        cancellation_token.clone(),
    )
    .await?;

    // Start monitoring tire pressure and convert them to HarMessage
    start_monitoring_units(
        retry_delay,
        subscriber_service,
        Variant::<TirePressure>::ALL_VARIANTS,
        &mut subscriptions,
        &|in_message: TirePressure, variant| {
            HarMessage::TirePressure(tire_pressure_to_string(variant), in_message.pressure)
        },
        har_tx.clone(),
        cancellation_token.clone(),
    )
    .await?;

    // Start monitoring vehicle speed and convert them to HarMessage
    start_monitoring_units(
        retry_delay,
        subscriber_service,
        Variant::<VehicleSpeed>::ALL_VARIANTS,
        &mut subscriptions,
        &|in_message: VehicleSpeed, variant| {
            HarMessage::VehicleSpeed(vehicle_speed_to_string(variant), in_message.speed as _)
        },
        har_tx.clone(),
        cancellation_token.clone(),
    )
    .await?;

    Ok(subscriptions)
}

/// Starts monitoring the given unit variants and sends the message after conversion.
/// - `retry_delay`: The duration to wait between retries.
/// - `monitor`: The SDV Client to use.
/// - `variants`: The Unit Variants to monitor.
/// - `subscriptions`: The `JoinSet`` to run the tasks.
/// - `message_converter`: Converts from SDV Data to the sender type
/// - `message_sender`: The sender for messages.
/// - `cancellation_token`: A token that can be stored and used by tasks.
pub async fn start_monitoring_units<
    MessageType: Message + Debug + UnitType + Metadata,
    OutMessageType: Debug + Send + 'static,
>(
    retry_delay: Duration,
    monitor: &mut SdvVehicleDataClient,
    variants: &'static [&Variant<MessageType>],
    subscriptions: &mut JoinSet<()>,
    message_converter: &'static (impl Fn(MessageType, &Variant<MessageType>) -> OutMessageType
                  + Send
                  + Sync),
    message_sender: Sender<OutMessageType>,
    cancellation_token: CancellationToken,
) -> Result<(), SdvStatus> {
    for variant in variants {
        info!("Starting to monitor variant: {:?}", variant);
        start_monitoring_unit(
            monitor,
            variant,
            subscriptions,
            retry_delay,
            message_converter,
            message_sender.clone(),
            cancellation_token.clone(),
        )
        .await?;
    }
    Ok(())
}

/// Starts monitoring the given unit and sends the message after conversion.
/// - `monitor`: The SDV Client to use.
/// - `variant`: The Unit Variant to monitor.
/// - `subscriptions`: The `JoinSet`` to run the tasks.
/// - `retry_delay`: The duration to wait between retries.
/// - `message_converter`: Converts from SDV Data to the sender type
/// - `message_sender`: The sender for messages.
/// - `cancellation_token`: A token that can be stored and used by tasks.
async fn start_monitoring_unit<
    MessageType: Message + Debug + UnitType + Metadata,
    OutMessageType: Debug + Send + 'static,
>(
    monitor: &mut SdvVehicleDataClient,
    variant: &'static Variant<MessageType>,
    subscriptions: &mut JoinSet<()>,
    retry_delay: Duration,
    message_converter: &'static (impl Fn(MessageType, &Variant<MessageType>) -> OutMessageType
                  + Send
                  + Sync),
    message_sender: Sender<OutMessageType>,
    cancellation_token: CancellationToken,
) -> Result<(), SdvStatus> {
    let options = sdv::mw::SubscribeOptions::default();
    loop {
        let observer = monitor.create_observer(variant, &options).await;
        if observer.is_ok() {
            subscriptions.spawn(monitor_observer(
                observer?,
                message_converter,
                variant,
                message_sender.clone(),
                cancellation_token.clone(),
            ));
            return Ok(());
        } else {
            warn!("Failed to start observer for {:?}", variant);
            sleep(retry_delay).await;
        }
    }
}

async fn monitor_observer<MessageType: Message + Debug, OutMessageType: Debug>(
    mut subscription: Pin<
        Box<dyn futures::stream::Stream<Item = SdvResult<Vec<MessageType>>> + Send>,
    >,
    message_converter: &'static (impl Fn(MessageType, &Variant<MessageType>) -> OutMessageType
                  + Send
                  + Sync),
    variant: &'static Variant<MessageType>,
    message_sender: Sender<OutMessageType>,
    cancellation_token: CancellationToken,
) {
    info!("Observing changes for {:?}", std::any::type_name::<MessageType>());
    loop {
        tokio::select! {
            messages = subscription.next() => {
                if let Some(Ok(messages)) = messages {
                    debug!("Received message list: {messages:?}");
                    for message in messages {
                        if let Err(_err) = message_sender.send(message_converter(message, variant)).await {
                            warn!("Error sending message to GRPC Proxy. Bailing.");
                            return;
                        }
                    }
                } else {
                    warn!("Received message list: {messages:?}. Bailing.");
                    return;
                }
            },
            () = cancellation_token.cancelled() => {
                info!("Monitoring stopped: {:?}", SdvStatus::new(SdvStatusCode::Cancelled));
                return;
            },
        };
    }
}

fn vehicle_speed_to_string(variant: &Variant<VehicleSpeed>) -> String {
    format!(
        "VehicleSpeed.{}",
        match variant.id {
            id if id == Variant::VEHICLE_SPEED.id => "VEHICLE_SPEED",
            id if id == Variant::MAX_SPEED.id => "MAX_SPEED",
            id if id == Variant::SPEEDLIMIT.id => "SPEEDLIMIT",
            // This should only happen when new units were added.
            _other => panic!("Unexpected variant: {:?}", variant),
        }
    )
}

fn telltale_status_to_string(variant: &Variant<TellTaleStatus>) -> String {
    format!(
        "TellTaleStatus.{}",
        match variant.id {
            id if id == Variant::OIL_PRESSURE.id => "OIL_PRESSURE",
            id if id == Variant::ENGINE_TEMP.id => "ENGINE_TEMP",
            id if id == Variant::CHECK_ENGINE.id => "CHECK_ENGINE",
            id if id == Variant::CHARGING_FAILURE.id => "CHARGING_FAILURE",
            id if id == Variant::SEATBELT_DRIVER.id => "SEATBELT_DRIVER",
            id if id == Variant::SEATBELT_PASSENGER.id => "SEATBELT_PASSENGER",
            id if id == Variant::LOW_TIRE_PRESSURE.id => "LOW_TIRE_PRESSURE",
            id if id == Variant::AIRBAG.id => "AIRBAG",
            id if id == Variant::ABS.id => "ABS",
            id if id == Variant::BRAKE.id => "BRAKE",
            id if id == Variant::TRACTION.id => "TRACTION",
            id if id == Variant::FOG_LIGHTS.id => "FOG_LIGHTS",
            id if id == Variant::PARK_LIGHTS.id => "PARK_LIGHTS",
            id if id == Variant::HIBEAM.id => "HIBEAM",
            id if id == Variant::LOWBEAM.id => "LOWBEAM",
            id if id == Variant::TURN_SIGNAL_LEFT.id => "TURN_SIGNAL_LEFT",
            id if id == Variant::TURN_SIGNAL_RIGHT.id => "TURN_SIGNAL_RIGHT",
            id if id == Variant::ADAS.id => "ADAS",
            id if id == Variant::MAX_SPEED_DISPLAYED.id => "MAX_SPEED_DISPLAYED",
            id if id == Variant::SPEED_LIMIT_DISPLAYED.id => "SPEED_LIMIT_DISPLAYED",
            id if id == Variant::EMERGENCY_LIGHT.id => "EMERGENCY_LIGHT",
            // This should only happen when new units were added.
            _other => panic!("Unexpected variant: {:?}", variant),
        }
    )
}

fn tire_pressure_to_string(variant: &Variant<TirePressure>) -> String {
    format!(
        "TirePressure.{}",
        match variant.id {
            id if id == Variant::FRONT_LEFT.id => "FRONT_LEFT",
            id if id == Variant::FRONT_RIGHT.id => "FRONT_RIGHT",
            id if id == Variant::REAR_LEFT.id => "REAR_LEFT",
            id if id == Variant::REAR_RIGHT.id => "REAR_RIGHT",
            id if id == Variant::FIFTH_WHEEL.id => "FIFTH_WHEEL",
            // This should only happen when new units were added.
            _other => panic!("Unexpected variant: {:?}", variant),
        }
    )
}

fn gear_to_string(variant: &Variant<CurrentGear>) -> String {
    if variant.id == Variant::UNIQUE.id {
        "CurrentGear".to_string()
    } else {
        // This should only happen when new units were added.
        panic!("Unexpected variant: {:?}", variant);
    }
}
