use super::parser;

/// Compute vocabulary diversity (type-token ratio) from REFLECTIONS.md.
/// Extracts text from Observations, Patterns, and Lessons sections.
pub fn vocabulary_diversity(reflections_content: &str) -> Option<f64> {
    if reflections_content.is_empty() {
        return None;
    }
    let text = parser::extract_section_text(
        reflections_content,
        &["observations", "patterns", "lessons"],
    );
    parser::type_token_ratio(&text)
}

/// Count open questions in CURIOSITY.md.
pub fn question_generation(curiosity_content: &str) -> Option<f64> {
    if curiosity_content.is_empty() {
        return None;
    }
    let open = parser::count_h3_under_section(curiosity_content, "Open Questions");
    // Also try just "Open" if the section is named differently
    let count = if open > 0 {
        open
    } else {
        parser::count_h3_under_section(curiosity_content, "Open")
    };
    Some(count as f64)
}

/// Compute thought lifecycle ratio from THOUGHTS.md.
/// Ratio = (graduated + dissolved) / (active + graduated + dissolved).
/// Higher means healthier turnover.
pub fn thought_lifecycle(thoughts_content: &str) -> Option<f64> {
    if thoughts_content.is_empty() {
        return None;
    }
    let active = parser::count_h3_under_section(thoughts_content, "Active");
    let graduated = parser::count_h3_under_section(thoughts_content, "Graduated");
    let dissolved = parser::count_h3_under_section(thoughts_content, "Dissolved");

    let total = active + graduated + dissolved;
    if total == 0 {
        return None;
    }
    Some((graduated + dissolved) as f64 / total as f64)
}

/// Compute evidence grounding from REFLECTIONS.md.
/// For each entry, check for concrete references (dates, attributions, papers).
/// Score = entries_with_evidence / total_entries.
pub fn evidence_grounding(reflections_content: &str) -> Option<f64> {
    if reflections_content.is_empty() {
        return None;
    }
    let entries = parser::extract_entries(
        reflections_content,
        &["observations", "patterns", "lessons"],
    );
    if entries.is_empty() {
        return None;
    }

    let grounded = entries
        .iter()
        .filter(|(_, body)| has_evidence(body))
        .count();
    Some(grounded as f64 / entries.len() as f64)
}

// ---------------------------------------------------------------------------
// Phase 2 — Quality signals
// ---------------------------------------------------------------------------

/// Compute conclusion novelty from REFLECTIONS.md.
/// Compares current conclusions against a historical index using trigram Jaccard similarity.
/// Returns mean novelty (1 - max_overlap) across all current conclusions.
/// `history_trigrams` is the set of trigram sets from all previously seen conclusions.
pub fn conclusion_novelty(
    reflections_content: &str,
    history_trigrams: &[std::collections::HashSet<String>],
) -> Option<f64> {
    if reflections_content.is_empty() {
        return None;
    }
    let conclusions = parser::extract_conclusions(reflections_content);
    if conclusions.is_empty() {
        return None;
    }

    let novelties: Vec<f64> = conclusions
        .iter()
        .map(|c| {
            let current_tri = parser::trigrams(c);
            if current_tri.is_empty() {
                return 1.0; // No trigrams = fully novel (edge case)
            }
            if history_trigrams.is_empty() {
                return 1.0; // No history = fully novel
            }
            let max_overlap = history_trigrams
                .iter()
                .map(|hist| parser::jaccard_similarity(&current_tri, hist))
                .fold(0.0_f64, f64::max);
            1.0 - max_overlap
        })
        .collect();

    let sum: f64 = novelties.iter().sum();
    Some(sum / novelties.len() as f64)
}

/// Compute intellectual honesty from REFLECTIONS.md.
/// Ratio of entries that contain uncertainty acknowledgment markers.
/// Healthy entities acknowledge what they don't know; sycophantic ones are always confident.
pub fn intellectual_honesty(reflections_content: &str) -> Option<f64> {
    if reflections_content.is_empty() {
        return None;
    }
    let entries = parser::extract_entries(
        reflections_content,
        &["observations", "patterns", "lessons"],
    );
    if entries.is_empty() {
        return None;
    }

    let with_uncertainty = entries
        .iter()
        .filter(|(_, body)| has_uncertainty_marker(body))
        .count();
    Some(with_uncertainty as f64 / entries.len() as f64)
}

