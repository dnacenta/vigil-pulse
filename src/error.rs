//! Unified error type for vigil-pulse.

use thiserror::Error;

/// Unified error type used across pipeline, reflection, and outcome modules.
#[derive(Debug, Error)]
pub enum VpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error("Reflection error: {0}")]
    Reflection(String),

    #[error("Outcome error: {0}")]
    Outcome(String),
}

/// Convenience alias for functions returning `VpError`.
pub type VpResult<T> = Result<T, VpError>;
