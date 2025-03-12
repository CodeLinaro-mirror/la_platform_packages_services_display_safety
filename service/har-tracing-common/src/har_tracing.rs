use log::warn;
use sdv_tracing::try_init_tracing;
use std::fmt;

/// Error Types for HAR Tracing.
#[derive(Debug)]
pub enum HarTracingError {
    /// Error during tracing initialization.
    InitError(String),
}

impl fmt::Display for HarTracingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarTracingError::InitError(err) => write!(f, "Tracing initialization failed: {}", err),
        }
    }
}

/// HAR Tracing utilities.
pub trait HarTracing {
    /// Initialize SDV Tracing
    fn init_har_tracing(&self) -> Result<(), HarTracingError> {
        if let Err(err) = try_init_tracing() {
            let error_message = format!("Failed to init tracing: {}", err);
            warn!("{}", error_message);
            Err(HarTracingError::InitError(error_message))?;
        }
        Ok(())
    }
}
