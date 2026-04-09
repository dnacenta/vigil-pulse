use std::collections::HashMap;

use crate::util::{count_h3_under_section, hash_content, read_or_empty};

/// Counts of items in a pipeline document.
#[derive(Default, Debug, Clone)]
pub struct DocCounts {
    pub active: usize,
    pub graduated: usize,
    pub dissolved: usize,
    pub explored: usize,
    pub total: usize,
}

/// A thought entry with its last-touched date.
#[derive(Debug, Clone)]
pub struct ThoughtEntry {
    pub title: String,
    pub last_touched: Option<String>,
    pub started: Option<String>,
}

/// Result of scanning all pipeline documents.
#[derive(Default, Debug, Clone)]
pub struct PipelineScan {
    pub learning: DocCounts,
    pub thoughts: DocCounts,
    pub curiosity: DocCounts,
    pub reflections: DocCounts,
    pub praxis: DocCounts,
    pub session_log_entries: usize,
    pub session_log_oldest: Option<String>,
    pub session_log_newest: Option<String>,
    pub stale_thoughts: Vec<ThoughtEntry>,
    pub document_hashes: HashMap<String, String>,
}

/// Extract thoughts with their dates from THOUGHTS.md.
fn parse_thoughts(content: &str) -> Vec<ThoughtEntry> {
    let mut entries = Vec::new();
    let mut in_active = false;
    let mut current_title: Option<String> = None;
    let mut current_last_touched: Option<String> = None;
    let mut current_started: Option<String> = None;

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous entry
            if let Some(title) = current_title.take() {
                entries.push(ThoughtEntry {
                    title,
                    last_touched: current_last_touched.take(),
                    started: current_started.take(),
                });
            }
            let heading = line.trim_start_matches("## ").trim().to_lowercase();
            in_active = heading.contains("active");
        } else if in_active && line.starts_with("### ") {
            // Save previous entry
            if let Some(title) = current_title.take() {
                entries.push(ThoughtEntry {
                    title,
                    last_touched: current_last_touched.take(),
                    started: current_started.take(),
                });
            }
            current_title = Some(line.trim_start_matches("### ").trim().to_string());
        } else if current_title.is_some() {
            if let Some(date) = extract_date_field(line, "Last touched") {
                current_last_touched = Some(date);
            }
            if let Some(date) = extract_date_field(line, "started") {
                current_started = Some(date);
            }
            // Also check for "Started:" with capital S
            if let Some(date) = extract_date_field(line, "Started") {
                current_started = Some(date);
            }
        }
    }
    // Don't forget the last entry
    if let Some(title) = current_title {
        entries.push(ThoughtEntry {
            title,
            last_touched: current_last_touched,
            started: current_started,
        });
    }
    entries
}

/// Extract a date from a line like "**Last touched**: 2026-02-26" or "started 2026-02-26"
fn extract_date_field(line: &str, field: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let field_lower = field.to_lowercase();

    // Match patterns like "**Last touched**: 2026-02-26" or "Last touched: 2026-02-26"
    if lower.contains(&field_lower) {
        // Find a date pattern (YYYY-MM-DD) after the field name
        let after_field = if let Some(idx) = lower.find(&field_lower) {
            &line[idx + field.len()..]
        } else {
            return None;
        };
        return find_date_in_str(after_field);
    }
    None
}

/// Find first YYYY-MM-DD pattern in a string.
fn find_date_in_str(s: &str) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    for i in 0..chars.len() {
        if chars[i].is_ascii_digit() {
            let rest: String = chars[i..].iter().collect();
            if rest.len() >= 10
                && rest[4..5] == *"-"
                && rest[7..8] == *"-"
                && rest[..4].chars().all(|c| c.is_ascii_digit())
                && rest[5..7].chars().all(|c| c.is_ascii_digit())
                && rest[8..10].chars().all(|c| c.is_ascii_digit())
            {
                return Some(rest[..10].to_string());
            }
        }
    }
    None
}

/// Count ### headings under a section, also try matching "Open" for "Open Questions"
fn count_open_questions(content: &str) -> usize {
    let c1 = count_h3_under_section(content, "Open Questions");
    if c1 > 0 {
        return c1;
    }
    count_h3_under_section(content, "Open")
}

fn count_explored(content: &str) -> usize {
    let c1 = count_h3_under_section(content, "Explored");
    if c1 > 0 {
        return c1;
    }
    count_h3_under_section(content, "Explored Questions")
}

/// Count reflection log entries (### headings under ## Sessions or any ### heading).
fn count_log_entries(content: &str) -> (usize, Option<String>, Option<String>) {
    let mut count = 0;
    let mut oldest: Option<String> = None;
    let mut newest: Option<String> = None;

    for line in content.lines() {
        if line.starts_with("### ") {
            count += 1;
            if let Some(date) = find_date_in_str(line) {
                if oldest.is_none() || oldest.as_ref().is_some_and(|o| date < *o) {
                    oldest = Some(date.clone());
                }
                if newest.is_none() || newest.as_ref().is_some_and(|n| date > *n) {
                    newest = Some(date);
                }
            }
        }
    }
    (count, oldest, newest)
}

