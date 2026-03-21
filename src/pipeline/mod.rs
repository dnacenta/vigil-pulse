//! Pipeline enforcement engine for AI entity self-evolution.
//!
//! Tracks document pipeline health (LEARNING -> THOUGHTS -> REFLECTIONS -> SELF/PRAXIS),
//! enforces thresholds, detects stale items, and provides session-level diffs.

pub mod archive;
pub mod calibrate;
pub mod checkpoint;
pub mod init;
pub mod nudge;
pub mod parser;
pub mod pulse;
pub mod review;
pub mod runtime;
pub mod scan;
pub mod state;
pub mod status;

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

pub fn praxis_dir(claude_dir: &PathBuf) -> PathBuf {
    claude_dir.join("praxis")
}

pub fn state_file(claude_dir: &PathBuf) -> PathBuf {
    praxis_dir(claude_dir).join("state.json")
}

pub fn checkpoints_dir(claude_dir: &PathBuf) -> PathBuf {
    praxis_dir(claude_dir).join("checkpoints")
}

pub fn settings_file(claude_dir: &PathBuf) -> PathBuf {
    claude_dir.join("settings.json")
}

pub fn rules_dir(claude_dir: &PathBuf) -> PathBuf {
    claude_dir.join("rules")
}

// Document paths
pub fn learning_file(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("LEARNING.md")
}

pub fn thoughts_file(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("THOUGHTS.md")
}

pub fn curiosity_file(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("CURIOSITY.md")
}

pub fn reflections_file(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("REFLECTIONS.md")
}

pub fn praxis_file(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("PRAXIS.md")
}

pub fn self_file(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("SELF.md")
}

pub fn session_log_file(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("SESSION-LOG.md")
}

// Archive directories
pub fn archives_dir(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("archives")
}

// Intent queue
pub fn intent_queue_file(docs_dir: &PathBuf) -> PathBuf {
    docs_dir.join("intent-queue.json")
}

// ---------------------------------------------------------------------------
// PraxisConfig
// ---------------------------------------------------------------------------

/// Configuration for pipeline enforcement.
#[derive(Debug, Clone)]
pub struct PraxisConfig {
    pub claude_dir: PathBuf,
    pub docs_dir: PathBuf,
    pub thoughts_staleness_days: u32,
    pub curiosity_staleness_days: u32,
    pub freeze_threshold: u32,
    pub pulse_cooldown_secs: u64,
}

impl Default for PraxisConfig {
    fn default() -> Self {
        Self {
            claude_dir: PathBuf::from("."),
            docs_dir: PathBuf::from("."),
            thoughts_staleness_days: 7,
            curiosity_staleness_days: 14,
            freeze_threshold: 3,
            pulse_cooldown_secs: 60,
        }
    }
}

// ---------------------------------------------------------------------------
// PraxisEcho (core struct)
// ---------------------------------------------------------------------------

/// The pipeline enforcement engine. Manages document pipeline health.
pub struct PraxisEcho {
    config: PraxisConfig,
}

impl PraxisEcho {
    pub fn new(config: PraxisConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &PraxisConfig {
        &self.config
    }

    pub fn claude_dir(&self) -> &PathBuf {
        &self.config.claude_dir
    }

    pub fn docs_dir(&self) -> &PathBuf {
        &self.config.docs_dir
    }
}
