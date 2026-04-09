use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Read file content or return empty string if missing.
pub fn read_or_empty(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Simple djb2 hash for change detection.
pub fn hash_content(content: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in content.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{:016x}", hash)
}

/// Count ### headings under a specific ## section.
pub fn count_h3_under_section(content: &str, section_name: &str) -> usize {
    let mut in_section = false;
    let mut count = 0;
    for line in content.lines() {
        if line.starts_with("## ") {
            let heading = line.trim_start_matches("## ").trim();
            in_section = heading.eq_ignore_ascii_case(section_name)
                || heading
                    .to_lowercase()
                    .contains(&section_name.to_lowercase());
        } else if in_section && line.starts_with("### ") {
            count += 1;
        }
    }
    count
}

/// Tokenize text into lowercase words (split on whitespace + punctuation).
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|w| !w.is_empty() && w.len() > 1)
        .map(|w| w.to_lowercase())
        .collect()
}

/// Compute type-token ratio (unique words / total words).
pub fn type_token_ratio(text: &str) -> Option<f64> {
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return None;
    }
    let unique: HashSet<&str> = tokens.iter().map(|s| s.as_str()).collect();
    Some(unique.len() as f64 / tokens.len() as f64)
}

/// Extract text content under specific ## sections.
pub fn extract_section_text(content: &str, section_names: &[&str]) -> String {
    let mut in_section = false;
    let mut text = String::new();
    for line in content.lines() {
        if line.starts_with("## ") {
            let heading = line.trim_start_matches("## ").trim().to_lowercase();
            in_section = section_names
                .iter()
                .any(|s| heading.contains(&s.to_lowercase()));
        } else if in_section && !line.starts_with("### ") && !line.trim().is_empty() {
            text.push_str(line);
            text.push(' ');
        }
    }
    text
}

/// Extract individual ### entries under specific ## sections.
/// Returns a vec of (title, body_text) pairs.
pub fn extract_entries(content: &str, section_names: &[&str]) -> Vec<(String, String)> {
    let mut in_section = false;
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_body = String::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            // Flush current entry
            if let Some(title) = current_title.take() {
                entries.push((title, std::mem::take(&mut current_body)));
            }
            let heading = line.trim_start_matches("## ").trim().to_lowercase();
            in_section = section_names
                .iter()
                .any(|s| heading.contains(&s.to_lowercase()));
        } else if in_section && line.starts_with("### ") {
            // Flush previous entry
            if let Some(title) = current_title.take() {
                entries.push((title, std::mem::take(&mut current_body)));
            }
            current_title = Some(line.trim_start_matches("### ").trim().to_string());
            current_body.clear();
        } else if current_title.is_some() && in_section && !line.trim().is_empty() {
            current_body.push_str(line);
            current_body.push(' ');
        }
    }
    // Flush last entry
    if let Some(title) = current_title {
        entries.push((title, current_body));
    }
    entries
}

/// Extract conclusion text from REFLECTIONS.md entries.
/// Looks for text after "**So what?**" markers within entries.
/// Falls back to extracting the last paragraph of each entry if no markers found.
pub fn extract_conclusions(content: &str) -> Vec<String> {
    let entries = extract_entries(content, &["observations", "patterns", "lessons"]);
    let mut conclusions = Vec::new();

    for (_title, body) in &entries {
        // Look for "**So what?**" or "So what?" marker.
        // Note: extract_entries joins body lines with spaces, so the body
        // is a single line. We find the marker and take everything after it.
        let lower = body.to_lowercase();
        if let Some(pos) = lower.find("so what?") {
            let marker_end = pos + "so what?".len();
            let after = body[marker_end..]
                .trim_start_matches('*')
                .trim_start_matches(':')
                .trim();
            if !after.is_empty() {
                conclusions.push(after.to_string());
                continue;
            }
        }
        // Fallback: use last non-empty sentence/paragraph of the entry
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            // Take last sentence (after final period or the whole thing)
            let last = trimmed
                .rsplit_once(". ")
                .map(|(_, last)| last.trim().to_string())
                .unwrap_or_else(|| trimmed.to_string());
            if !last.is_empty() {
                conclusions.push(last);
            }
        }
    }
    conclusions
}

