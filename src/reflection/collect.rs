use std::collections::{HashMap, HashSet};

use owo_colors::OwoColorize;

use super::{analyze, parser, signals, state};

pub fn run(trigger: &str) -> Result<(), String> {
    let reflections_content = parser::read_or_empty(&super::reflections_file()?);
    let thoughts_content = parser::read_or_empty(&super::thoughts_file()?);
    let curiosity_content = parser::read_or_empty(&super::curiosity_file()?);

    // Load conclusion index for novelty comparison
    let conclusion_index = state::load_conclusion_index()?;
    let history_trigrams: Vec<HashSet<String>> = conclusion_index
        .entries
        .iter()
        .map(|e| e.trigrams.iter().cloned().collect())
        .collect();

    // Load position index for position_delta and comfort_index
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

    // Extract signals
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

    // Append current conclusions to the index (append-only)
    update_conclusion_index(&reflections_content, &conclusion_index)?;

    // Append current positions to the index (append-only)
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

    // Load existing history, append, trim to max
    let config = state::load_config()?;
    let mut history = state::load_signals()?;
    history.push(vector);
    if history.len() > config.max_history {
        let excess = history.len() - config.max_history;
        history.drain(..excess);
    }
    state::save_signals(&history)?;

    // Run analysis
    let analysis = analyze::run(&history, &config);
    state::save_analysis(&analysis)?;

    // Metacognitive calibration: check for pending prediction and compute surprise
    let calibration_result = resolve_calibration(&sigs)?;

    // Print summary
    println!("{} Collected signal vector ({trigger})", "✓".green());
    print_signal("  vocabulary_diversity", sigs.vocabulary_diversity);
    print_signal("  question_generation", sigs.question_generation);
    print_signal("  thought_lifecycle", sigs.thought_lifecycle);
    print_signal("  evidence_grounding", sigs.evidence_grounding);
    print_signal("  conclusion_novelty", sigs.conclusion_novelty);
    print_signal("  intellectual_honesty", sigs.intellectual_honesty);
    print_signal("  position_delta", sigs.position_delta);
    print_signal("  comfort_index", sigs.comfort_index);
    println!(
        "  History: {} data points ({} max)",
        history.len(),
        config.max_history
    );

    // Print calibration result
    match calibration_result {
        Some(record) => {
            println!(
                "  {} Calibration: mean surprise {:.3}",
                "⚖".cyan(),
                record.mean_surprise
            );
            for sig in &record.signals {
                let accuracy = if sig.surprise < 0.1 {
                    "accurate".green().to_string()
                } else if sig.surprise < 0.2 {
                    "close".yellow().to_string()
                } else {
                    "surprised".red().to_string()
                };
                println!(
                    "    {}: predicted {:.2}, actual {:.2} ({})",
                    sig.name, sig.predicted, sig.actual, accuracy
                );
            }
        }
        None => {
            println!("  No pending prediction — use `vigil-echo predict` to submit one.");
        }
    }

    Ok(())
}

/// Check for a pending calibration prediction and resolve it against actual measurements.
fn resolve_calibration(
    actual: &state::Signals,
) -> Result<Option<state::CalibrationRecord>, String> {
    let mut history = state::load_calibration()?;

    let prediction = match history.pending_prediction.take() {
        Some(p) => p,
        None => {
            state::save_calibration(&history)?;
            return Ok(None);
        }
    };

    let mut signals = Vec::new();
    let mut total_surprise = 0.0;
    let mut count = 0;

    for &name in super::SIGNAL_NAMES {
        let predicted = match prediction.predictions.get(name) {
            Some(&v) => v,
            None => continue,
        };
        let actual_val = match super::get_signal(actual, name) {
            Some(v) => v,
            None => continue,
        };
        let surprise = (predicted - actual_val).abs();
        total_surprise += surprise;
        count += 1;

        signals.push(state::CalibrationSignal {
            name: name.to_string(),
            predicted,
            actual: actual_val,
            surprise,
        });
    }

    let mean_surprise = if count > 0 {
        total_surprise / count as f64
    } else {
        0.0
    };

    let record = state::CalibrationRecord {
        timestamp: state::now_iso(),
        signals,
        mean_surprise,
    };

    // Append to history, keep last 50 records
    history.records.push(record.clone());
    if history.records.len() > 50 {
        let excess = history.records.len() - 50;
        history.records.drain(..excess);
    }

    state::save_calibration(&history)?;

    Ok(Some(record))
}

/// Submit a calibration prediction — entity predicts its own signal scores
/// before the next `collect` measurement.
pub fn predict(predictions: std::collections::HashMap<String, f64>) -> Result<(), String> {
    let mut history = state::load_calibration()?;

    history.pending_prediction = Some(state::CalibrationPrediction {
        timestamp: state::now_iso(),
        predictions,
    });

    state::save_calibration(&history)?;

    println!(
        "{} Prediction submitted — will be compared against next `collect` measurement.",
        "⚖".cyan()
    );

    Ok(())
}

