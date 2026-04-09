use std::collections::HashMap;

use owo_colors::OwoColorize;

use super::{analyze, parser, signals, state};
use crate::error::VpResult;

pub fn run(trigger: &str) -> VpResult<()> {
    let reflections_content = parser::read_or_empty(
        &super::reflections_file().map_err(crate::error::VpError::Reflection)?,
    );
    let thoughts_content =
        parser::read_or_empty(&super::thoughts_file().map_err(crate::error::VpError::Reflection)?);
    let curiosity_content =
        parser::read_or_empty(&super::curiosity_file().map_err(crate::error::VpError::Reflection)?);

    // Extract signals
    let sigs = state::Signals {
        vocabulary_diversity: signals::vocabulary_diversity(&reflections_content),
        question_generation: signals::question_generation(&curiosity_content),
        thought_lifecycle: signals::thought_lifecycle(&thoughts_content),
        evidence_grounding: signals::evidence_grounding(&reflections_content),
    };

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
    println!(
        "  History: {} data points ({} max)",
        history.len(),
        config.max_history
    );

    Ok(())
}

fn print_signal(label: &str, value: Option<f64>) {
    match value {
        Some(v) => println!("{label}: {v:.2}"),
        None => println!("{label}: —"),
    }
}
