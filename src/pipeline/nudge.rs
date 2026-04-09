use std::fs;

use super::state;
use super::PraxisConfig;
use crate::error::{VpError, VpResult};

const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

const MAX_PENDING: usize = 10;

pub fn run(config: &PraxisConfig, topic: &str, when: &str, priority: &str) -> VpResult<()> {
    let queue_path = super::intent_queue_file(&config.docs_dir);

    // Load existing queue
    let mut queue: Vec<serde_json::Value> = if queue_path.exists() {
        let content = fs::read_to_string(&queue_path)?;
        if content.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&content)?
        }
    } else {
        Vec::new()
    };

    // Count pending intents
    let pending = queue
        .iter()
        .filter(|i| {
            i.get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("pending")
                == "pending"
        })
        .count();

    if pending >= MAX_PENDING {
        return Err(VpError::Pipeline(format!(
            "Intent queue is full ({pending} pending). Complete or remove some before adding more."
        )));
    }

    // Check for duplicates
    let topic_lower = topic.to_lowercase();
    let duplicate = queue.iter().any(|i| {
        let is_pending = i
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("pending")
            == "pending";
        let matches_topic = i
            .get("topic")
            .and_then(|t| t.as_str())
            .map(|t| t.to_lowercase() == topic_lower)
            .unwrap_or(false);
        is_pending && matches_topic
    });

    if duplicate {
        println!("{YELLOW}~{RESET} Topic already in queue: \"{topic}\"");
        return Ok(());
    }

    // Resolve schedule time
    let scheduled_at = resolve_when(when)?;

    // Build intent entry
    let intent = serde_json::json!({
        "topic": topic,
        "priority": priority,
        "status": "pending",
        "created": state::now_iso(),
        "scheduled_at": scheduled_at,
        "source": "praxis-echo nudge",
        "prompt_template": format!(
            "Research the following topic from CURIOSITY.md: \"{topic}\". \
             Go deep — read papers, search the web, form your own position. \
             Capture findings in LEARNING.md, develop thoughts in THOUGHTS.md."
        ),
    });

    queue.push(intent);

    // Write back
    let json = serde_json::to_string_pretty(&queue)?;
    fs::write(&queue_path, format!("{json}\n"))?;

    println!("{GREEN}✓{RESET} Queued intent: \"{topic}\"");
    println!("  Priority:  {priority}");
    println!("  Scheduled: {scheduled_at}");
    println!("  Queue:     {} pending", pending + 1);

    Ok(())
}

/// Resolve relative time like "+2h", "+30m" to an ISO timestamp,
/// or pass through an ISO 8601 string as-is.
fn resolve_when(when: &str) -> VpResult<String> {
    if let Some(rest) = when.strip_prefix('+') {
        let (num_str, unit) = if let Some(n) = rest.strip_suffix('h') {
            (n, 'h')
        } else if let Some(n) = rest.strip_suffix('m') {
            (n, 'm')
        } else if let Some(n) = rest.strip_suffix('d') {
            (n, 'd')
        } else {
            return Err(VpError::Pipeline(format!(
                "Invalid relative time: \"{when}\". Use +Nh, +Nm, or +Nd."
            )));
        };

        let num: u64 = num_str
            .parse()
            .map_err(|_| VpError::Pipeline(format!("Invalid number in time: \"{when}\"")))?;

        let offset_secs = match unit {
            'h' => num * 3600,
            'm' => num * 60,
            'd' => num * 86400,
            _ => unreachable!(),
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let target = now + offset_secs;

        // Convert back to ISO using shared date algorithm
        let days = target / 86400;
        let time_secs = target % 86400;
        let hours = time_secs / 3600;
        let minutes = (time_secs % 3600) / 60;
        let seconds = time_secs % 60;

        let (year, m, d) = crate::util::days_to_date(days);

        Ok(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, m, d, hours, minutes, seconds
        ))
    } else {
        // Assume ISO 8601
        Ok(when.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_hours() {
        let result = resolve_when("+2h").unwrap();
        assert!(result.contains('T'));
        assert!(result.ends_with('Z'));
    }

    #[test]
    fn resolves_relative_minutes() {
        let result = resolve_when("+30m").unwrap();
        assert!(result.contains('T'));
    }

    #[test]
    fn passes_through_iso() {
        let result = resolve_when("2026-03-01T10:00:00Z").unwrap();
        assert_eq!(result, "2026-03-01T10:00:00Z");
    }

    #[test]
    fn rejects_invalid() {
        assert!(resolve_when("+2x").is_err());
    }
}
