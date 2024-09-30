// Copyright 2023 Google LLC

//! Contains a fake vehicle data loop, published to the actual
//! SDV-publisher using GRPC.

use crate::demo_sequence::create_demo_sequence;
use crate::grpc::create_grpc_client;
use crate::utils::play_steps;
use log::info;
use log::warn;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tokio_util::sync::CancellationToken;

mod demo_sequence;
mod grpc;
mod utils;

/// Service bundle struct definition.
pub struct HarSdvFakeVehicleDataPublisherServiceBundle {
    _context: ContextRef,
    running_task: Arc<Mutex<Option<Cancelable>>>,
}

struct Cancelable {
    cancellation_token: Option<CancellationToken>,
}

impl Cancelable {
    pub fn cancel(&mut self) {
        if let Some(token) = self.cancellation_token.take() {
            token.cancel();
        }
    }
}

// Register the new service bundle.
sdv_lifecycle_client::register_service_bundle!(HarSdvFakeVehicleDataPublisherServiceBundle);

impl ServiceBundle for HarSdvFakeVehicleDataPublisherServiceBundle {
    /// Creates a new instance of the HarSdvFakeVehicleDataPublisherServiceBundle.
    /// Called when service bundle is created by the system.
    ///
    /// Context object is provided as a parameter that gives access to the
    /// communication stack APIs.
    fn new(_context: ContextRef) -> HarSdvFakeVehicleDataPublisherServiceBundle {
        HarSdvFakeVehicleDataPublisherServiceBundle { _context, running_task: Default::default() }
    }

    /// Called when the service bundle is started by the system.
    fn on_start(&mut self) {
        sdv_log::init_logger("fake_vehicledata_publisher_sb").unwrap();
        let mut running_task = self.running_task.lock().unwrap();
        if running_task.is_some() {
            warn!("on_start: Task already running");
            return;
        }
        let token = CancellationToken::new();
        running_task.replace(Cancelable { cancellation_token: Some(token.clone()) });
        drop(running_task);

        let cloned_token = token.clone();
        thread::spawn(move || {
            let runtime =
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

            runtime.block_on(async move {
                let mut service = create_grpc_client("127.0.0.1:7002".to_string());
                let mut complete = false;
                while !complete {
                    let sequence = create_demo_sequence();
                    let token = cloned_token.clone();

                    tokio::select! {
                        _ = token.cancelled() => {
                            complete = true;
                        }
                        _ = play_steps(&mut service, sequence) => {
                            info!("Demo sequence completed.");
                        }
                    };
                }
            });
        });
    }

    /// Called when the service bundle is stopped by the system in preparation
    /// for shutdown or suspend to RAM/Disc.
    fn on_stop(&mut self) {
        let mut running_task = self.running_task.lock().unwrap();
        if let Some(mut task) = running_task.take() {
            task.cancel();
            info!("Task canceled");
        }
        info!("Service bundle stopped.");
    }
}

/// Called when the service bundle is destroyed by the system.
impl Drop for HarSdvFakeVehicleDataPublisherServiceBundle {
    fn drop(&mut self) {
        self.on_stop();
    }
}
