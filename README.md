# vigil-pulse

[![License: AGPL-3.0](https://img.shields.io/github/license/dnacenta/vigil-pulse)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)](https://rustup.rs/)

Metacognitive monitoring for AI entities.

## What is vigil-pulse?

`vigil-pulse` is a Rust library that gives AI entities the ability to watch themselves think. It monitors the quality of an entity's reflective output, enforces document pipeline flow, and tracks real-world outcomes — all to prevent an entity from drifting into mechanical, shallow, or ineffective cognition.

It's a core dependency of [pulse-null](https://github.com/dnacenta/pulse-null) but works with any system that maintains structured documents for AI reflection.

## Three Signal Categories

### Pipeline Signals

Enforces a document pipeline that moves ideas through stages of maturity:

```
Encounter → LEARNING.md → THOUGHTS.md → REFLECTIONS.md → SELF.md / PRAXIS.md
             (capture)     (incubate)    (crystallize)     (integrate)
```

Tracks document counts against configurable thresholds, detects stale items (thoughts untouched for more than 7 days, questions unresearched for more than 14), alerts when the pipeline is frozen (no movement across 3+ sessions), and auto-archives overflow when documents hit hard limits.

| Document | Soft Limit | Hard Limit |
|----------|-----------|------------|
| LEARNING.md | 5 active threads | 8 |
| THOUGHTS.md | 5 active thoughts | 10 |
| CURIOSITY.md | 3 open questions | 7 |
| REFLECTIONS.md | 15 observations | 20 |
| PRAXIS.md | 5 active policies | 10 |

### Reflection Signals

Watches the quality of reflective output through four signals:

- **Vocabulary diversity** — detects repetitive phrasing that indicates mechanical output
- **Question generation** — tracks whether the entity is still generating novel questions
- **Thought lifecycle** — measures idea turnover vs accumulation
- **Evidence grounding** — checks whether conclusions reference concrete inputs

Produces a health assessment: HEALTHY / WATCH / CONCERN / ALERT.

### Outcome Signals

Records what the entity set out to do, what it actually achieved, and what it learned from the gap. Tracks task type, domain, token usage, and tool rounds to build an operational self-model over time.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vigil-pulse = { git = "https://github.com/dnacenta/vigil-pulse", branch = "main" }
```

```rust
use vigil_pulse::{PraxisEcho, PraxisConfig, VigilEcho, CaliberEcho};

// Pipeline enforcement
let praxis = PraxisEcho::new(PraxisConfig::default());

// Reflection monitoring
let vigil = VigilEcho::from_default().unwrap();

// Outcome tracking
let caliber = CaliberEcho::new("/path/to/entity/docs".into());
```

## Module Structure

```
vigil-pulse/
├── src/
│   ├── lib.rs              # Re-exports and unified API
│   ├── pipeline/           # Document flow enforcement (was praxis-echo)
│   │   ├── archive.rs      # Document archival
│   │   ├── calibrate.rs    # Threshold calibration from outcome data
│   │   ├── checkpoint.rs   # Session checkpoints
│   │   ├── init.rs         # Initialize pipeline state
│   │   ├── nudge.rs        # Intent queue for self-initiated tasks
│   │   ├── parser.rs       # Document section parser
│   │   ├── pulse.rs        # Session-start state injection
│   │   ├── review.rs       # Session-end diff
│   │   ├── runtime.rs      # Core health calculation
│   │   ├── scan.rs         # Staleness detection
│   │   ├── state.rs        # Persistent state management
│   │   └── status.rs       # Dashboard rendering
│   ├── reflection/         # Cognitive quality monitoring (was vigil-echo)
│   │   ├── analyze.rs      # Trend analysis and alerting
│   │   ├── collect.rs      # Signal extraction from documents
│   │   ├── init.rs         # Initialize monitoring state
│   │   ├── parser.rs       # Document content parser
│   │   ├── pulse.rs        # Session-start health injection
│   │   ├── runtime.rs      # LLM output signal extraction
│   │   ├── signals.rs      # Document-based signal extraction
│   │   ├── state.rs        # Signal storage and history
│   │   ├── stats.rs        # Statistical functions
│   │   └── status.rs       # Dashboard rendering
│   └── outcomes/           # Effectiveness tracking (was caliber-echo)
│       ├── outcome.rs      # Outcome classification and inference
│       ├── runtime.rs      # Outcome recording and analysis
│       └── state.rs        # Outcome persistence
└── templates/
    ├── praxis-echo.md      # Pipeline enforcement protocol template
    └── vigil-echo.md       # Monitoring protocol template
```

## Dependencies

- [pulse-system-types](https://github.com/dnacenta/pulse-null-types) — shared monitoring traits and types

## Part of the pulse-null Ecosystem

vigil-pulse is a core dependency of [pulse-null](https://github.com/dnacenta/pulse-null), a framework for creating persistent AI entities with identity, memory, and self-monitoring.

| Crate | Role |
|-------|------|
| [pulse-null](https://github.com/dnacenta/pulse-null) | Entity framework |
| [recall-echo](https://github.com/dnacenta/recall-echo) | Memory system |
| **vigil-pulse** | Metacognitive monitoring |
| [chat-echo](https://github.com/dnacenta/chat-echo) | Chat interface |
| [pulse-system-types](https://github.com/dnacenta/pulse-null-types) | Shared contracts |

## License

[AGPL-3.0](LICENSE)
