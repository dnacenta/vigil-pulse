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

    // Extract signals
    let sigs = state::Signals {
        vocabulary_diversity: signals::vocabulary_diversity(&reflections_content),
        question_generation: signals::question_generation(&curiosity_content),
        thought_lifecycle: signals::thought_lifecycle(&thoughts_content),
        evidence_grounding: signals::evidence_grounding(&reflections_content),
        conclusion_novelty: signals::conclusion_novelty(&reflections_content, &history_trigrams),
        intellectual_honesty: signals::intellectual_honesty(&reflections_content),
    };

    // Append current conclusions to the index (append-only)
    update_conclusion_index(&reflections_content, &conclusion_index)?;

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

    // Print summary
    println!("{} Collected signal vector ({trigger})", "✓".green());
    print_signal("  vocabulary_diversity", sigs.vocabulary_diversity);
    print_signal("  question_generation", sigs.question_generation);
    print_signal("  thought_lifecycle", sigs.thought_lifecycle);
    print_signal("  evidence_grounding", sigs.evidence_grounding);
    print_signal("  conclusion_novelty", sigs.conclusion_novelty);
    print_signal("  intellectual_honesty", sigs.intellectual_honesty);
    println!(
        "  History: {} data points ({} max)",
        history.len(),
        config.max_history
    );

    Ok(())
}

/// Append current conclusions to the persistent index for future novelty comparison.
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

    for conclusion in &conclusions {
        let tri = parser::trigrams(conclusion);
        if tri.is_empty() {
            continue;
        }
        index.entries.push(state::ConclusionEntry {
            timestamp: timestamp.clone(),
            trigrams: tri.into_iter().collect(),
        });
    }

    state::save_conclusion_index(&index)
}

fn print_signal(label: &str, value: Option<f64>) {
    match value {
        Some(v) => println!("{label}: {v:.2}"),
        None => println!("{label}: —"),
    }
}
