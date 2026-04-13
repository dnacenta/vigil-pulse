use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::{VpError, VpResult};

/// A single signal vector collected at a point in time.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignalVector {
    pub timestamp: String,
    pub trigger: String,
    pub signals: Signals,
    #[serde(default)]
    pub document_hashes: HashMap<String, String>,
}

/// Cognitive quality signals. Null means document was missing or not enough data.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Signals {
    // Phase 1 — activity signals
    pub vocabulary_diversity: Option<f64>,
    pub question_generation: Option<f64>,
    pub thought_lifecycle: Option<f64>,
    pub evidence_grounding: Option<f64>,
    // Phase 2 — quality signals
    #[serde(default)]
    pub conclusion_novelty: Option<f64>,
    #[serde(default)]
    pub intellectual_honesty: Option<f64>,
    #[serde(default)]
    pub position_delta: Option<f64>,
    #[serde(default)]
    pub comfort_index: Option<f64>,
}

/// Per-signal trend direction.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Trend {
    Improving,
    Stable,
    Declining,
}

/// Per-signal trend info.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignalTrend {
    pub current: Option<f64>,
    pub trend: Trend,
    pub delta: f64,
}

/// Alert level.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AlertLevel {
    Healthy,
    Watch,
    Concern,
    Alert,
}

/// Analysis result written to analysis.json.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Analysis {
    pub timestamp: String,
    pub alert_level: AlertLevel,
    pub signals: HashMap<String, SignalTrend>,
    pub improving_count: usize,
    pub stable_count: usize,
    pub declining_count: usize,
    pub highlight: Option<String>,
    pub watch_messages: Vec<String>,
    pub data_points: usize,
}

/// Configuration with thresholds.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub thresholds: HashMap<String, ThresholdPair>,
    pub window_size: usize,
    pub max_history: usize,
    pub alert_after_sessions: usize,
    pub cooldown_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThresholdPair {
    pub decline: f64,
    pub improve: f64,
}

impl Default for Config {
    fn default() -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert(
            "vocabulary_diversity".to_string(),
            ThresholdPair {
                decline: -0.05,
                improve: 0.05,
            },
        );
        thresholds.insert(
            "evidence_grounding".to_string(),
            ThresholdPair {
                decline: -0.10,
                improve: 0.10,
            },
        );
        thresholds.insert(
            "question_generation".to_string(),
            ThresholdPair {
                decline: -1.0,
                improve: 1.0,
            },
        );
        thresholds.insert(
            "thought_lifecycle".to_string(),
            ThresholdPair {
                decline: -0.10,
                improve: 0.10,
            },
        );
        for name in [
            "conclusion_novelty",
            "intellectual_honesty",
            "position_delta",
            "comfort_index",
        ] {
            thresholds.insert(
                name.to_string(),
                ThresholdPair {
                    decline: -0.10,
                    improve: 0.10,
                },
            );
        }
        Config {
            thresholds,
            window_size: 10,
            max_history: 50,
            alert_after_sessions: 7,
            cooldown_seconds: 60,
        }
    }
}

/// Pulse state (last run time for cooldown).
#[derive(Serialize, Deserialize, Default)]
pub struct PulseState {
    pub last_pulse: Option<String>,
}

// --- Conclusion/Position indexes ---

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConclusionIndex {
    pub entries: Vec<ConclusionEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConclusionEntry {
    pub timestamp: String,
    pub trigrams: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PositionIndex {
    pub entries: Vec<PositionEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PositionEntry {
    pub timestamp: String,
    pub entry_title: String,
    pub text: String,
    pub trigrams: Vec<String>,
    pub has_justification: bool,
}

// --- Calibration (HOT-2) ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalibrationPrediction {
    pub timestamp: String,
    pub predictions: HashMap<String, f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalibrationRecord {
    pub timestamp: String,
    pub signals: Vec<CalibrationSignal>,
    pub mean_surprise: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalibrationSignal {
    pub name: String,
    pub predicted: f64,
    pub actual: f64,
    pub surprise: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CalibrationHistory {
    pub records: Vec<CalibrationRecord>,
    pub pending_prediction: Option<CalibrationPrediction>,
}

// --- Load/save helpers ---

pub fn load_signals() -> VpResult<Vec<SignalVector>> {
    let path = super::signals_file().map_err(VpError::Reflection)?;
    load_signals_from(&path)
}

pub fn load_signals_from(path: &Path) -> VpResult<Vec<SignalVector>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_signals(signals: &[SignalVector]) -> VpResult<()> {
    let path = super::signals_file().map_err(VpError::Reflection)?;
    save_signals_to(signals, &path)
}

pub fn save_signals_to(signals: &[SignalVector], path: &Path) -> VpResult<()> {
    let json = serde_json::to_string_pretty(signals)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

pub fn load_analysis() -> VpResult<Option<Analysis>> {
    let path = super::analysis_file().map_err(VpError::Reflection)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let analysis = serde_json::from_str(&content)?;
    Ok(Some(analysis))
}

pub fn save_analysis(analysis: &Analysis) -> VpResult<()> {
    let path = super::analysis_file().map_err(VpError::Reflection)?;
    let json = serde_json::to_string_pretty(analysis)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

pub fn load_config() -> VpResult<Config> {
    let path = super::config_file().map_err(VpError::Reflection)?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn load_pulse_state() -> VpResult<PulseState> {
    let path = super::vigil_dir()
        .map_err(VpError::Reflection)?
        .join("pulse-state.json");
    if !path.exists() {
        return Ok(PulseState::default());
    }
    let content = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_pulse_state(state: &PulseState) -> VpResult<()> {
    let path = super::vigil_dir()
        .map_err(VpError::Reflection)?
        .join("pulse-state.json");
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

// --- Index load/save ---

fn vigil_file(name: &str) -> VpResult<std::path::PathBuf> {
    Ok(super::vigil_dir().map_err(VpError::Reflection)?.join(name))
}

pub fn load_conclusion_index() -> VpResult<ConclusionIndex> {
    let path = vigil_file("conclusion-index.json")?;
    if !path.exists() {
        return Ok(ConclusionIndex::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(&path)?)?)
}

pub fn save_conclusion_index(index: &ConclusionIndex) -> VpResult<()> {
    let path = vigil_file("conclusion-index.json")?;
    fs::write(path, serde_json::to_string_pretty(index)? + "\n")?;
    Ok(())
}

pub fn load_position_index() -> VpResult<PositionIndex> {
    let path = vigil_file("position-index.json")?;
    if !path.exists() {
        return Ok(PositionIndex::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(&path)?)?)
}

pub fn save_position_index(index: &PositionIndex) -> VpResult<()> {
    let path = vigil_file("position-index.json")?;
    fs::write(path, serde_json::to_string_pretty(index)? + "\n")?;
    Ok(())
}

pub fn load_calibration() -> VpResult<CalibrationHistory> {
    let path = vigil_file("calibration.json")?;
    if !path.exists() {
        return Ok(CalibrationHistory::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(&path)?)?)
}

pub fn save_calibration(history: &CalibrationHistory) -> VpResult<()> {
    let path = vigil_file("calibration.json")?;
    fs::write(path, serde_json::to_string_pretty(history)? + "\n")?;
    Ok(())
}

// Re-export shared timestamp helpers so existing callers like `state::now_iso()` keep working.
pub use crate::util::{now_epoch_secs, now_iso, parse_iso_epoch};