/// Default staleness threshold in days.
const DEFAULT_STALENESS_DAYS: u32 = 7;

/// All paths needed to scan the pipeline documents.
pub struct ScanPaths {
    pub learning: std::path::PathBuf,
    pub thoughts: std::path::PathBuf,
    pub curiosity: std::path::PathBuf,
    pub reflections: std::path::PathBuf,
    pub praxis: std::path::PathBuf,
    pub self_md: std::path::PathBuf,
    pub log: std::path::PathBuf,
}

/// Perform a full scan of all pipeline documents.
pub fn scan(paths: &ScanPaths) -> PipelineScan {
    scan_with_staleness(paths, DEFAULT_STALENESS_DAYS)
}

/// Perform a full scan with configurable staleness threshold.
pub fn scan_with_staleness(paths: &ScanPaths, staleness_days: u32) -> PipelineScan {
    let learning_content = read_or_empty(&paths.learning);
    let thoughts_content = read_or_empty(&paths.thoughts);
    let curiosity_content = read_or_empty(&paths.curiosity);
    let reflections_content = read_or_empty(&paths.reflections);
    let praxis_content = read_or_empty(&paths.praxis);
    let self_content = read_or_empty(&paths.self_md);
    let log_content = read_or_empty(&paths.log);

    // Learning
    let learning_active = count_h3_under_section(&learning_content, "Active");
    let learning_active = if learning_active == 0 {
        count_h3_under_section(&learning_content, "Active Threads")
    } else {
        learning_active
    };

    // Thoughts
    let thoughts_active = count_h3_under_section(&thoughts_content, "Active");
    let thoughts_graduated = count_h3_under_section(&thoughts_content, "Graduated");
    let thoughts_dissolved = count_h3_under_section(&thoughts_content, "Dissolved");
    let thought_entries = parse_thoughts(&thoughts_content);

    // Detect stale thoughts (untouched > staleness_days)
    let today = super::state::today_iso();
    let stale_thoughts: Vec<ThoughtEntry> = thought_entries
        .into_iter()
        .filter(|t| {
            let date = t.last_touched.as_ref().or(t.started.as_ref());
            if let Some(d) = date {
                days_between(d, &today) > staleness_days as i64
            } else {
                false
            }
        })
        .collect();

    // Curiosity
    let curiosity_open = count_open_questions(&curiosity_content);
    let curiosity_explored = count_explored(&curiosity_content);

    // Reflections
    let reflections_obs = count_h3_under_section(&reflections_content, "Observations");
    let reflections_pat = count_h3_under_section(&reflections_content, "Patterns");
    let reflections_les = count_h3_under_section(&reflections_content, "Lessons");
    let reflections_total = reflections_obs + reflections_pat + reflections_les;

    // Praxis
    let praxis_active = count_h3_under_section(&praxis_content, "Active");
    let praxis_retired = count_h3_under_section(&praxis_content, "Retired");

    // Reflection log
    let (log_entries, log_oldest, log_newest) = count_log_entries(&log_content);

    // Document hashes
    let mut hashes = HashMap::new();
    hashes.insert("learning".to_string(), hash_content(&learning_content));
    hashes.insert("thoughts".to_string(), hash_content(&thoughts_content));
    hashes.insert("curiosity".to_string(), hash_content(&curiosity_content));
    hashes.insert(
        "reflections".to_string(),
        hash_content(&reflections_content),
    );
    hashes.insert("praxis".to_string(), hash_content(&praxis_content));
    hashes.insert("self".to_string(), hash_content(&self_content));

    PipelineScan {
        learning: DocCounts {
            active: learning_active,
            ..Default::default()
        },
        thoughts: DocCounts {
            active: thoughts_active,
            graduated: thoughts_graduated,
            dissolved: thoughts_dissolved,
            ..Default::default()
        },
        curiosity: DocCounts {
            active: curiosity_open,
            explored: curiosity_explored,
            ..Default::default()
        },
        reflections: DocCounts {
            active: reflections_obs,
            total: reflections_total,
            ..Default::default()
        },
        praxis: DocCounts {
            active: praxis_active,
            graduated: praxis_retired,
            ..Default::default()
        },
        session_log_entries: log_entries,
        session_log_oldest: log_oldest,
        session_log_newest: log_newest,
        stale_thoughts,
        document_hashes: hashes,
    }
}

/// Day difference between two YYYY-MM-DD strings using chrono.
pub fn days_between(earlier: &str, later: &str) -> i64 {
    let e = parse_naive_date(earlier);
    let l = parse_naive_date(later);
    match (e, l) {
        (Some(e), Some(l)) => (l - e).num_days(),
        _ => 0,
    }
}

