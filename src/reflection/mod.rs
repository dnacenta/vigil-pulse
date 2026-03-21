//! Metacognitive monitoring for AI entity self-evolution.
//!
//! Tracks cognitive health signals (vocabulary diversity, question generation,
//! thought lifecycle, evidence grounding), analyzes trends over a rolling window,
//! and surfaces alerts when reflective output becomes mechanical.

pub mod analyze;
pub mod collect;
pub mod init;
pub mod parser;
pub mod pulse;
pub mod runtime;
pub mod signals;
pub mod state;
pub mod stats;
pub mod status;

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Base Claude directory (~/.claude or VIGIL_ECHO_HOME override).
pub fn claude_dir() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("VIGIL_ECHO_HOME") {
        return Ok(PathBuf::from(p));
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "Could not determine home directory".to_string())?;
    Ok(home.join(".claude"))
}

/// Home directory for documents (~/ or VIGIL_ECHO_DOCS override).
pub fn docs_dir() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("VIGIL_ECHO_DOCS") {
        return Ok(PathBuf::from(p));
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "Could not determine home directory".to_string())
}

pub fn vigil_dir() -> Result<PathBuf, String> {
    Ok(claude_dir()?.join("vigil"))
}

pub fn signals_file() -> Result<PathBuf, String> {
    Ok(vigil_dir()?.join("signals.json"))
}

pub fn analysis_file() -> Result<PathBuf, String> {
    Ok(vigil_dir()?.join("analysis.json"))
}

pub fn config_file() -> Result<PathBuf, String> {
    Ok(vigil_dir()?.join("config.json"))
}

pub fn settings_file() -> Result<PathBuf, String> {
    Ok(claude_dir()?.join("settings.json"))
}

pub fn rules_dir() -> Result<PathBuf, String> {
    Ok(claude_dir()?.join("rules"))
}

// Document paths
pub fn reflections_file() -> Result<PathBuf, String> {
    Ok(docs_dir()?.join("REFLECTIONS.md"))
}

pub fn thoughts_file() -> Result<PathBuf, String> {
    Ok(docs_dir()?.join("THOUGHTS.md"))
}

pub fn curiosity_file() -> Result<PathBuf, String> {
    Ok(docs_dir()?.join("CURIOSITY.md"))
}

#[allow(dead_code)] // Phase 2: position_delta signal
pub fn self_file() -> Result<PathBuf, String> {
    Ok(docs_dir()?.join("SELF.md"))
}

// ---------------------------------------------------------------------------
// VigilEcho — core struct
// ---------------------------------------------------------------------------

/// The metacognitive monitor. Tracks reflection quality over time.
pub struct VigilEcho {
    claude_dir: PathBuf,
    docs_dir: PathBuf,
}

impl VigilEcho {
    pub fn new(claude_dir: PathBuf, docs_dir: PathBuf) -> Self {
        Self {
            claude_dir,
            docs_dir,
        }
    }

    pub fn from_default() -> Result<Self, String> {
        Ok(Self::new(self::claude_dir()?, self::docs_dir()?))
    }

    pub fn claude_dir(&self) -> &PathBuf {
        &self.claude_dir
    }

    pub fn docs_dir(&self) -> &PathBuf {
        &self.docs_dir
    }
}