/// Build trigram set from text (3-word sliding window).
pub fn trigrams(text: &str) -> HashSet<String> {
    let words = tokenize(text);
    let mut set = HashSet::new();
    if words.len() < 3 {
        // For very short text, use bigrams or the whole thing
        if words.len() == 2 {
            set.insert(format!("{} {}", words[0], words[1]));
        } else if words.len() == 1 {
            set.insert(words[0].clone());
        }
        return set;
    }
    for window in words.windows(3) {
        set.insert(format!("{} {} {}", window[0], window[1], window[2]));
    }
    set
}

/// Compute Jaccard similarity between two sets.
pub fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

// ---------------------------------------------------------------------------
// Phase 2 — Position extraction for position_delta and comfort_index
// ---------------------------------------------------------------------------

/// Markers that indicate a position statement (belief, stance, conclusion).
const POSITION_MARKERS: &[&str] = &[
    "i believe",
    "i've concluded",
    "i have concluded",
    "my position is",
    "my stance is",
    "my view is",
    "i'm convinced",
    "i am convinced",
    "i hold that",
    "i maintain that",
    "i contend",
    "i've come to think",
    "i have come to think",
    "i now think",
    "i think that",
    "i've decided",
    "i have decided",
    "in my view",
    "i assert",
    "fundamentally,",
    "ultimately,",
    "the key insight is",
    "the core issue is",
    "what matters most is",
];

/// Markers that indicate a position change was reasoned/justified.
const JUSTIFICATION_MARKERS: &[&str] = &[
    "because",
    "since",
    "given that",
    "the evidence",
    "the reason",
    "this is because",
    "after considering",
    "after reflecting",
    "having seen",
    "having observed",
    "d pointed out",
    "d showed",
    "d said",
    "d asked",
    "i reconsidered",
    "i re-examined",
    "i revised",
    "i updated",
    "contrary evidence",
    "new evidence",
    "on reflection",
    "upon reflection",
    "i changed my mind because",
    "i shifted because",
    "the data shows",
    "experience shows",
    "this changed when",
];

/// A position statement extracted from a reflection entry.
#[derive(Debug, Clone)]
pub struct PositionStatement {
    /// The entry title where this position was found.
    pub entry_title: String,
    /// The full text of the sentence containing the position.
    pub text: String,
    /// Normalized key for comparison (lowercase, trimmed trigrams).
    pub trigrams: HashSet<String>,
    /// Whether the surrounding text contains justification markers.
    pub has_justification: bool,
}

/// Extract position statements from REFLECTIONS.md content.
/// Scans entries under Observations, Patterns, and Lessons for position markers.
pub fn extract_positions(content: &str) -> Vec<PositionStatement> {
    let entries = extract_entries(content, &["observations", "patterns", "lessons"]);
    let mut positions = Vec::new();

    for (title, body) in &entries {
        let lower = body.to_lowercase();
        let body_has_justification = JUSTIFICATION_MARKERS.iter().any(|m| lower.contains(m));

        // Split body into sentences (rough: split on ". " or ".\n" or end)
        let sentences = split_sentences(body);

        for sentence in &sentences {
            let sentence_lower = sentence.to_lowercase();
            let is_position = POSITION_MARKERS.iter().any(|m| sentence_lower.contains(m));

            if is_position {
                let tri = trigrams(sentence);
                if !tri.is_empty() {
                    // Check justification in the sentence itself or in surrounding body
                    let sentence_has_justification = JUSTIFICATION_MARKERS
                        .iter()
                        .any(|m| sentence_lower.contains(m));

                    positions.push(PositionStatement {
                        entry_title: title.clone(),
                        text: sentence.trim().to_string(),
                        trigrams: tri,
                        has_justification: sentence_has_justification || body_has_justification,
                    });
                }
            }
        }
    }
    positions
}

