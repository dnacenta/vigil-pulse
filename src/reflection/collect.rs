use std::collections::{HashMap, HashSet};

use owo_colors::OwoColorize;

use super::{analyze, parser, signals, state};
use crate::error::VpResult;
use crate::util;

pub fn run(trigger: &str) -> VpResult<()> {
    let reflections_content = parser::read_or_empty(
        &super::reflections_file().map_err(crate::error::VpError::Reflection)?,
    );
    let thoughts_content =
        parser::read_or_empty(&super::thoughts_file().map_err(crate::error::VpError::Reflection)?);
    let curiosity_content =
        parser::read_or_empty(&super::curiosity_file().map_err(crate::error::VpError::Reflection)?);

    // Load indexes for Phase 2 signals
    let conclusion_index = state::load_conclusion_index()?;
    let history_trigrams: Vec<HashSet<String>> = conclusion_index
        .entries
        .iter()
        .map(|e| e.trigrams.iter().cloned().collect())
        .collect();

    let position_index = state::load_position_index()?;
    let history_positions: Vec<(String, HashSet<String>, bool)> = position_index
        .entries
        .iter()
        .map(|e| {
            (
                e.text.clone(),
                e.trigrams.iter().cloned().collect(),
                e.has_justification,
            )
        })
        .collect();

    // Extract all signals
    let sigs = state::Signals {
        vocabulary_diversity: signals::vocabulary_diversity(&reflections_content),
        question_generation: signals::question_generation(&curiosity_content),
        thought_lifecycle: signals::thought_lifecycle(&thoughts_content),
        evidence_grounding: signals::evidence_grounding(&reflections_content),
        conclusion_novelty: signals::conclusion_novelty(&reflections_content, &history_trigrams),
        intellectual_honesty: signals::intellectual_honesty(&reflections_content),
        position_delta: signals::position_delta(&reflections_content, &history_positions),
        comfort_index: signals::comfort_index(&reflections_content, &history_positions),
    };

    // Update indexes
    update_conclusion_index(&reflections_content, &conclusion_index)?;
    update_position_index(&reflections_content, &position_index)?;

    // Document hashes for change detection
    let mut hashes = HashMap::new();
    hashes.insert(
        "reflections".to_string(),
        parser::hash_content(&reflections_content),
    );
    hashes.insert(
        "thoughts".to_string(),
        parser::hash_content(&thoughts_content),
    );
    hashes.insert(
        "curiosity".to_string(),
        parser::hash_content(&curiosity_content),
    );

    let vector = state::SignalVector {
        timestamp: state::now_iso(),
        trigger: trigger.to_string(),
        signals: sigs.clone(),
        document_hashes: hashes,
    };

    let config = state::load_config()?;
    let mut history = state::load_signals()?;
    history.push(vector);
    if history.len() > config.max_history {
        let excess = history.len() - config.max_history;
        history.drain(..excess);
    }
    state::save_signals(&history)?;

    let analysis = analyze::run(&history, &config);
    state::save_analysis(&analysis)?;

    // Calibration: resolve pending prediction
    let calibration_result = resolve_calibration(&sigs)?;

    // Print summary
    println!("{} Collected signal vector ({trigger})", "✓".green());
    for &name in &util::SIGNAL_NAMES {
        let val = util::get_signal(&sigs, name);
        print_signal(&format!("  {}", util::friendly_name(name)), val);
    }
    println!(
        "  History: {} data points ({} max)",
        history.len(),
        config.max_history
    );

    if let Some(record) = calibration_result {
        println!(
            "  {} Calibration: mean surprise {:.3}",
            "⚖".cyan(),
            record.mean_surprise
        );
    }

    Ok(())
}

fn print_signal(label: &str, value: Option<f64>) {
    match value {
        Some(v) => println!("{label}: {v:.2}"),
        None => println!("{label}: —"),
    }
}

const MAX_CONCLUSION_ENTRIES: usize = 500;
const MAX_POSITION_ENTRIES: usize = 500;

