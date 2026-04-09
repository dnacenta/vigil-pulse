use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::VpResult;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct State {
    pub version: u32,
    pub last_pulse: Option<String>,
    pub last_review: Option<String>,
    pub last_archive: Option<String>,
    #[serde(default)]
    pub session_start_snapshot: Option<Snapshot>,
    #[serde(default)]
    pub pipeline: PipelineState,
    #[serde(default)]
    pub staleness: StalenessState,
    #[serde(default)]
    pub session_history: Vec<SessionRecord>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Snapshot {
    pub learning_threads: usize,
    pub active_thoughts: usize,
    pub open_questions: usize,
    pub observation_count: usize,
    pub active_policies: usize,
    pub session_log_entries: usize,
    #[serde(default)]
    pub document_hashes: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PipelineState {
    pub last_movement: Option<String>,
    pub frozen_session_count: u32,
    pub total_graduations: u32,
    pub total_dissolutions: u32,
    pub total_archival_ops: u32,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct StalenessState {
    #[serde(default)]
    pub thoughts: HashMap<String, String>,
    #[serde(default)]
    pub questions: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionRecord {
    pub date: String,
    pub learning_delta: i32,
    pub thoughts_touched: i32,
    pub graduations: i32,
    pub dissolutions: i32,
    pub reflections_added: i32,
    pub pipeline_active: bool,
}

pub fn load(config: &super::PraxisConfig) -> VpResult<State> {
    let path = super::state_file(&config.claude_dir);
    load_from(&path)
}

pub fn load_from(path: &Path) -> VpResult<State> {
    if !path.exists() {
        return Ok(State {
            version: 1,
            ..Default::default()
        });
    }
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save(state: &State, config: &super::PraxisConfig) -> VpResult<()> {
    let path = super::state_file(&config.claude_dir);
    save_to(state, &path)
}

pub fn save_to(state: &State, path: &Path) -> VpResult<()> {
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

// Re-export shared timestamp helpers so existing callers like `state::now_iso()` keep working.
pub use crate::util::{now_iso, today_iso};
