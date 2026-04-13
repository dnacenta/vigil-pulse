//! Shared utility functions used across pipeline, reflection, and outcome modules.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Timestamp helpers (no chrono dependency)
// ---------------------------------------------------------------------------

/// Current UTC time as an ISO 8601 string.
pub fn now_iso() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    let (year, month, day) = days_to_date(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Today's date as `YYYY-MM-DD`.
pub fn today_iso() -> String {
    let ts = now_iso();
    ts[..10].to_string()
}

/// Current epoch seconds.
pub fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Convert days since Unix epoch to (year, month, day).
///
/// Algorithm from <http://howardhinnant.github.io/date_algorithms.html>.
pub fn days_to_date(days_since_epoch: u64) -> (u64, u64, u64) {
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// Convert (year, month, day) to days since Unix epoch.
pub fn date_to_days(year: u64, month: u64, day: u64) -> u64 {
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Parse an ISO 8601 timestamp (`YYYY-MM-DDThh:mm:ssZ`) to epoch seconds.
pub fn parse_iso_epoch(ts: &str) -> Option<u64> {
    if ts.len() < 19 {
        return None;
    }
    let year: u64 = ts[..4].parse().ok()?;
    let month: u64 = ts[5..7].parse().ok()?;
    let day: u64 = ts[8..10].parse().ok()?;
    let hours: u64 = ts[11..13].parse().ok()?;
    let minutes: u64 = ts[14..16].parse().ok()?;
    let seconds: u64 = ts[17..19].parse().ok()?;

    let days = date_to_days(year, month, day);
    Some(days * 86400 + hours * 3600 + minutes * 60 + seconds)
}

// ---------------------------------------------------------------------------
// Markdown helpers
// ---------------------------------------------------------------------------

/// Count `###` headings under a specific `##` section.
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

// ---------------------------------------------------------------------------
// File / hashing helpers
// ---------------------------------------------------------------------------

/// Read a file to string, returning an empty string on error.
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

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Signal constants
// ---------------------------------------------------------------------------

/// Canonical list of tracked cognitive signal names.
pub const SIGNAL_NAMES: [&str; 8] = [
    "vocabulary_diversity",
    "question_generation",
    "thought_lifecycle",
    "evidence_grounding",
    "conclusion_novelty",
    "intellectual_honesty",
    "position_delta",
    "comfort_index",
];

/// Human-friendly signal name.
pub fn friendly_name(name: &str) -> &str {
    match name {
        "vocabulary_diversity" => "vocabulary diversity",
        "question_generation" => "question generation",
        "thought_lifecycle" => "thought lifecycle",
        "evidence_grounding" => "evidence grounding",
        "conclusion_novelty" => "conclusion novelty",
        "intellectual_honesty" => "intellectual honesty",
        "position_delta" => "position delta",
        "comfort_index" => "comfort index",
        _ => name,
    }
}

/// Extract a signal value by name from a Signals struct.
pub fn get_signal(signals: &crate::reflection::state::Signals, name: &str) -> Option<f64> {
    match name {
        "vocabulary_diversity" => signals.vocabulary_diversity,
        "question_generation" => signals.question_generation,
        "thought_lifecycle" => signals.thought_lifecycle,
        "evidence_grounding" => signals.evidence_grounding,
        "conclusion_novelty" => signals.conclusion_novelty,
        "intellectual_honesty" => signals.intellectual_honesty,
        "position_delta" => signals.position_delta,
        "comfort_index" => signals.comfort_index,
        _ => None,
    }
}
