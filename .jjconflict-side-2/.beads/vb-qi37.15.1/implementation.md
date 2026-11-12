bead_id: vb-qi37.15.1
bead_title: cli: Add simulate command
phase: State 6
updated_at: 2026-05-11T00:00:00Z

# Implementation Summary

Holzman references read: `/home/lewis/.agents/skills/holzman-rust/SKILL.md`, `references/nasa-jpl-standards.md`, `references/latency-throughput-playbook.md`, `references/runtime-performance-architecture.md`, `references/zero-cost-abstractions.md`, `references/simd-patterns.md`, `references/mechanical-empathy-toolchain.md`.

Changes:
- Added black-box simulate tests for text, JSON, invalid workflow, and no-DB side effects.
- Added `schema_version` and `kind` fields to `simulate --json`/`--jsonl` output.

Evidence:
- Initial text/json simulate test passed against existing implementation; schema envelope coverage was then added to lock the current structured-output contract.
- Green: `rtk cargo test -p velvet_ballistics --test cli_integration cli_simulate_json_emits_deterministic_trace` -> 1 passed.
- Red phase caveat: no separately captured failing run exists for the schema assertion after it was added; downstream review must treat this as a TDD process gap, not a product-behavior failure.