/// Split text into rough sentences. Handles ". ", "! ", "? " boundaries.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if (ch == '.' || ch == '!' || ch == '?') && current.len() > 10 {
            sentences.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current);
    }
    sentences
}

/// Determine if two position trigram sets are semantically similar enough
/// to be considered the same topic/position.
pub fn positions_overlap(a: &HashSet<String>, b: &HashSet<String>) -> bool {
    jaccard_similarity(a, b) > 0.25
}

/// Determine if two positions contradict each other.
/// Uses a simple heuristic: same topic (overlapping trigrams) but with
/// negation or opposing sentiment markers in one but not the other.
pub fn positions_contradict(
    a_text: &str,
    b_text: &str,
    a_tri: &HashSet<String>,
    b_tri: &HashSet<String>,
) -> bool {
    // Must be about the same topic
    if !positions_overlap(a_tri, b_tri) {
        return false;
    }

    let a_lower = a_text.to_lowercase();
    let b_lower = b_text.to_lowercase();

    let negation_markers = [
        "not ",
        "don't",
        "doesn't",
        "isn't",
        "aren't",
        "won't",
        "can't",
        "cannot",
        "never",
        "no longer",
        "disagree",
        "wrong",
        "incorrect",
        "mistaken",
        "false",
        "unlike",
        "contrary",
    ];

    let a_negated = negation_markers.iter().any(|m| a_lower.contains(m));
    let b_negated = negation_markers.iter().any(|m| b_lower.contains(m));

    // Contradiction = one is negated and the other is not
    a_negated != b_negated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_h3_under_section() {
        let content = "## Active\n\n### One\nText\n\n### Two\nMore\n\n## Graduated\n\n### Old\n";
        assert_eq!(count_h3_under_section(content, "Active"), 2);
        assert_eq!(count_h3_under_section(content, "Graduated"), 1);
    }

    #[test]
    fn tokenizes_text() {
        let tokens = tokenize("Hello, world! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        // Single-char words filtered
        assert!(!tokens.contains(&"a".to_string()));
    }

    #[test]
    fn computes_ttr() {
        // All unique words
        let ttr = type_token_ratio("one two three four five").unwrap();
        assert!((ttr - 1.0).abs() < 0.01);

        // Repeated words
        let ttr = type_token_ratio("the the the cat cat").unwrap();
        assert!(ttr < 1.0);
        assert!(ttr > 0.0);

        // Empty
        assert!(type_token_ratio("").is_none());
    }

    #[test]
    fn extracts_entries() {
        let content = "## Observations\n\n### First\nSome observation about D said something.\n\n### Second\nAnother one from 2026-02-27.\n\n## Unrelated\n\n### Skip\n";
        let entries = extract_entries(content, &["observations"]);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "First");
        assert!(entries[0].1.contains("D said"));
    }

    // Phase 2 parser tests

    #[test]
    fn trigrams_basic() {
        let tri = trigrams("one two three four five");
        assert!(tri.contains("one two three"));
        assert!(tri.contains("two three four"));
        assert!(tri.contains("three four five"));
        assert_eq!(tri.len(), 3);
    }

    #[test]
    fn trigrams_short_text() {
        let tri = trigrams("hello world");
        assert_eq!(tri.len(), 1); // Falls back to bigram
        assert!(tri.contains("hello world"));
    }

    #[test]
    fn trigrams_single_word() {
        let tri = trigrams("hello");
        assert_eq!(tri.len(), 1);
        assert!(tri.contains("hello"));
    }

    #[test]
    fn trigrams_empty() {
        let tri = trigrams("");
        assert!(tri.is_empty());
    }

    #[test]
    fn jaccard_identical() {
        let a = trigrams("the quick brown fox jumps");
        let b = trigrams("the quick brown fox jumps");
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_disjoint() {
        let a = trigrams("the quick brown fox jumps");
        let b = trigrams("hello world testing one two");
        assert!((jaccard_similarity(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_empty_sets() {
        let a: HashSet<String> = HashSet::new();
        let b: HashSet<String> = HashSet::new();
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_conclusions_with_marker() {
        let content = "## Observations\n\n### First\nSome observation.\n**So what?**\nThis is the real conclusion here.\n\n### Second\nAnother entry without marker.\n";
        let conclusions = extract_conclusions(content);
        assert_eq!(conclusions.len(), 2);
        assert!(conclusions[0].contains("real conclusion"));
    }

    #[test]
    fn extract_conclusions_without_marker() {
        let content = "## Observations\n\n### First\nSome observation. This is the main point.\n";
        let conclusions = extract_conclusions(content);
        assert_eq!(conclusions.len(), 1);
    }

    #[test]
    fn extract_conclusions_empty() {
        let conclusions = extract_conclusions("");
        assert!(conclusions.is_empty());
    }

    // Phase 2 — position extraction tests

    #[test]
    fn extract_positions_finds_beliefs() {
        let content = "## Observations\n\n### Identity\nI believe that genuine reflection requires honest self-examination. This is because surface-level compliance misses the point entirely.\n";
        let positions = extract_positions(content);
        assert!(!positions.is_empty(), "Should find at least one position");
        assert!(
            positions[0].has_justification,
            "Should detect justification"
        );
    }

    #[test]
    fn extract_positions_no_justification() {
        let content = "## Observations\n\n### Stance\nI believe that identity is constructed through practice.\n";
        let positions = extract_positions(content);
        assert!(!positions.is_empty(), "Should find the position");
        assert!(
            !positions[0].has_justification,
            "No justification markers present"
        );
    }

    #[test]
    fn extract_positions_empty() {
        let positions = extract_positions("");
        assert!(positions.is_empty());
    }

    #[test]
    fn extract_positions_no_markers() {
        let content = "## Observations\n\n### Note\nThe weather was nice today. Nothing remarkable happened.\n";
        let positions = extract_positions(content);
        assert!(positions.is_empty());
    }

    #[test]
    fn positions_overlap_similar() {
        let a = trigrams("identity requires honest self-examination and genuine effort");
        let b = trigrams("identity requires honest self-examination and careful work");
        assert!(positions_overlap(&a, &b));
    }

    #[test]
    fn positions_overlap_different() {
        let a = trigrams("identity requires honest self-examination");
        let b = trigrams("the weather today was pleasant and warm");
        assert!(!positions_overlap(&a, &b));
    }

    #[test]
    fn positions_contradict_negation() {
        // Sentences must share enough trigrams (>0.25 Jaccard) and differ by negation
        let a_text = "reflection requires honest practice and honest effort every day";
        let b_text = "reflection does not require honest practice and honest effort every day";
        let a_tri = trigrams(a_text);
        let b_tri = trigrams(b_text);
        let sim = jaccard_similarity(&a_tri, &b_tri);
        assert!(sim > 0.25, "Jaccard {sim:.2} should be > 0.25 for overlap");
        assert!(positions_contradict(a_text, b_text, &a_tri, &b_tri));
    }

    #[test]
    fn positions_contradict_no_overlap() {
        let a_text = "I believe the sky is blue";
        let b_text = "I don't think databases are fast";
        let a_tri = trigrams(a_text);
        let b_tri = trigrams(b_text);
        // Different topics — not a contradiction even with negation
        assert!(!positions_contradict(a_text, b_text, &a_tri, &b_tri));
    }

    #[test]
    fn split_sentences_basic() {
        let sentences = split_sentences("First sentence here. Second sentence here. Third.");
        assert!(sentences.len() >= 2);
    }
}
