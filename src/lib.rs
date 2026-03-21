//! vigil-pulse — Metacognitive monitoring for AI entities.
//!
//! Unified self-monitoring system that combines three signal categories:
//!
//! - **Pipeline signals** — document flow enforcement, thresholds, staleness detection
//! - **Reflection signals** — vocabulary diversity, question generation, thought lifecycle
//! - **Outcome signals** — task effectiveness, prediction accuracy, domain performance
//!
//! Each category can be used independently or together for a unified health assessment.

pub mod pipeline;
pub mod reflection;
pub mod outcomes;

// Re-export core types for convenience
pub use pipeline::{PraxisConfig, PraxisEcho};
pub use reflection::VigilEcho;
pub use outcomes::CaliberEcho;
