bead_id: vb-ogwh
phase: 13
updated_at: 2026-05-17T22:27:00Z

# Assurance Bundle

Requirement POST-001 maps to:
- Code: `crates/vb_runtime/src/runtime.rs`.
- Tests: `tick_shard_shutdown_drains_and_reports_dead` plus companion directive tests.
- Scoped command: `rtk cargo test -p vb_runtime tick_shard_` -> 4 passed.
- Canonical command: `moon ci --force --summary normal` -> 23 actions completed, all pass after rebase.
