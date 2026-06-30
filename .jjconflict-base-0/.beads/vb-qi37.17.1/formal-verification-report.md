# Formal Verification Report — vb-qi37.17.1

**Bead**: vb-qi37.17.1 — "cli: Add incident command"
**Date**: 2026-05-18
**Workspace**: /home/lewis/src/go-skill-vb-qi37.17.1

## Inputs
- proof-obligations.jsonl: 9 obligations (COMPILE-001, COMPILE-002, UNWRAP-001, UNWRAP-002, DEAD-001, UNIT-001, UNIT-002, INT-001, QA-001)
- delivery-scope.jsonl: 18 scope items (vb_cli files, vb_storage recovery, dead code, tests)
- baseline-report.md: 57 E0061 errors, 4 unwrap violations, no tests
- contract-verification-review.md: All 10 contract clauses (PRE-001–POST-004, INV-001–INV-006) mapped; review: PASS
- verification-layers.md: Static scan + cargo test only; formal proofs waived (pure functions, no unsafe, no concurrency, no temporal behavior)

## Tool Availability
- cargo: 1.97.0-nightly — available
- clippy (nightly): available
- moon: 2.2.4 — available
- cargo kani: not required (waived)
- verus: not required (waived)
- tlc/TLC: not required (waived)
- cargo clippy (stable): available but not needed

## Obligation Results

### COMPILE-001 (INV-005: recover_full_journal 5-arg)
- risk: medium, scope: workspace, required: true, layer: static-scan
- command: `cargo check --workspace`
- result: **PASS** — 0 E0061 errors for recover_full_journal across all crates
- evidence: `cargo check --workspace` compiles clean; 57 prior E0061 errors eliminated

### COMPILE-002 (INV-005: replay_events 3-arg)
- risk: medium, scope: workspace, required: true, layer: static-scan
- command: `cargo check --workspace`
- result: **PASS** — 0 E0061 errors for replay_events across all crates
- evidence: `cargo check --workspace` compiles clean; 10 prior E0061 errors eliminated

### UNWRAP-001 (INV-001: zero-unwrap in cmd_incident)
- risk: high, scope: touched-crate, required: true, layer: static-scan
- command: `cargo clippy --package vb_cli --lib --bins --all-features -- -D warnings`
- result: **PASS** — 0 clippy warnings, 0 unwrap_or_default violations in cmd_incident
- evidence: vb_cli clippy passes clean with -D warnings

### UNWRAP-002 (INV-001: as_str().unwrap_or on Option)
- risk: low, scope: touched-crate, required: false, layer: waiver
- command: N/A
- result: **WAIVED** — `as_str()` returns `Option<&str>`, `unwrap_or("unknown")` is zero-panic (Option, not Result)
- evidence: contract-verification-review.md approves waiver; contract.md documents as safe

### DEAD-001 (INV-006: parse_incident dead code removed)
- risk: low, scope: touched-crate, required: true, layer: static-scan
- command: `cargo check --workspace`
- result: **PASS** — 0 dead_code warnings; parse_incident function removed from source
- evidence: No dead_code warnings; function at run_db.rs:144-151 confirmed removed

### UNIT-001 (POST-001: 8 unit tests for build_incident_report)
- risk: medium, scope: touched-crate, required: true, layer: cargo test
- command: `cargo test --package vb_cli --lib commands_incident::tests`
- result: **PASS** — 8 unit tests all passing with no panics or assertion failures
- evidence: Tests cover: empty events, run failed, action completed+failed, run cancelled, multiple steps, unknown variants

### UNIT-002 (POST-002: 5 unit tests for build_repair_hints)
- risk: medium, scope: touched-crate, required: true, layer: cargo test
- command: `cargo test --package vb_cli --lib commands_incident::tests`
- result: **PASS** — 5 unit tests all passing with correct hint counts
- evidence: Tests cover: RunFailed empty/partial/full hints, RunCancelled empty/full hints, unknown code

### INT-001 (POST-003: 3 integration tests for cmd_incident)
- risk: medium, scope: touched-crate, required: true, layer: cargo test
- command: `cargo test --package vb_cli --test incident_integration`
- result: **PASS** — 3 integration tests passing; JSON output valid, no stack traces
- evidence: Tests cover: failed run JSON output, missing run error output, non-failed run exit code

### QA-001 (INV-002: no stack traces in any output path)
- risk: high, scope: touched-crate, required: true, layer: manual-qa / automated
- command: Source code inspection + test evidence
- result: **PASS** — All output paths use structured Error types, match/Result patterns, no panic paths
- evidence: cmd_incident uses `thiserror::Error` for all error paths; serde_json uses `match Result` not unwrap; no format! with Debug on JournalError in output paths

## Waivers
- UNWRAP-002: Approved by contract-verification-review.md — `as_str().unwrap_or()` on Option is zero-panic
- Formal verification (TLA+/Verus/Kani/Flux): Approved — pure functions, no unsafe, no concurrency, no temporal behavior

## Residual Risk
- **NONE** within bead scope. All 9 obligations accounted for: 8 PASS, 1 WAIVED.
- Pre-existing workspace debt in vb_runtime::primitives::collect (3 test failures) and xtask (10 clippy warnings) is unrelated to incident command and DEFERRED_GLOBAL.

## Overall Verification Status

STATUS: APPROVED

All required obligations are PASS or WAIVED. No bead-local, new-regression, or release/critical failures exist. The formal verification gauntlet is clear for vb-qi37.17.1.
