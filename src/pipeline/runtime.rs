//! Runtime pipeline health and state management for vigil-pulse.
//!
//! This module provides pipeline health calculation, state tracking,
//! document archiving, and health rendering functions that pulse-null's
//! scheduler and CLI call directly. Functions accept path and threshold
//! parameters rather than depending on pulse-null config types.

use std::path::Path;

use pulse_system_types::monitoring::{
    self as shared, DocumentCounts, DocumentHealth, PipelineHealth, PipelineState,
    PipelineThresholds, ThresholdStatus,
};

use crate::error::VpResult;

const STATE_FILENAME: &str = "pipeline-state.json";

// ---------------------------------------------------------------------------
// Types (re-exported from shared, with local helpers)
// ---------------------------------------------------------------------------

// Re-export so callers that used `runtime::Thresholds` etc. keep compiling.
// The concrete types now live in `pulse_system_types::monitoring`.

/// Convenience alias — maps to the shared type.
pub type Thresholds = PipelineThresholds;

/// Build a `DocumentHealth` from a count and its threshold pair.
fn doc_health(count: usize, soft: usize, hard: usize) -> DocumentHealth {
    let status = if count >= hard {
        ThresholdStatus::Red
    } else if count >= soft {
        ThresholdStatus::Yellow
    } else {
        ThresholdStatus::Green
    };
    DocumentHealth {
        count,
        soft,
        hard,
        status,
    }
}

// ---------------------------------------------------------------------------
// PipelineState persistence
// ---------------------------------------------------------------------------

/// Load state from `{root_dir}/pipeline-state.json`.
pub fn load_state(root_dir: &Path) -> PipelineState {
    let path = root_dir.join(STATE_FILENAME);
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        PipelineState::default()
    }
}

/// Save state to `{root_dir}/pipeline-state.json`.
pub fn save_state(state: &PipelineState, root_dir: &Path) -> VpResult<()> {
    let path = root_dir.join(STATE_FILENAME);
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Pipeline health calculation
// ---------------------------------------------------------------------------

/// Count entries in a markdown file by counting ## and ### headers
/// (excluding known structural headers).
fn count_entries(path: &Path) -> usize {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            (trimmed.starts_with("## ") || trimmed.starts_with("### "))
                && !is_structural_header(trimmed)
        })
        .count()
}

/// Headers that are document structure, not content entries.
pub fn is_structural_header(line: &str) -> bool {
    let structural = [
        "## Open Questions",
        "## Themes",
        "## Explored",
        "## Core Identity",
        "## How I Think",
        "## Moral Foundation",
        "## Philosophical Positions",
        "## Growth Log",
        "## Core Values",
        "## How I Communicate",
    ];
    structural.iter().any(|s| line.starts_with(s))
}

/// Calculate pipeline health from document files.
///
/// Reads documents from `{root_dir}/journal/` and checks counts against
/// the provided thresholds.
pub fn calculate(root_dir: &Path, thresholds: &Thresholds) -> PipelineHealth {
    let journal = root_dir.join("journal");

    let learning_count = count_entries(&journal.join("LEARNING.md"));
    let thoughts_count = count_entries(&journal.join("THOUGHTS.md"));
    let curiosity_count = count_entries(&journal.join("CURIOSITY.md"));
    let reflections_count = count_entries(&journal.join("REFLECTIONS.md"));
    let praxis_count = count_entries(&journal.join("PRAXIS.md"));

    let learning = doc_health(
        learning_count,
        thresholds.learning_soft,
        thresholds.learning_hard,
    );
    let thoughts = doc_health(
        thoughts_count,
        thresholds.thoughts_soft,
        thresholds.thoughts_hard,
    );
    let curiosity = doc_health(
        curiosity_count,
        thresholds.curiosity_soft,
        thresholds.curiosity_hard,
    );
    let reflections = doc_health(
        reflections_count,
        thresholds.reflections_soft,
        thresholds.reflections_hard,
    );
    let praxis = doc_health(praxis_count, thresholds.praxis_soft, thresholds.praxis_hard);

    let mut warnings = Vec::new();
    if learning.status == ThresholdStatus::Red {
        warnings.push(format!(
            "LEARNING at hard limit ({}/{}). Archive needed.",
            learning_count, thresholds.learning_hard
        ));
    }
    if thoughts.status == ThresholdStatus::Red {
        warnings.push(format!(
            "THOUGHTS at hard limit ({}/{}). Archive needed.",
            thoughts_count, thresholds.thoughts_hard
        ));
    }
    if curiosity.status == ThresholdStatus::Red {
        warnings.push(format!(
            "CURIOSITY at hard limit ({}/{}). Archive needed.",
            curiosity_count, thresholds.curiosity_hard
        ));
    }
    if reflections.status == ThresholdStatus::Red {
        warnings.push(format!(
            "REFLECTIONS at hard limit ({}/{}). Archive needed.",
            reflections_count, thresholds.reflections_hard
        ));
    }
    if praxis.status == ThresholdStatus::Red {
        warnings.push(format!(
            "PRAXIS at hard limit ({}/{}). Archive needed.",
            praxis_count, thresholds.praxis_hard
        ));
    }

    PipelineHealth {
        learning,
        thoughts,
        curiosity,
        reflections,
        praxis,
        warnings,
    }
}

