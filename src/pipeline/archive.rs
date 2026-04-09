use std::fs;

use super::parser;
use super::state;
use super::PraxisConfig;
use crate::error::VpResult;

const BOLD: &str = "\x1b[1m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

struct ArchiveCandidate {
    document: String,
    current: usize,
    hard_limit: usize,
    overflow: usize,
}

pub fn run(config: &PraxisConfig, dry_run: bool) -> VpResult<()> {
    let scan = parser::scan_with_config(config);

    let mut candidates: Vec<ArchiveCandidate> = Vec::new();
    let t = &config.thresholds;

    // Check each document against hard limits
    let checks: &[(&str, usize, usize)] = &[
        ("LEARNING.md", scan.learning.active, t.learning_hard),
        ("THOUGHTS.md", scan.thoughts.active, t.thoughts_hard),
        ("CURIOSITY.md", scan.curiosity.active, t.curiosity_hard),
        ("REFLECTIONS.md", scan.reflections.total, t.reflections_hard),
        ("PRAXIS.md", scan.praxis.active, t.praxis_hard),
    ];

    for &(document, current, hard_limit) in checks {
        if current >= hard_limit {
            candidates.push(ArchiveCandidate {
                document: document.to_string(),
                current,
                hard_limit,
                overflow: current - hard_limit,
            });
        }
    }

    if candidates.is_empty() {
        println!("{GREEN}✓{RESET} All documents within thresholds. Nothing to archive.");
        return Ok(());
    }

    let mode = if dry_run { "DRY RUN" } else { "ARCHIVE" };
    println!("\n{BOLD}praxis-echo{RESET} — {mode}\n");

    for c in &candidates {
        println!(
            "  {YELLOW}⚠{RESET} {}: {}/{} ({} over limit)",
            c.document, c.current, c.hard_limit, c.overflow
        );
    }

    if dry_run {
        println!("\n  {YELLOW}Dry run{RESET} — no changes made.");
        println!("  Run without --dry-run to archive overflow content.");
        println!();
        return Ok(());
    }

    // Create archive directories and placeholder files
    let archives = super::archives_dir(&config.docs_dir);
    let today = state::today_iso();

    for c in &candidates {
        let sub = match c.document.as_str() {
            "LEARNING.md" => "learning",
            "THOUGHTS.md" => "thoughts",
            "CURIOSITY.md" => "curiosity",
            "REFLECTIONS.md" => "reflections",
            "PRAXIS.md" => "praxis",
            _ => continue,
        };
        let archive_dir = archives.join(sub);
        if !archive_dir.exists() {
            fs::create_dir_all(&archive_dir)?;
        }

        // Create an archive marker file — actual content migration is manual
        // because automatically removing markdown sections is fragile
        let marker = archive_dir.join(format!("archive-needed-{today}.md"));
        let content = format!(
            "# Archive Needed — {}\n\n\
             Date: {today}\n\
             Current count: {}\n\
             Hard limit: {}\n\
             Overflow: {} items need archiving\n\n\
             Review the document and move the oldest/most integrated items here.\n",
            c.document, c.current, c.hard_limit, c.overflow
        );
        fs::write(&marker, content)?;
        println!(
            "  {GREEN}✓{RESET} Created archive marker: {}",
            marker.display()
        );
    }

    // Update state
    let mut st = state::load(config)?;
    st.last_archive = Some(state::now_iso());
    st.pipeline.total_archival_ops += 1;
    state::save(&st, config)?;

    println!("\n  Archive markers created. Review each document and move overflow content.\n");

    Ok(())
}
