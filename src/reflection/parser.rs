use std::collections::HashSet;

// Re-export shared helpers so existing callers like `parser::read_or_empty()` keep working.
pub use crate::util::{count_h3_under_section, hash_content, read_or_empty};

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
    if !entries.is_empty() {
        return entries;
    }
    // Fallback: treat each ## heading as an entry (flat format like R-NNN:)
    extract_entries_flat(content)
}

/// Flat mode: treat each ## heading as an entry.
fn extract_entries_flat(content: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_body = String::new();
    for line in content.lines() {
        if line.starts_with("## ") {
            if let Some(title) = current_title.take() {
                entries.push((title, std::mem::take(&mut current_body)));
            }
            current_title = Some(line.trim_start_matches("## ").trim().to_string());
            current_body.clear();
        } else if current_title.is_some() && !line.trim().is_empty() {
            current_body.push_str(line);
            current_body.push(' ');
        }
    }
    if let Some(title) = current_title {
        entries.push((title, current_body));
    }
    entries
}

// ---------------------------------------------------------------------------
// Phase 2 helpers
// ---------------------------------------------------------------------------

/// Extract conclusion text from entries (after "So what?" marker or last sentence).
pub fn extract_conclusions(content: &str) -> Vec<String> {
    let entries = extract_entries(content, &["observations", "patterns", "lessons"]);
    let mut conclusions = Vec::new();
    for (_title, body) in &entries {
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
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            let last = trimmed
                .rsplit_once(". ")
                .map(|(_, l)| l.trim().to_string())
                .unwrap_or_else(|| trimmed.to_string());
            if !last.is_empty() {
                conclusions.push(last);
            }
        }
    }
    conclusions
}

/// Build trigram set from text.
pub fn trigrams(text: &str) -> HashSet<String> {
    let words = tokenize(text);
    let mut set = HashSet::new();
    if words.len() < 3 {
        if words.len() == 2 {
            set.insert(format!("{} {}", words[0], words[1]));
        } else if words.len() == 1 {
            set.insert(words[0].clone());
        }
        return set;
    }
    for w in words.windows(3) {
        set.insert(format!("{} {} {}", w[0], w[1], w[2]));
    }
    set
}

/// Jaccard similarity between two sets.
pub fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    inter as f64 / union as f64
}

/// Position statement extracted from reflections.
#[derive(Debug, Clone)]
pub struct PositionStatement {
    pub entry_title: String,
    pub text: String,
    pub trigrams: HashSet<String>,
    pub has_justification: bool,
}

const POSITION_MARKERS: &[&str] = &[
    "i believe",
    "i've concluded",
    "my position is",
    "my stance is",
    "i'm convinced",
    "i hold that",
    "i maintain that",
    "i now think",
    "i've decided",
    "in my view",
    "the key insight is",
    "what matters most is",
];

const JUSTIFICATION_MARKERS: &[&str] = &[
    "because",
    "since",
    "given that",
    "the evidence",
    "this is because",
    "after considering",
    "after reflecting",
    "i reconsidered",
    "on reflection",
    "the data shows",
    "experience shows",
    "d said",
    "d pointed out",
];

/// Extract position statements from reflections.
pub fn extract_positions(content: &str) -> Vec<PositionStatement> {
    let entries = extract_entries(content, &["observations", "patterns", "lessons"]);
    let mut positions = Vec::new();
    for (title, body) in &entries {
        let lower = body.to_lowercase();
        let body_justified = JUSTIFICATION_MARKERS.iter().any(|m| lower.contains(m));
        for sentence in split_sentences(body) {
            let sl = sentence.to_lowercase();
            if POSITION_MARKERS.iter().any(|m| sl.contains(m)) {
                let tri = trigrams(&sentence);
                if !tri.is_empty() {
                    let sent_justified = JUSTIFICATION_MARKERS.iter().any(|m| sl.contains(m));
                    positions.push(PositionStatement {
                        entry_title: title.clone(),
                        text: sentence.trim().to_string(),
                        trigrams: tri,
                        has_justification: sent_justified || body_justified,
                    });
                }
            }
        }
    }
    positions
}

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

pub fn positions_overlap(a: &HashSet<String>, b: &HashSet<String>) -> bool {
    jaccard_similarity(a, b) > 0.25
}

pub fn positions_contradict(
    a_text: &str,
    b_text: &str,
    a_tri: &HashSet<String>,
    b_tri: &HashSet<String>,
) -> bool {
    if !positions_overlap(a_tri, b_tri) {
        return false;
    }
    let (al, bl) = (a_text.to_lowercase(), b_text.to_lowercase());
    let neg = [
        "not ",
        "don't",
        "doesn't",
        "isn't",
        "won't",
        "can't",
        "cannot",
        "never",
        "no longer",
        "disagree",
    ];
    let a_neg = neg.iter().any(|m| al.contains(m));
    let b_neg = neg.iter().any(|m| bl.contains(m));
    a_neg != b_neg
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
}
