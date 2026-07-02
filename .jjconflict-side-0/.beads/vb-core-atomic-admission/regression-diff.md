# Regression Diff

STATUS: REJECTED

bead_id: vb-core-atomic-admission
state: 11
workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission`
attempt: state11-formal-exec-retry
updated_at: 2026-05-16T19:30:00Z

## Baseline Classification

Baseline report captured clean jj status with no bead-local work started.
`moon ci`, Miri, cargo-mutants, and cargo semver-checks had no pre-existing evidence in this isolated workspace.
The `vb_ipc` socket path issue and `source-length` jj/git issue are pre-existing global tooling constraints.

## Current State 11 Failures

### FAIL_LOCAL: MIRI-CODEC-009
**Root cause**: Compile error in `crates/vb_storage/src/codec_miri_tests.rs:315`. `JournalEvent::RunCancelled` initializer missing fields `attempt` and `reason`.
**Scope**: touched vb_storage crate.
**Regression classification**: Local. The fixture predates the current JournalEvent enum shape. Not an environment failure; not unrelated global debt.
**Fix required**: Add missing fields to `RunCancelled` initializer in `codec_miri_tests.rs`.

### FAIL_LOCAL: MUT-ERR-010
**Root cause**: Baseline cargo test fails. 9 vb_storage tests assert `gate_count == 2` while State 10 implementation returns `gate_count == 15`. Same root cause as STATIC-SCAN-011.
**Scope**: touched vb_storage crate.
**Regression classification**: Local. The test assertions predate the State 10 15-gate implementation.
**Fix required**: Align vb_storage admission test assertions with 15-gate implementation.

### FAIL_LOCAL: STATIC-SCAN-011
**Root cause**: moon ci fails with three sub-failures:
1. `lint-src`: 21 clippy errors in `fuzz/src/lib.rs` (unwrap_used, let_underscore_must_use, as_conversions, arithmetic_side_effects, len_zero).
2. `source-length`: `fatal: not a git repository` (jj workspace tooling constraint — pre-existing unrelated DEFERRED_GLOBAL).
3. `test`: 5 vb_ipc socket failures (pre-existing unrelated DEFERRED_GLOBAL) + 9 vb_storage gate_count assertions (15 vs 2 — local).
**Scope**: touched vb_storage + fuzz test crate.
**Regression classification**: Local (gate_count test mismatch and fuzz clippy); pre-existing unrelated (jj workspace and vb_ipc socket).
**Fix required**: Repair vb_storage gate_count test assertions; repair fuzz/lib.rs clippy violations; source-length and vb_ipc are pre-existing.

### FAIL_LOCAL: INTEG-FAIL-012
**Root cause**: Same as STATIC-SCAN-011. moon ci nonzero; vb_storage gate_count assertions block full integration suite.
**Scope**: touched vb_storage.
**Regression classification**: Local.

### FAIL_LOCAL: API-COMPAT-013
**Root cause**: `cargo semver-checks --workspace` exit 101; cannot retrieve index for unpublished `vb_codegen` crate.
**Scope**: changed-api (vb_storage/vb_runtime public admission APIs).
**Regression classification**: Local tooling-suitability. The exact command cannot produce API evidence for this unpublished workspace. Not an API breakage; tool limitation.
**Fix required**: Replace with approved baseline-aware semver command or approve waiver for unpublished workspace.

### FAIL_LOCAL: ERR-INVALID-015 through ERR-INDEX-022
**Root cause**: Same as STATIC-SCAN-011. moon ci nonzero blocks all moon-backed error-scenario obligations.
**Scope**: touched vb_storage.
**Regression classification**: Local. All share the same vb_storage gate_count test assertion blocker.

## Diff from Prior State 11 Runs

This is the second State 11 formal execution retry. Compared to prior attempt:
- **Same**: TLA+, Verus, Kani waiver, fuzz waiver, performance waiver all PASS/WAIVED.
- **Same**: Miri still fails with same `codec_miri_tests.rs:315` compile error.
- **Same**: cargo-mutants still fails because baseline test fails with same gate_count assertion.
- **Same**: moon ci still fails with same 3 sub-failures (lint-src, source-length, test).
- **Same**: API-COMPAT-013 still fails with same vb_codegen registry issue.
- **New finding**: None. All failures are identical to prior State 11 attempt.

## Pre-existing Unrelated Global Debt (DEFERRED_GLOBAL Candidates)

These failures are unrelated to the strict accepted-run admission bead scope:
1. `source-length`: jj workspace is not a git repository. Tooling assumption not specific to this bead.
2. `vb_ipc` socket tests (5 failures): `path must be shorter than SUN_LEN`. Pre-existing IPC test issue unrelated to admission/storage.

Per formal-verifier rule `scope_before_status`: these are pre-existing unrelated workspace debt. They are recorded as DEFERRED_GLOBAL follow-up for the global workspace, not as blockers for this bead's advancement.

## Classification Summary

| Obligation | Classification | Blocker |
|-----------|----------------|---------|
| TLA-ATOM-001 | PASS | — |
| VERUS-PRE-001 through VERUS-ERR-006 | PASS | — |
| KANI-PROP-007 | WAIVED | — |
| FUZZ-ART-008 | WAIVED | — |
| MIRI-CODEC-009 | FAIL_LOCAL | Repair `codec_miri_tests.rs` fixture |
| MUT-ERR-010 | FAIL_LOCAL | Align vb_storage gate_count test assertions |
| STATIC-SCAN-011 | FAIL_LOCAL | Align gate_count + fix fuzz clippy |
| INTEG-FAIL-012 | FAIL_LOCAL | Same as STATIC-SCAN-011 |
| API-COMPAT-013 | FAIL_LOCAL | Approve semver replacement/waiver |
| PERF-NONGOAL-014 | WAIVED | — |
| ERR-INVALID-015 through ERR-INDEX-022 | FAIL_LOCAL | Same as STATIC-SCAN-011 |

No FAIL_REGRESSION classified this run. All failures are local to touched crates or tooling-suitability issues.
