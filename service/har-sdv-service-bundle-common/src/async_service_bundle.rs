// Copyright 2024 Google LLC

//! Async extensions to service bundles.

use async_trait::async_trait;
use log::debug;
use log::warn;
use sdv::comms::id::ServiceFqin;
use sdv::comms::ContextRef;
use sdv::lifecycle::service_bundle::ServiceBundle;
use sdv::mw::Communicate;
use sdv::mw::SdvComms;
use sdv::status::SdvStatus;
use std::marker::PhantomData;
use std::marker::Sync;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// A generic service bundle implementation that launches the
/// AsyncServiceBundleLauncher.
pub struct AsyncServiceBundle<T: AsyncServiceBundleLauncher + Sync + Send> {
    // Entry point for comm stack APIs.
    comms: Arc<dyn Communicate>,
    self_fqin: ServiceFqin,
    // Used to shutdown a runtime after on_stop request
    cancellation_token: Option<CancellationToken>,
    _async_service_bundle_phantom: PhantomData<T>,
}

/// A service bundle that runs in an async context.
#[async_trait]
pub trait AsyncServiceBundleLauncher {
    /// Called before launch.
    fn new(comms: Arc<dyn Communicate>) -> Self;

    /// Launches the bundle, which should only return once it is not expected to any more work.
    /// The cancellation token can be used to stop execution of the bundle.
    // Note: consuming self. This can be handy at implementing sync+sync for this trait,
    // because this instance will not be used by anything else.
    async fn launch(self, cancellation_token: CancellationToken) -> Result<(), SdvStatus>;
}

impl<T: AsyncServiceBundleLauncher + Sync + Send + 'static> ServiceBundle
    for AsyncServiceBundle<T>
{
    fn new(context: ContextRef) -> Self {
        let self_fqin = context.get_self_fqin();
        debug!("Creating {}.", self_fqin);
        let comms = Arc::new(SdvComms { context });
        AsyncServiceBundle {
            comms,
            self_fqin,
            cancellation_token: None,
            _async_service_bundle_phantom: Default::default(),
        }
    }

    /// Called when the service bundle is started by the system.
    fn on_start(&mut self) {
        debug!("Starting {}.", &self.self_fqin);
        // Spawn new execution thread to avoid blocking the `on_start` method.
        // New thread creates tokio runtime and launches the async service bundle implementation.
        self.spawn_execution_thread().expect("Cannot start service bundle");
    }

    /// Called when the service bundle is stopped by the system in preparation
    /// for shutdown or suspend to RAM/Disc.
    fn on_stop(&mut self) {
        debug!("Stopping {}.", &self.self_fqin);
        // Trigger cancellation token to exit Tokio runtime execution and drop it
        if let Some(cancellation_token) = self.cancellation_token.as_mut() {
            cancellation_token.cancel();
            self.cancellation_token = None;
        } else {
            warn!("Cancellation token must have been created on_start");
        }
    }
}

impl<T: AsyncServiceBundleLauncher + Sync + Send + 'static> AsyncServiceBundle<T> {
    /// Spawns the thread hosting the async runtime.
    fn spawn_execution_thread(&mut self) -> Result<(), String> {
        if self.cancellation_token.is_some() {
            warn!("Cancellation token already exists.");
        }
        let async_service_bundle = T::new(self.comms.clone());
        let cancellation_token = CancellationToken::new();
        self.cancellation_token = Some(cancellation_token.clone());
        let Ok(runtime) =
            tokio::runtime::Builder::new_current_thread().enable_time().enable_io().build()
        else {
            return Err("Failed to start the tokio runtime!".to_string());
        };
        let service_sqin = self.self_fqin.clone();
        let _ = std::thread::spawn(move || {
            // Due to async nature of MW/Comms APIs we have
            // to execute a service bundle inside a runtime (Tokio).
            runtime.block_on(async move {
                debug!("Starting {} execution loop", &service_sqin);
                // Launch the async bundle launcher.
                let res = tokio::select! {
                    res = async_service_bundle.launch(cancellation_token.clone()) => res,
                    () = cancellation_token.cancelled() => Err(sdv::status::SdvStatus::new(sdv::status::SdvStatusCode::Cancelled)),
                };
                debug!("Stopped {} execution loop with result: {res:?}.", &service_sqin);
            });
            // Runtime is dropped, stopped.
        });
        Ok(())
    }
}