/// Show calibration accuracy over time.
pub fn calibration_status() -> Result<(), String> {
    let history = state::load_calibration()?;

    if history.records.is_empty() {
        println!("No calibration data yet. Submit predictions with `vigil-echo predict`.");
        return Ok(());
    }

    println!();
    println!("  {} — metacognitive calibration", "vigil-echo".bold());
    println!();
    println!("  {} calibration records", history.records.len());

    // Overall accuracy
    let mean_surprise: f64 =
        history.records.iter().map(|r| r.mean_surprise).sum::<f64>() / history.records.len() as f64;

    let accuracy_label = if mean_surprise < 0.1 {
        "well-calibrated".green().to_string()
    } else if mean_surprise < 0.2 {
        "moderately calibrated".yellow().to_string()
    } else {
        "poorly calibrated".red().to_string()
    };

    println!("  Mean surprise: {:.3} ({})", mean_surprise, accuracy_label);

    // Per-signal accuracy (average surprise over all records)
    let mut signal_surprises: std::collections::HashMap<String, (f64, usize)> =
        std::collections::HashMap::new();
    for record in &history.records {
        for sig in &record.signals {
            let entry = signal_surprises.entry(sig.name.clone()).or_insert((0.0, 0));
            entry.0 += sig.surprise;
            entry.1 += 1;
        }
    }

    println!();
    for &name in super::SIGNAL_NAMES {
        if let Some(&(total, count)) = signal_surprises.get(name) {
            let avg = total / count as f64;
            let label = if avg < 0.1 {
                "✓"
            } else if avg < 0.2 {
                "~"
            } else {
                "✗"
            };
            println!(
                "    {} {}: avg surprise {:.3}",
                label,
                super::friendly_name(name),
                avg
            );
        }
    }

    // Trend: compare first half to second half
    if history.records.len() >= 6 {
        let mid = history.records.len() / 2;
        let first_half: f64 = history.records[..mid]
            .iter()
            .map(|r| r.mean_surprise)
            .sum::<f64>()
            / mid as f64;
        let second_half: f64 = history.records[mid..]
            .iter()
            .map(|r| r.mean_surprise)
            .sum::<f64>()
            / (history.records.len() - mid) as f64;

        let trend = if second_half < first_half - 0.02 {
            "improving ↑".green().to_string()
        } else if second_half > first_half + 0.02 {
            "degrading ↓".red().to_string()
        } else {
            "stable →".to_string()
        };
        println!();
        println!("  Trend: {trend}");
    }

    if history.pending_prediction.is_some() {
        println!();
        println!("  Pending prediction waiting for next `collect`.");
    }

    println!();

    Ok(())
}

/// Maximum entries in the conclusion index before trimming oldest.
const MAX_CONCLUSION_ENTRIES: usize = 500;

/// Append current conclusions to the persistent index for future novelty comparison.
/// Deduplicates against existing entries (skips if Jaccard > 0.95) and trims to
/// MAX_CONCLUSION_ENTRIES to prevent unbounded growth.
fn update_conclusion_index(
    reflections_content: &str,
    existing: &state::ConclusionIndex,
) -> Result<(), String> {
    let conclusions = parser::extract_conclusions(reflections_content);
    if conclusions.is_empty() {
        return Ok(());
    }

    let mut index = existing.clone();
    let timestamp = state::now_iso();

    // Build existing trigram sets for dedup comparison
    let existing_sets: Vec<HashSet<String>> = index
        .entries
        .iter()
        .map(|e| e.trigrams.iter().cloned().collect())
        .collect();

    for conclusion in &conclusions {
        let tri = parser::trigrams(conclusion);
        if tri.is_empty() {
            continue;
        }
        // Skip if this conclusion is a near-duplicate of an existing entry
        let is_duplicate = existing_sets
            .iter()
            .any(|existing| parser::jaccard_similarity(&tri, existing) > 0.95);
        if is_duplicate {
            continue;
        }
        let mut sorted: Vec<String> = tri.into_iter().collect();
        sorted.sort();
        index.entries.push(state::ConclusionEntry {
            timestamp: timestamp.clone(),
            trigrams: sorted,
        });
    }

    // Trim oldest entries if over the cap
    if index.entries.len() > MAX_CONCLUSION_ENTRIES {
        let excess = index.entries.len() - MAX_CONCLUSION_ENTRIES;
        index.entries.drain(..excess);
    }

    state::save_conclusion_index(&index)
}

/// Maximum entries in the position index before trimming oldest.
const MAX_POSITION_ENTRIES: usize = 500;

/// Append current positions to the persistent index for future comparison.
/// Deduplicates against existing entries (skips if Jaccard > 0.95) and trims to
/// MAX_POSITION_ENTRIES to prevent unbounded growth.
fn update_position_index(
    reflections_content: &str,
    existing: &state::PositionIndex,
) -> Result<(), String> {
    let positions = parser::extract_positions(reflections_content);
    if positions.is_empty() {
        return Ok(());
    }

    let mut index = existing.clone();
    let timestamp = state::now_iso();

    // Build existing trigram sets for dedup
    let existing_sets: Vec<std::collections::HashSet<String>> = index
        .entries
        .iter()
        .map(|e| e.trigrams.iter().cloned().collect())
        .collect();

    for pos in &positions {
        if pos.trigrams.is_empty() {
            continue;
        }
        // Skip near-duplicates
        let is_duplicate = existing_sets
            .iter()
            .any(|existing| parser::jaccard_similarity(&pos.trigrams, existing) > 0.95);
        if is_duplicate {
            continue;
        }
        let mut sorted: Vec<String> = pos.trigrams.iter().cloned().collect();
        sorted.sort();
        index.entries.push(state::PositionEntry {
            timestamp: timestamp.clone(),
            entry_title: pos.entry_title.clone(),
            text: pos.text.clone(),
            trigrams: sorted,
            has_justification: pos.has_justification,
        });
    }

    // Trim oldest entries if over the cap
    if index.entries.len() > MAX_POSITION_ENTRIES {
        let excess = index.entries.len() - MAX_POSITION_ENTRIES;
        index.entries.drain(..excess);
    }

    state::save_position_index(&index)
}

fn print_signal(label: &str, value: Option<f64>) {
    match value {
        Some(v) => println!("{label}: {v:.2}"),
        None => println!("{label}: —"),
    }
}
