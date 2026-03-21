//! Outcome tracking for AI entity self-evolution.
//!
//! Records what was attempted, what happened, and how predictions
//! compared to reality. Provides operational self-model data.

pub mod outcome;
pub mod runtime;
pub mod state;

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Caliber data directory: `{docs_dir}/caliber/`
pub fn caliber_dir(docs_dir: &Path) -> PathBuf {
    docs_dir.join("caliber")
}

/// Path to outcomes.json
pub fn outcomes_file(docs_dir: &Path) -> PathBuf {
    caliber_dir(docs_dir).join("outcomes.json")
}

/// Path to CALIBER.md
pub fn caliber_md(docs_dir: &Path) -> PathBuf {
    docs_dir.join("CALIBER.md")
}

// ---------------------------------------------------------------------------
// Core struct
// ---------------------------------------------------------------------------

/// Outcome tracker. Records and analyzes task execution results.
pub struct CaliberEcho {
    docs_dir: PathBuf,
}

impl CaliberEcho {
    pub fn new(docs_dir: PathBuf) -> Self {
        Self { docs_dir }
    }

    pub fn docs_dir(&self) -> &Path {
        &self.docs_dir
    }
}