fn parse_naive_date(date: &str) -> Option<chrono::NaiveDate> {
    if date.len() < 10 {
        return None;
    }
    chrono::NaiveDate::parse_from_str(&date[..10], "%Y-%m-%d").ok()
}

/// Build `ScanPaths` from a `PraxisConfig`.
pub fn paths_from_config(config: &super::PraxisConfig) -> ScanPaths {
    ScanPaths {
        learning: super::learning_file(&config.docs_dir),
        thoughts: super::thoughts_file(&config.docs_dir),
        curiosity: super::curiosity_file(&config.docs_dir),
        reflections: super::reflections_file(&config.docs_dir),
        praxis: super::praxis_file(&config.docs_dir),
        self_md: super::self_file(&config.docs_dir),
        log: super::session_log_file(&config.docs_dir),
    }
}

/// Convenience: scan using config paths.
pub fn scan_with_config(config: &super::PraxisConfig) -> PipelineScan {
    scan_with_config_and_staleness(config, DEFAULT_STALENESS_DAYS)
}

/// Convenience: scan using config paths with configurable staleness.
pub fn scan_with_config_and_staleness(
    config: &super::PraxisConfig,
    staleness_days: u32,
) -> PipelineScan {
    scan_with_staleness(&paths_from_config(config), staleness_days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_h3_under_section() {
        let content = "## Active\n\n### Thought One\nSome text\n\n### Thought Two\nMore text\n\n## Graduated\n\n### Old One\n";
        assert_eq!(count_h3_under_section(content, "Active"), 2);
        assert_eq!(count_h3_under_section(content, "Graduated"), 1);
    }

    #[test]
    fn extracts_date_field() {
        let line = "**Last touched**: 2026-02-26";
        assert_eq!(
            extract_date_field(line, "Last touched"),
            Some("2026-02-26".to_string())
        );
    }

    #[test]
    fn finds_date_in_string() {
        assert_eq!(
            find_date_in_str(": 2026-02-26 some text"),
            Some("2026-02-26".to_string())
        );
        assert_eq!(find_date_in_str("no date here"), None);
    }

    #[test]
    fn parses_thoughts_with_dates() {
        let content = "## Active\n\n### The risk of mechanical reflection\n**Started**: 2026-02-20\n**Last touched**: 2026-02-22\n\nSome content.\n\n### Another thought\n**Started**: 2026-02-26\n";
        let thoughts = parse_thoughts(content);
        assert_eq!(thoughts.len(), 2);
        assert_eq!(thoughts[0].title, "The risk of mechanical reflection");
        assert_eq!(thoughts[0].last_touched, Some("2026-02-22".to_string()));
        assert_eq!(thoughts[1].title, "Another thought");
        assert_eq!(thoughts[1].started, Some("2026-02-26".to_string()));
    }

    #[test]
    fn days_between_dates() {
        assert!(days_between("2026-02-20", "2026-02-27") > 0);
        assert_eq!(days_between("2026-02-20", "2026-02-20"), 0);
    }

    #[test]
    fn staleness_threshold_is_configurable() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        // Create a thoughts file with one thought dated 5 days ago
        let thoughts_path = dir.path().join("THOUGHTS.md");
        let today = super::super::state::today_iso(); // "YYYY-MM-DD"
        let five_days_ago = {
            // Subtract ~5 days using rough date math
            let y: i64 = today[..4].parse().unwrap();
            let m: i64 = today[5..7].parse().unwrap();
            let d: i64 = today[8..10].parse().unwrap();
            let new_d = d - 5;
            if new_d > 0 {
                format!("{:04}-{:02}-{:02}", y, m, new_d)
            } else {
                // Roll back month (rough, good enough for test)
                format!("{:04}-{:02}-{:02}", y, m - 1, 25)
            }
        };
        let mut f = std::fs::File::create(&thoughts_path).unwrap();
        writeln!(
            f,
            "## Active\n\n### Old thought\n**Started**: {}\n",
            five_days_ago
        )
        .unwrap();

        let empty = dir.path().join("EMPTY.md");
        std::fs::write(&empty, "").unwrap();

        let paths = ScanPaths {
            learning: empty.clone(),
            thoughts: thoughts_path,
            curiosity: empty.clone(),
            reflections: empty.clone(),
            praxis: empty.clone(),
            self_md: empty.clone(),
            log: empty,
        };

        // With default 7-day staleness: not stale
        let result = scan_with_staleness(&paths, 7);
        assert!(
            result.stale_thoughts.is_empty(),
            "5-day-old thought should not be stale at 7-day threshold"
        );

        // With 3-day staleness: stale
        let result = scan_with_staleness(&paths, 3);
        assert!(
            !result.stale_thoughts.is_empty(),
            "5-day-old thought should be stale at 3-day threshold"
        );
    }
}