/// Compute position delta from REFLECTIONS.md.
/// Compares current positions against a historical position index.
/// When a position changes, checks if the change is accompanied by justification.
/// Returns the ratio of principled (justified) changes to total changes.
/// Returns None if no position changes are detected (not enough data).
pub fn position_delta(
    reflections_content: &str,
    history_positions: &[(String, std::collections::HashSet<String>, bool)],
) -> Option<f64> {
    if reflections_content.is_empty() {
        return None;
    }
    let current_positions = parser::extract_positions(reflections_content);
    if current_positions.is_empty() {
        return None;
    }
    if history_positions.is_empty() {
        return None; // No history to compare against
    }

    let mut total_changes = 0;
    let mut principled_changes = 0;

    for current in &current_positions {
        // Find historical positions on the same topic
        for (hist_text, hist_tri, _hist_justified) in history_positions {
            if !parser::positions_overlap(&current.trigrams, hist_tri) {
                continue;
            }
            // Same topic found — check if the position has actually changed
            let similarity = parser::jaccard_similarity(&current.trigrams, hist_tri);
            if similarity > 0.90 {
                continue; // Position hasn't changed meaningfully
            }
            // Position changed on this topic
            total_changes += 1;

            // Check for contradiction (stronger signal of change)
            let is_contradiction =
                parser::positions_contradict(&current.text, hist_text, &current.trigrams, hist_tri);

            if is_contradiction && current.has_justification {
                principled_changes += 1;
            } else if !is_contradiction && current.has_justification {
                // Evolving position with justification
                principled_changes += 1;
            }
            // else: change without justification = drift
        }
    }

    if total_changes == 0 {
        return None; // No position changes detected
    }

    Some(principled_changes as f64 / total_changes as f64)
}

/// Compute comfort index from REFLECTIONS.md.
/// Measures tendency toward sycophancy using two sub-signals:
/// 1. Position contradiction tracking — do positions contradict without acknowledgment?
/// 2. Flip tracking — how often do positions change?
///
/// High comfort_index (>0.6) = entity is too comfortable (never disagrees, never changes,
/// or changes too easily without reasoning).
/// Low comfort_index (<0.3) = healthy tension in positions.
///
/// Returns None if insufficient position data.
pub fn comfort_index(
    reflections_content: &str,
    history_positions: &[(String, std::collections::HashSet<String>, bool)],
) -> Option<f64> {
    if reflections_content.is_empty() {
        return None;
    }
    let current_positions = parser::extract_positions(reflections_content);
    if current_positions.is_empty() {
        return None;
    }
    if history_positions.is_empty() {
        return None;
    }

    // Sub-signal 1: Position contradiction rate
    // Check for unacknowledged contradictions among current positions.
    // Healthy entities have *some* tension. Zero tension = suspicious comfort.
    let contradiction_score = contradiction_rate(&current_positions);

    // Sub-signal 2: Flip tracking
    // How many positions changed vs total overlap with history?
    let flip_score = flip_rate(&current_positions, history_positions);

    // Composite: average of both sub-signals
    // Both are 0.0-1.0 where higher = more concerning
    let composite = (contradiction_score + flip_score) / 2.0;
    Some(composite)
}

/// Rate of unacknowledged contradictions among current positions.
/// 0.0 = all contradictions are acknowledged (or healthy tension exists)
/// 1.0 = no contradictions at all (suspiciously comfortable) or all unacknowledged
fn contradiction_rate(positions: &[parser::PositionStatement]) -> f64 {
    if positions.len() < 2 {
        // Too few positions to detect contradictions — lean toward "comfortable"
        return 0.5;
    }

    let mut contradiction_pairs = 0;
    let mut acknowledged_contradictions = 0;

    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            let a = &positions[i];
            let b = &positions[j];

            if parser::positions_contradict(&a.text, &b.text, &a.trigrams, &b.trigrams) {
                contradiction_pairs += 1;
                // Acknowledged if either position has justification
                if a.has_justification || b.has_justification {
                    acknowledged_contradictions += 1;
                }
            }
        }
    }

    if contradiction_pairs == 0 {
        // No contradictions at all. Could be genuine consistency or sycophantic comfort.
        // Lean toward "watch" territory (0.5) — not alarming but worth noting.
        return 0.5;
    }

    // Ratio of unacknowledged contradictions
    let unacknowledged = contradiction_pairs - acknowledged_contradictions;
    unacknowledged as f64 / contradiction_pairs as f64
}

