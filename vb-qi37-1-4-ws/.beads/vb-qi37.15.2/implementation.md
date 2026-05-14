bead_id: vb-qi37.15.2
bead_title: cli: Add submit command and job ledger
phase: State 6
updated_at: 2026-05-11T00:00:00Z

# Implementation Summary

Holzman references read: `/home/lewis/.agents/skills/holzman-rust/SKILL.md`, `references/nasa-jpl-standards.md`, `references/latency-throughput-playbook.md`, `references/runtime-performance-architecture.md`, `references/zero-cost-abstractions.md`, `references/simd-patterns.md`, `references/mechanical-empathy-toolchain.md`.

Changes:
- Added black-box submit tests for JSON identifiers, missing input diagnostics, unknown durability, and later inspection.
- Repaired submit durable event recording by appending `vb_storage::JournalEvent::RunAccepted` through the already-open journal instead of reopening the same Fjall DB through `runtime_journal_for_mode` while locked.

Evidence:
- Red: `cli_submit_json_returns_structured_identifiers` failed before implementation with `FjallError: Locked`.
- Green: `rtk cargo test -p velvet_ballastics --test cli_integration cli_submit_json_returns_structured_identifiers` -> 1 passed.