/// Extract counts from health for state tracking.
pub fn counts_from_health(health: &PipelineHealth) -> DocumentCounts {
    DocumentCounts {
        learning: health.learning.count,
        thoughts: health.thoughts.count,
        curiosity: health.curiosity.count,
        reflections: health.reflections.count,
        praxis: health.praxis.count,
    }
}

/// Render pipeline health as text for prompt injection.
pub fn render(health: &PipelineHealth, sessions_frozen: u32, freeze_threshold: u32) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "LEARNING: {}/{} ({}) | THOUGHTS: {}/{} ({}) | CURIOSITY: {}/{} ({}) | REFLECTIONS: {}/{} ({}) | PRAXIS: {}/{} ({})",
        health.learning.count, health.learning.hard, health.learning.status,
        health.thoughts.count, health.thoughts.hard, health.thoughts.status,
        health.curiosity.count, health.curiosity.hard, health.curiosity.status,
        health.reflections.count, health.reflections.hard, health.reflections.status,
        health.praxis.count, health.praxis.hard, health.praxis.status,
    ));

    if sessions_frozen >= freeze_threshold {
        lines.push(format!(
            "FROZEN: No pipeline movement for {} sessions.",
            sessions_frozen
        ));
    }

    for warning in &health.warnings {
        lines.push(format!("Warning: {}", warning));
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Archiving
// ---------------------------------------------------------------------------

/// Check all documents and auto-archive any that hit their hard limit.
/// Returns a list of documents that were archived.
pub fn check_and_archive(
    root_dir: &Path,
    thresholds: &Thresholds,
    health: &PipelineHealth,
) -> Vec<String> {
    let mut archived = Vec::new();

    if health.learning.count >= thresholds.learning_hard
        && archive_document(root_dir, "journal/LEARNING.md", "archives/learning").is_ok()
    {
        archived.push("LEARNING.md".to_string());
    }
    if health.thoughts.count >= thresholds.thoughts_hard
        && archive_document(root_dir, "journal/THOUGHTS.md", "archives/thoughts").is_ok()
    {
        archived.push("THOUGHTS.md".to_string());
    }
    if health.curiosity.count >= thresholds.curiosity_hard
        && archive_document(root_dir, "journal/CURIOSITY.md", "archives/curiosity").is_ok()
    {
        archived.push("CURIOSITY.md".to_string());
    }
    if health.reflections.count >= thresholds.reflections_hard
        && archive_document(root_dir, "journal/REFLECTIONS.md", "archives/reflections").is_ok()
    {
        archived.push("REFLECTIONS.md".to_string());
    }
    if health.praxis.count >= thresholds.praxis_hard
        && archive_document(root_dir, "journal/PRAXIS.md", "archives/praxis").is_ok()
    {
        archived.push("PRAXIS.md".to_string());
    }

    archived
}

/// Archive a single document: move oldest entries to archive file, keep recent ones.
fn archive_document(root_dir: &Path, source_rel: &str, archive_dir_rel: &str) -> VpResult<()> {
    let source_path = root_dir.join(source_rel);
    let archive_dir = root_dir.join(archive_dir_rel);
    std::fs::create_dir_all(&archive_dir)?;

    let content = std::fs::read_to_string(&source_path)?;
    let (header, sections) = split_by_headers(&content);

    if sections.is_empty() {
        return Ok(());
    }

    let split_point = sections.len() / 2;
    let (to_archive, to_keep) = sections.split_at(split_point);

    if to_archive.is_empty() {
        return Ok(());
    }

    let date = crate::util::today_iso();
    let archive_file = archive_dir.join(format!("archive-{}.md", date));

    let archive_content = if archive_file.exists() {
        let existing = std::fs::read_to_string(&archive_file)?;
        format!("{}\n{}", existing, to_archive.join("\n"))
    } else {
        let doc_name = source_rel.rsplit('/').next().unwrap_or(source_rel);
        format!(
            "# Archive — {} ({})\n\n{}",
            doc_name,
            date,
            to_archive.join("\n")
        )
    };
    std::fs::write(&archive_file, archive_content)?;

    let new_content = format!("{}\n{}", header, to_keep.join("\n"));
    std::fs::write(&source_path, new_content)?;

    Ok(())
}

/// Manually archive a specific document (for CLI use).
pub fn archive_document_by_name(root_dir: &Path, document: &str) -> VpResult<String> {
    let (source, archive_dir) = match document.to_lowercase().as_str() {
        "learning" => ("journal/LEARNING.md", "archives/learning"),
        "thoughts" => ("journal/THOUGHTS.md", "archives/thoughts"),
        "curiosity" => ("journal/CURIOSITY.md", "archives/curiosity"),
        "reflections" => ("journal/REFLECTIONS.md", "archives/reflections"),
        "praxis" => ("journal/PRAXIS.md", "archives/praxis"),
        _ => {
            return Err(crate::error::VpError::Pipeline(format!(
                "Unknown document: {}. Valid: learning, thoughts, curiosity, reflections, praxis",
                document
            )))
        }
    };

    archive_document(root_dir, source, archive_dir)?;
    Ok(format!("Archived entries from {}", source))
}

/// Split markdown content into a header (everything before first ##) and sections.
fn split_by_headers(content: &str) -> (String, Vec<String>) {
    let mut header = String::new();
    let mut sections: Vec<String> = Vec::new();
    let mut current_section = String::new();
    let mut in_header = true;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if (trimmed.starts_with("## ") || trimmed.starts_with("### "))
            && !is_structural_header(trimmed)
        {
            if in_header {
                in_header = false;
            } else if !current_section.is_empty() {
                sections.push(current_section.clone());
            }
            current_section = format!("{}\n", line);
        } else if in_header {
            header.push_str(line);
            header.push('\n');
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    if !current_section.is_empty() {
        sections.push(current_section);
    }

    (header, sections)
}

/// List archived files for a document type.
pub fn list_archives(root_dir: &Path, document: Option<&str>) -> VpResult<Vec<String>> {
    let dirs: Vec<&str> = match document {
        Some(d) => match d.to_lowercase().as_str() {
            "learning" => vec!["archives/learning"],
            "thoughts" => vec!["archives/thoughts"],
            "curiosity" => vec!["archives/curiosity"],
            "reflections" => vec!["archives/reflections"],
            "praxis" => vec!["archives/praxis"],
            _ => {
                return Err(crate::error::VpError::Pipeline(format!(
                    "Unknown document: {}",
                    d
                )))
            }
        },
        None => vec![
            "archives/learning",
            "archives/thoughts",
            "archives/curiosity",
            "archives/reflections",
            "archives/praxis",
        ],
    };

    let mut files = Vec::new();
    for dir in dirs {
        let path = root_dir.join(dir);
        if path.exists() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".md") {
                            files.push(format!("{}/{}", dir, name));
                        }
                    }
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

// ---------------------------------------------------------------------------
// Trait implementation: PipelineMonitor
// ---------------------------------------------------------------------------

/// Concrete implementation of the PipelineMonitor trait.
///
/// pulse-null core creates this and stores it as `Arc<dyn PipelineMonitor>`.
/// All existing functions are preserved for standalone CLI use.
pub struct PraxisMonitor;

impl PraxisMonitor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PraxisMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl shared::PipelineMonitor for PraxisMonitor {
    fn calculate(&self, root_dir: &Path, thresholds: &PipelineThresholds) -> PipelineHealth {
        calculate(root_dir, thresholds)
    }

    fn render_for_prompt(
        &self,
        health: &PipelineHealth,
        sessions_frozen: u32,
        freeze_threshold: u32,
    ) -> String {
        render(health, sessions_frozen, freeze_threshold)
    }

    fn counts_from_health(&self, health: &PipelineHealth) -> DocumentCounts {
        counts_from_health(health)
    }

    fn load_state(&self, root_dir: &Path) -> PipelineState {
        load_state(root_dir)
    }

    fn save_state(
        &self,
        root_dir: &Path,
        state: &PipelineState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        save_state(state, root_dir).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn check_and_archive(
        &self,
        root_dir: &Path,
        thresholds: &PipelineThresholds,
        health: &PipelineHealth,
    ) -> Vec<String> {
        check_and_archive(root_dir, thresholds, health)
    }

    fn list_archives(
        &self,
        root_dir: &Path,
        document: Option<&str>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        list_archives(root_dir, document).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn archive_by_name(
        &self,
        root_dir: &Path,
        document: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        archive_document_by_name(root_dir, document)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_journal(dir: &Path, filename: &str, content: &str) {
        let journal = dir.join("journal");
        fs::create_dir_all(&journal).unwrap();
        fs::write(journal.join(filename), content).unwrap();
    }

    #[test]
    fn test_count_entries_empty() {
        let dir = TempDir::new().unwrap();
        setup_journal(dir.path(), "LEARNING.md", "# Learning\n\nEmpty doc.\n");
        let count = count_entries(&dir.path().join("journal/LEARNING.md"));
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_entries_with_headers() {
        let dir = TempDir::new().unwrap();
        setup_journal(
            dir.path(),
            "THOUGHTS.md",
            "# Thoughts\n\n## First thought\n\nContent.\n\n## Second thought\n\nMore content.\n\n### Sub-thought\n\nDetail.\n",
        );
        let count = count_entries(&dir.path().join("journal/THOUGHTS.md"));
        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_entries_skips_structural() {
        let dir = TempDir::new().unwrap();
        setup_journal(
            dir.path(),
            "CURIOSITY.md",
            "# Curiosity\n\n## Open Questions\n\n### What is X?\n\n### What is Y?\n\n## Themes\n\n## Explored\n\n### Old question\n",
        );
        let count = count_entries(&dir.path().join("journal/CURIOSITY.md"));
        assert_eq!(count, 3);
    }

    #[test]
    fn test_threshold_status() {
        let green = doc_health(3, 5, 8);
        assert_eq!(green.status, ThresholdStatus::Green);

        let yellow = doc_health(5, 5, 8);
        assert_eq!(yellow.status, ThresholdStatus::Yellow);

        let red = doc_health(8, 5, 8);
        assert_eq!(red.status, ThresholdStatus::Red);
    }

    #[test]
    fn test_calculate_health() {
        let dir = TempDir::new().unwrap();
        let journal = dir.path().join("journal");
        fs::create_dir_all(&journal).unwrap();
        fs::write(
            journal.join("LEARNING.md"),
            "# Learning\n\n## Topic 1\n\n## Topic 2\n",
        )
        .unwrap();
        fs::write(journal.join("THOUGHTS.md"), "# Thoughts\n").unwrap();
        fs::write(
            journal.join("CURIOSITY.md"),
            "# Curiosity\n\n## Open Questions\n\n## Themes\n\n## Explored\n",
        )
        .unwrap();
        fs::write(journal.join("REFLECTIONS.md"), "# Reflections\n").unwrap();
        fs::write(journal.join("PRAXIS.md"), "# Praxis\n").unwrap();

        let thresholds = Thresholds::default();
        let health = calculate(dir.path(), &thresholds);

        assert_eq!(health.learning.count, 2);
        assert_eq!(health.learning.status, ThresholdStatus::Green);
        assert_eq!(health.thoughts.count, 0);
        assert_eq!(health.curiosity.count, 0);
        assert!(health.warnings.is_empty());
    }

    #[test]
    fn test_pipeline_state_load_save() {
        let dir = TempDir::new().unwrap();

        let mut state = load_state(dir.path());
        assert_eq!(state.session_count, 0);

        let counts = DocumentCounts {
            learning: 2,
            thoughts: 1,
            curiosity: 0,
            reflections: 3,
            praxis: 1,
        };
        state.update_counts(&counts, &crate::util::now_iso());
        save_state(&state, dir.path()).unwrap();

        let loaded = load_state(dir.path());
        assert_eq!(loaded.session_count, 1);
        assert_eq!(loaded.last_counts, counts);
        assert_eq!(loaded.sessions_without_movement, 0);
    }

    #[test]
    fn test_pipeline_state_detects_no_movement() {
        let mut state = PipelineState::default();
        let counts = DocumentCounts {
            learning: 2,
            ..Default::default()
        };
        state.update_counts(&counts, &crate::util::now_iso());
        assert_eq!(state.sessions_without_movement, 0);

        state.update_counts(&counts, &crate::util::now_iso());
        assert_eq!(state.sessions_without_movement, 1);

        state.update_counts(&counts, &crate::util::now_iso());
        assert_eq!(state.sessions_without_movement, 2);

        let new_counts = DocumentCounts {
            learning: 3,
            ..Default::default()
        };
        state.update_counts(&new_counts, &crate::util::now_iso());
        assert_eq!(state.sessions_without_movement, 0);
    }

    #[test]
    fn test_split_by_headers() {
        let content =
            "# Title\n\nPreamble.\n\n## Entry 1\n\nContent 1.\n\n## Entry 2\n\nContent 2.\n";
        let (header, sections) = split_by_headers(content);
        assert!(header.contains("Title"));
        assert!(header.contains("Preamble"));
        assert_eq!(sections.len(), 2);
        assert!(sections[0].contains("Entry 1"));
        assert!(sections[1].contains("Entry 2"));
    }

    #[test]
    fn test_archive_document() {
        let dir = TempDir::new().unwrap();
        let journal = dir.path().join("journal");
        let archives = dir.path().join("archives/learning");
        fs::create_dir_all(&journal).unwrap();
        fs::create_dir_all(&archives).unwrap();

        fs::write(
            journal.join("LEARNING.md"),
            "# Learning\n\nPreamble.\n\n## Topic 1\n\nOld content.\n\n## Topic 2\n\nOlder content.\n\n## Topic 3\n\nNew content.\n\n## Topic 4\n\nNewest content.\n",
        ).unwrap();

        archive_document(dir.path(), "journal/LEARNING.md", "archives/learning").unwrap();

        let remaining = fs::read_to_string(journal.join("LEARNING.md")).unwrap();
        let (_, sections) = split_by_headers(&remaining);
        assert_eq!(sections.len(), 2);

        let archive_files: Vec<_> = fs::read_dir(&archives).unwrap().flatten().collect();
        assert_eq!(archive_files.len(), 1);
    }

    #[test]
    fn test_list_archives_empty() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("archives/learning")).unwrap();
        let files = list_archives(dir.path(), Some("learning")).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_render_output() {
        let health = PipelineHealth {
            learning: doc_health(3, 5, 8),
            thoughts: doc_health(5, 5, 10),
            curiosity: doc_health(2, 3, 7),
            reflections: doc_health(10, 15, 20),
            praxis: doc_health(3, 5, 10),
            warnings: Vec::new(),
        };
        let text = render(&health, 0, 3);
        assert!(text.contains("LEARNING: 3/8"));
        assert!(text.contains("THOUGHTS: 5/10"));
        assert!(!text.contains("FROZEN"));
    }

    #[test]
    fn test_render_frozen() {
        let health = PipelineHealth {
            learning: doc_health(0, 5, 8),
            thoughts: doc_health(0, 5, 10),
            curiosity: doc_health(0, 3, 7),
            reflections: doc_health(0, 15, 20),
            praxis: doc_health(0, 5, 10),
            warnings: Vec::new(),
        };
        let text = render(&health, 4, 3);
        assert!(text.contains("FROZEN"));
        assert!(text.contains("4 sessions"));
    }

    #[test]
    fn test_counts_from_health() {
        let health = PipelineHealth {
            learning: doc_health(2, 5, 8),
            thoughts: doc_health(3, 5, 10),
            curiosity: doc_health(1, 3, 7),
            reflections: doc_health(5, 15, 20),
            praxis: doc_health(2, 5, 10),
            warnings: Vec::new(),
        };
        let counts = counts_from_health(&health);
        assert_eq!(counts.learning, 2);
        assert_eq!(counts.thoughts, 3);
        assert_eq!(counts.curiosity, 1);
        assert_eq!(counts.reflections, 5);
        assert_eq!(counts.praxis, 2);
    }
}