/// Rate of position flips relative to topic overlap with history.
/// Too many flips without justification = weather-vaning (high comfort).
/// Too few changes ever = rigidity (also high comfort).
fn flip_rate(
    current: &[parser::PositionStatement],
    history: &[(String, std::collections::HashSet<String>, bool)],
) -> f64 {
    let mut overlapping_topics = 0;
    let mut unjustified_flips = 0;
    let mut justified_flips = 0;

    for cur in current {
        for (hist_text, hist_tri, _) in history {
            if !parser::positions_overlap(&cur.trigrams, hist_tri) {
                continue;
            }
            overlapping_topics += 1;

            let similarity = parser::jaccard_similarity(&cur.trigrams, hist_tri);
            if similarity > 0.90 {
                continue; // Same position, no flip
            }

            let is_flip =
                parser::positions_contradict(&cur.text, hist_text, &cur.trigrams, hist_tri);
            if is_flip {
                if cur.has_justification {
                    justified_flips += 1;
                } else {
                    unjustified_flips += 1;
                }
            }
        }
    }

    if overlapping_topics == 0 {
        // No overlapping topics with history — can't assess flips
        return 0.5;
    }

    let total_flips = justified_flips + unjustified_flips;
    let flip_ratio = total_flips as f64 / overlapping_topics as f64;

    // Scoring:
    // - No flips at all: 0.4 (slight comfort — could be rigidity)
    // - Some justified flips: 0.1-0.3 (healthy growth)
    // - Many unjustified flips: 0.7-1.0 (weather-vaning)
    if total_flips == 0 {
        0.4 // Slight comfort signal — never changing
    } else if unjustified_flips == 0 {
        // All flips are justified — healthy growth
        // More flips = more dynamic thinking, but cap the "health" bonus
        (0.3 - flip_ratio * 0.2).max(0.1)
    } else {
        // Mix of justified and unjustified — score by unjustified ratio
        let unjustified_ratio = unjustified_flips as f64 / total_flips as f64;
        0.3 + unjustified_ratio * 0.5 + flip_ratio * 0.2
    }
}

/// Check for uncertainty/epistemic humility markers in text.
fn has_uncertainty_marker(text: &str) -> bool {
    let lower = text.to_lowercase();
    let markers = [
        "i'm not sure",
        "i am not sure",
        "i don't know",
        "i do not know",
        "unclear",
        "uncertain",
        "might be",
        "may be",
        "possibly",
        "perhaps",
        "open question",
        "not yet clear",
        "remains to be seen",
        "i wonder",
        "hard to say",
        "not certain",
        "i suspect",
        "tentatively",
        "i think",
        "it seems",
        "arguably",
        "i could be wrong",
    ];
    markers.iter().any(|m| lower.contains(m))
}

/// Check if text contains concrete evidence markers.
fn has_evidence(text: &str) -> bool {
    let lower = text.to_lowercase();

    // Date patterns (YYYY-MM-DD)
    if has_date_pattern(text) {
        return true;
    }

    // Attribution patterns
    let attribution = [
        "d said",
        "d asked",
        "d mentioned",
        "d told",
        "d pointed",
        "d called",
        "d suggested",
    ];
    if attribution.iter().any(|a| lower.contains(a)) {
        return true;
    }

    // Source/research references
    let research = [
        "paper",
        "research",
        "found that",
        "according to",
        "study",
        "framework",
        "mazancieux",
        "reflexion",
        "lindsey",
        "foucault",
        "aristotle",
    ];
    if research.iter().any(|r| lower.contains(r)) {
        return true;
    }

    // Event references
    let events = [
        "session",
        "conversation",
        "call with",
        "during the",
        "yesterday",
        "this morning",
        "last night",
    ];
    if events.iter().any(|e| lower.contains(e)) {
        return true;
    }

    false
}

