bead_id: vb-ib8i
phase: 10
updated_at: 2026-05-17T22:10:00Z
attempt: 1-of-7

Implementation summary:
- Applied rustfmt to resolve format blockers.
- Removed unused helper bindings in `vb_expr` fallback evaluator.
- Made `vb_runtime::engine::{property_tests, tests}` test-only modules.
- Removed stale unused test helpers/imports.
- Updated workspace benchmark code for current ID widths, `ShardCommand::Submit`, `RunState`, snapshot serialization surrogate, `rtrb` Producer/Consumer API, and required dev-dependencies.
- Replaced fuzz `expect` with checked handling and collapsed nested `if`.