fn update_conclusion_index(content: &str, existing: &state::ConclusionIndex) -> VpResult<()> {
    let conclusions = parser::extract_conclusions(content);
    if conclusions.is_empty() {
        return Ok(());
    }
    let mut index = existing.clone();
    let existing_sets: Vec<HashSet<String>> = index
        .entries
        .iter()
        .map(|e| e.trigrams.iter().cloned().collect())
        .collect();
    let ts = state::now_iso();
    for c in &conclusions {
        let tri = parser::trigrams(c);
        if tri.is_empty() {
            continue;
        }
        if existing_sets
            .iter()
            .any(|e| parser::jaccard_similarity(&tri, e) > 0.95)
        {
            continue;
        }
        let mut sorted: Vec<String> = tri.into_iter().collect();
        sorted.sort();
        index.entries.push(state::ConclusionEntry {
            timestamp: ts.clone(),
            trigrams: sorted,
        });
    }
    if index.entries.len() > MAX_CONCLUSION_ENTRIES {
        index
            .entries
            .drain(..index.entries.len() - MAX_CONCLUSION_ENTRIES);
    }
    state::save_conclusion_index(&index)
}

fn update_position_index(content: &str, existing: &state::PositionIndex) -> VpResult<()> {
    let positions = parser::extract_positions(content);
    if positions.is_empty() {
        return Ok(());
    }
    let existing_sets: Vec<HashSet<String>> = existing
        .entries
        .iter()
        .map(|e| e.trigrams.iter().cloned().collect())
        .collect();
    let mut index = existing.clone();
    let ts = state::now_iso();
    for p in &positions {
        if existing_sets
            .iter()
            .any(|e| parser::jaccard_similarity(&p.trigrams, e) > 0.95)
        {
            continue;
        }
        let mut sorted: Vec<String> = p.trigrams.iter().cloned().collect();
        sorted.sort();
        index.entries.push(state::PositionEntry {
            timestamp: ts.clone(),
            entry_title: p.entry_title.clone(),
            text: p.text.clone(),
            trigrams: sorted,
            has_justification: p.has_justification,
        });
    }
    if index.entries.len() > MAX_POSITION_ENTRIES {
        index
            .entries
            .drain(..index.entries.len() - MAX_POSITION_ENTRIES);
    }
    state::save_position_index(&index)
}

fn resolve_calibration(actual: &state::Signals) -> VpResult<Option<state::CalibrationRecord>> {
    let mut history = state::load_calibration()?;
    let prediction = match history.pending_prediction.take() {
        Some(p) => p,
        None => {
            state::save_calibration(&history)?;
            return Ok(None);
        }
    };
    let mut sigs = Vec::new();
    let mut total = 0.0;
    let mut count = 0;
    for &name in &util::SIGNAL_NAMES {
        let pred = match prediction.predictions.get(name) {
            Some(&v) => v,
            None => continue,
        };
        let act = match util::get_signal(actual, name) {
            Some(v) => v,
            None => continue,
        };
        let surprise = (pred - act).abs();
        total += surprise;
        count += 1;
        sigs.push(state::CalibrationSignal {
            name: name.to_string(),
            predicted: pred,
            actual: act,
            surprise,
        });
    }
    let mean_surprise = if count > 0 { total / count as f64 } else { 0.0 };
    let record = state::CalibrationRecord {
        timestamp: state::now_iso(),
        signals: sigs,
        mean_surprise,
    };
    history.records.push(record.clone());
    if history.records.len() > 50 {
        history.records.drain(..history.records.len() - 50);
    }
    state::save_calibration(&history)?;
    Ok(Some(record))
}

/// Submit a calibration prediction.
pub fn predict(predictions: HashMap<String, f64>) -> VpResult<()> {
    let mut history = state::load_calibration()?;
    history.pending_prediction = Some(state::CalibrationPrediction {
        timestamp: state::now_iso(),
        predictions,
    });
    state::save_calibration(&history)?;
    println!("{} Prediction submitted.", "⚖".cyan());
    Ok(())
}