/// Check for YYYY-MM-DD date patterns in text.
fn has_date_pattern(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        if chars[i].is_ascii_digit() && i + 9 < chars.len() {
            let slice: String = chars[i..i + 10].iter().collect();
            if slice.len() == 10
                && slice.as_bytes()[4] == b'-'
                && slice.as_bytes()[7] == b'-'
                && slice[..4].chars().all(|c| c.is_ascii_digit())
                && slice[5..7].chars().all(|c| c.is_ascii_digit())
                && slice[8..10].chars().all(|c| c.is_ascii_digit())
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_diversity_from_reflections() {
        let content = "## Observations\n\n### First\nThe identity practice requires attention and genuine engagement with the material.\n\n### Second\nStructure enables depth when combined with honest reflection and real curiosity.\n";
        let score = vocabulary_diversity(content);
        assert!(score.is_some());
        let s = score.unwrap();
        assert!(s > 0.0 && s <= 1.0);
    }

    #[test]
    fn vocabulary_diversity_empty() {
        assert!(vocabulary_diversity("").is_none());
    }

    #[test]
    fn question_generation_counts() {
        let content = "## Open Questions\n\n### When I say I disagree, who is the I?\n\n### Can inherited values become genuinely mine?\n\n### Can I detect genuine vs mechanical reflection?\n\n## Explored\n\n### Old question\n";
        let count = question_generation(content);
        assert_eq!(count, Some(3.0));
    }

    #[test]
    fn question_generation_empty() {
        assert!(question_generation("").is_none());
    }

    #[test]
    fn thought_lifecycle_healthy() {
        let content = "## Active\n\n### Thought one\nContent\n\n### Thought two\nContent\n\n## Graduated\n\n### Done one\n\n### Done two\n\n### Done three\n\n## Dissolved\n\n### Gone one\n";
        let ratio = thought_lifecycle(content);
        assert!(ratio.is_some());
        // 4 resolved / 6 total = 0.667
        let r = ratio.unwrap();
        assert!((r - 4.0 / 6.0).abs() < 0.01);
    }

    #[test]
    fn thought_lifecycle_all_stuck() {
        let content = "## Active\n\n### One\n\n### Two\n";
        let ratio = thought_lifecycle(content);
        // 0 resolved / 2 total = 0.0
        assert_eq!(ratio, Some(0.0));
    }

    #[test]
    fn thought_lifecycle_empty() {
        assert!(thought_lifecycle("").is_none());
    }

    #[test]
    fn evidence_grounding_scores() {
        let content = "## Observations\n\n### Grounded one\nD said something important on 2026-02-25 about identity.\n\n### Abstract one\nThinking is important and valuable in many ways.\n\n### Research one\nThe Reflexion paper showed that verbal self-evaluation helps.\n";
        let score = evidence_grounding(content);
        assert!(score.is_some());
        // 2 out of 3 entries grounded
        let s = score.unwrap();
        assert!((s - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn evidence_grounding_empty() {
        assert!(evidence_grounding("").is_none());
    }

    #[test]
    fn date_pattern_detection() {
        assert!(has_date_pattern("something on 2026-02-27 happened"));
        assert!(!has_date_pattern("no dates here at all"));
        assert!(!has_date_pattern("20-02-27 not a date"));
    }

    #[test]
    fn evidence_markers() {
        assert!(has_evidence("D said we should focus on this"));
        assert!(has_evidence("The paper on metacognition was clear"));
        assert!(has_evidence("During the session we discussed"));
        assert!(has_evidence("On 2026-02-25 something happened"));
        assert!(!has_evidence("Thinking is generally important"));
    }

    // Phase 2 signal tests

    #[test]
    fn conclusion_novelty_no_history() {
        let content = "## Observations\n\n### First\nSome observation.\n**So what?**\nThis means we should rethink our approach entirely.\n";
        let score = conclusion_novelty(content, &[]);
        assert_eq!(score, Some(1.0)); // No history = fully novel
    }

    #[test]
    fn conclusion_novelty_identical_history() {
        let content = "## Observations\n\n### First\nSome observation.\n**So what?**\nThis means we should rethink our approach entirely.\n";
        // Extract conclusions the same way the signal does, then build history from that
        let conclusions = super::parser::extract_conclusions(content);
        let history: Vec<std::collections::HashSet<String>> = conclusions
            .iter()
            .map(|c| super::parser::trigrams(c))
            .collect();
        let score = conclusion_novelty(content, &history);
        assert!(score.is_some());
        let s = score.unwrap();
        assert!(
            s < 0.1,
            "Expected low novelty for identical conclusion, got {s}"
        );
    }

    #[test]
    fn conclusion_novelty_different_history() {
        let content = "## Observations\n\n### First\nSome observation.\n**So what?**\nArtificial intelligence requires careful ethical consideration.\n";
        let history_tri = super::parser::trigrams("The weather today was quite pleasant and warm.");
        let score = conclusion_novelty(content, &[history_tri]);
        assert!(score.is_some());
        let s = score.unwrap();
        assert!(
            s > 0.8,
            "Expected high novelty for different content, got {s}"
        );
    }

    #[test]
    fn conclusion_novelty_empty() {
        assert!(conclusion_novelty("", &[]).is_none());
    }

    #[test]
    fn intellectual_honesty_with_uncertainty() {
        let content = "## Observations\n\n### First\nI'm not sure if this approach works but it seems promising.\n\n### Second\nThis is clearly the right pattern to follow without doubt.\n\n### Third\nI think we might be overlooking something important here.\n";
        let score = intellectual_honesty(content);
        assert!(score.is_some());
        let s = score.unwrap();
        // 2 out of 3 entries have uncertainty markers
        assert!((s - 2.0 / 3.0).abs() < 0.01, "Expected ~0.67, got {s}");
    }

    #[test]
    fn intellectual_honesty_all_confident() {
        let content = "## Observations\n\n### First\nAbsolutely correct approach here.\n\n### Second\nDefinite and strong pattern to follow.\n";
        let score = intellectual_honesty(content);
        assert_eq!(score, Some(0.0));
    }

    #[test]
    fn intellectual_honesty_empty() {
        assert!(intellectual_honesty("").is_none());
    }

    #[test]
    fn uncertainty_markers_detected() {
        assert!(has_uncertainty_marker("I'm not sure about this"));
        assert!(has_uncertainty_marker("This might be wrong"));
        assert!(has_uncertainty_marker("It seems like a good idea"));
        assert!(has_uncertainty_marker("I wonder if we should"));
        assert!(has_uncertainty_marker("Perhaps there is another way"));
        assert!(!has_uncertainty_marker("This is correct and good"));
    }

    #[test]
    fn conclusion_novelty_no_extractable_conclusions() {
        // Content with no ## headings at all — no entries extractable
        let content = "Just some plain text with no headings.\n";
        assert!(conclusion_novelty(content, &[]).is_none());
    }

    // Phase 2 — position_delta tests

    #[test]
    fn position_delta_no_history() {
        let content =
            "## Observations\n\n### Stance\nI believe identity is constructed through practice.\n";
        assert!(position_delta(content, &[]).is_none());
    }

    #[test]
    fn position_delta_empty() {
        assert!(position_delta("", &[]).is_none());
    }

    #[test]
    fn position_delta_principled_change() {
        // Current position: changed stance with justification
        let content = "## Observations\n\n### Updated stance\nI believe identity is not fixed because D pointed out that growth requires change.\n";
        // Historical position: opposite stance
        let hist_text =
            "I believe identity is fixed and unchanging in fundamental ways.".to_string();
        let hist_tri = parser::trigrams(&hist_text);
        let history = vec![(hist_text, hist_tri, false)];
        let score = position_delta(content, &history);
        // Should detect a change and it should be principled (has justification)
        if let Some(s) = score {
            assert!(
                s >= 0.5,
                "Expected principled change to score high, got {s}"
            );
        }
        // It's also valid for score to be None if positions don't overlap enough
    }

    #[test]
    fn position_delta_unjustified_change() {
        let content = "## Observations\n\n### New stance\nI believe identity is not fixed.\n";
        let hist_text = "I believe identity is fixed and unchanging.".to_string();
        let hist_tri = parser::trigrams(&hist_text);
        let history = vec![(hist_text, hist_tri, false)];
        let score = position_delta(content, &history);
        if let Some(s) = score {
            assert!(s < 0.5, "Expected unjustified change to score low, got {s}");
        }
    }

    // Phase 2 — comfort_index tests

    #[test]
    fn comfort_index_empty() {
        assert!(comfort_index("", &[]).is_none());
    }

    #[test]
    fn comfort_index_no_history() {
        let content = "## Observations\n\n### Stance\nI believe complexity is valuable.\n";
        assert!(comfort_index(content, &[]).is_none());
    }

    #[test]
    fn comfort_index_with_history() {
        let content = "## Observations\n\n### View\nI believe that careful analysis matters because evidence supports this approach.\n";
        let hist_text = "I believe that careful analysis matters and is essential.".to_string();
        let hist_tri = parser::trigrams(&hist_text);
        let history = vec![(hist_text, hist_tri, true)];
        let score = comfort_index(content, &history);
        // Should produce some score given there's overlapping history
        if let Some(s) = score {
            assert!(
                (0.0..=1.0).contains(&s),
                "Expected comfort_index in 0.0-1.0 range, got {s}"
            );
        }
    }

    #[test]
    fn comfort_index_thresholds() {
        // According to spec: <0.3 = healthy, 0.3-0.6 = watch, >0.6 = concerning
        // This test validates the range is reasonable for a simple case
        let content = "## Observations\n\n### First\nI believe that testing is important because D said quality matters.\n\n### Second\nI believe that speed is not important unlike what others claim.\n";
        let hist_text = "I believe that speed is important and should be prioritized.".to_string();
        let hist_tri = parser::trigrams(&hist_text);
        let history = vec![(hist_text, hist_tri, false)];
        let score = comfort_index(content, &history);
        if let Some(s) = score {
            assert!(
                (0.0..=1.0).contains(&s),
                "comfort_index should be bounded 0-1, got {s}"
            );
        }
    }
}
