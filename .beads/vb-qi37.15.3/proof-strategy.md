# Proof Strategy — vb-qi37.15.3

**Bead:** vb-qi37.15.3 — cli: Add trace command
**Phase:** State 4 (proof-planner output)
**Generated:** 2026-05-18

---

## Scope Summary

This bead adds a `trace` CLI command that performs read-only replay of Fjall-journaled `JournalEvent` records for a given `run_id`. The core proof surface is the purity and determinism of two functions in `crates/vb_cli/src/commands_journal.rs`:

- `build_trace(events: &[JournalEvent]) -> Vec<TraceEntry>`
- `trace_one(idx: usize, event: &JournalEvent) -> TraceEntry`

All journal I/O, CLI dispatch, and output formatting are excluded from formal proof (shell layer).

---

## Discovery Evidence

| Check | Result |
|---|---|
| `commands_journal.rs` exists at `crates/vb_cli/src/commands_journal.rs` | CONFIRMED |
| `#![forbid(unsafe_code)]` present in `commands_journal.rs` | CONFIRMED |
| No `unsafe`, `unwrap`, `expect`, `panic`, `todo!`, `unimplemented!` in `commands_journal.rs` | CONFIRMED |
| No `tokio`, `Mutex`, `RwLock`, `Atomic`, `spawn` in `commands_journal.rs` | CONFIRMED |
| No existing Verus annotations in `commands_journal.rs` | CONFIRMED — proof-writer must add |
| No existing Kani harnesses for trace functions | CONFIRMED — proof-writer must add |
| No existing proptest for `build_trace` | CONFIRMED — TRACE-PROP-001 must be added |
| `JournalEvent` enum is storage-validated (trusted boundary) | CONFIRMED via contract.md |
| Delivery-scope.jsonl | MISSING — scope derived from contract.md and proof-obligations.jsonl |

---

## Risk Classification

| Risk | Classification | Rationale |
|---|---|---|
| Rust-local invariant (determinism) | **Verus** | Pure function determinism; cheapest lane that formally proves same-input/same-output |
| CLI output correctness | **gauntlet-standard / moon ci** | Black-box integration tests validate format contracts |
| Error handling | **static-scan + gauntlet-standard** | `parse_run_id` clippy; error-path integration tests |
| Property-based coverage | **proptest (optional)** | Not safety-critical; low risk; discretionary |
| TLA+ temporal behavior | **waived** | Read-only journal replay; no state machine, no concurrency, no liveness beyond "events exist" |
| Concurrency / atomics | **not_applicable** | `commands_journal.rs` has no concurrency primitives |
| Unsafe / UB | **not_applicable** | Module-level `#![forbid(unsafe_code)]`; no raw pointers |
| Fuzz | **not_applicable** | `run_id` is validated by `parse_run_id` before use; journal is trusted storage |
| Miri | **not_applicable** | No unsafe code in scope |

---

## Verifier Lane Assignments

### Lane 1 — Verus (proofObligations: TRACE-VERUS-001, TRACE-VERUS-002)

**Target:** `crates/vb_cli/src/commands_journal.rs::build_trace`, `trace_one`
**Artifact:** `verification/verus/vb_cli/commands_journal.rs` (verus-lang per-crate layout)
**Command:** `cargo verus --package vb_cli` or `verus crates/vb_cli/src/commands_journal.rs`
**Expected evidence:** `verus-report.md` showing 0 errors for `build_trace` and `trace_one` spec/proof functions

Scope:
- INV-001 determinism: `spec_build_trace` models pure mapping; `proof_build_trace_deterministic` shows same input slice yields same output Vec in same order
- INV-001 completeness: every `JournalEvent` variant maps to exactly one `TraceEntry`
- Index correspondence: `entry.index == position in input slice` for all entries
- TRACE-VERUS-002 variant coverage: all 16 `JournalEvent` variants handled in `trace_one` match expression
- `impl Clone` for `TraceEntry` not required for proofs;ghost-only

Assumptions:
- `JournalEvent` variants are storage-validated (trusted boundary, not in proof scope)
- `Seq`, `Step`, and other newtype wrappers expose `.get()` to raw values (no opaque state)
- No side effects: no I/O, no Mutex, no global state

### Lane 2 — Static Scan / Clippy (proofObligation: TRACE-ERR-001)

**Target:** `crates/vb_cli/src/args.rs::parse_run_id`
**Artifact:** `crates/vb_cli/src/args.rs`
**Command:** `cargo clippy -p vb_cli -- -D warnings`
**Expected evidence:** `clippy-report.txt` with 0 warnings for `args.rs`

Scope:
- ERR-001: invalid `run_id` format returns `ParseError` (not panic, not unwrap)
- PRE-001: `parse_run_id` validates run_id format with explicit error path

### Lane 3 — gauntlet-standard / moon ci (proofObligations: TRACE-CLI-001 through 007, TRACE-ERR-002, TRACE-ERR-004)

**Target:** `crates/vb_cli/tests/cli_integration.rs` (or `crates/workspace_tests/tests/cli_integration.rs`)
**Command:** `moon ci` (gauntlet-standard)
**Expected evidence:** `moon-report.md` showing all trace integration tests pass

Each obligation maps to a specific test:
- TRACE-CLI-001: trace on known run_id outputs ordered entries
- TRACE-CLI-002: json/jsonl TraceEntry fields (index, event_type, seq, extra_json)
- TRACE-CLI-003: `--json` produces single JSON object with run_id, trace array, total
- TRACE-CLI-004: `--jsonl` produces one JSON object per entry + final total line
- TRACE-CLI-005: text format `[index] EventType step? (seq N)`
- TRACE-CLI-006: non-existent run returns empty trace / exit 0
- TRACE-CLI-007: storage error returns non-zero exit code
- TRACE-ERR-002: invalid db path returns CliExitCode::StorageError
- TRACE-ERR-004: corrupted journal read failure returns CliExitCode::StorageError

### Lane 4 — proptest (proofObligation: TRACE-PROP-001, optional/low)

**Target:** `crates/vb_cli/src/commands_journal.rs`
**Artifact:** proptest suite for `build_trace`
**Command:** `cargo test -p vb_cli -- commands_journal --proptest` (or `--test-threads=1` if sequential)
**Expected evidence:** `proptest-report.md` deterministic property test passes

Scope:
- Determinism: generate N random `JournalEvent` slices; verify `build_trace` produces identical output on repeated calls
- Completeness: verify all events produce a `TraceEntry`
- Assumptions: `JournalEvent` generation is bounded by known variant set; seq/step values in safe range

Status: `required: false` per proof-obligations.jsonl

### Waived Lanes

| Lane | Waiver Reason |
|---|---|
| TLA+ | No temporal behavior in read-only journal replay. No state machine, concurrency, retry logic, or liveness beyond "events eventually appear if they exist". Compensating evidence: INV-001 covered by Verus + proptest. Permanent waiver unless scope adds temporal behavior. |
| Kani | Bounded state model not required; Verus proves determinism over all JournalEvent variants; Kani would add CI cost without catching distinct defects |
| Flux | No refinement-type properties; determinism is naturally proven via Verus pure-function specs |
| Loom | No concurrency in `commands_journal.rs`; no threads, channels, or atomic operations |
| Miri | `#![forbid(unsafe_code)]` on the entire module; no UB surface |
| Fuzz | `run_id` is pre-validated by `parse_run_id` before journal lookup; Fjall journal is trusted storage |

---

## Corrected Artifact Paths

The proof-obligations.jsonl references `crates/velvet_ballistics/src/commands_journal.rs` which does not exist in this workspace. The correct path is `crates/vb_cli/src/commands_journal.rs`. Proof-writer must use the corrected path.

| Obligation | Corrected Artifact Path |
|---|---|
| TRACE-VERUS-001 | `crates/vb_cli/src/commands_journal.rs::build_trace` |
| TRACE-VERUS-002 | `crates/vb_cli/src/commands_journal.rs::trace_one` |
| TRACE-PROP-001 | `crates/vb_cli/src/commands_journal.rs` (proptest suite) |
| TRACE-ERR-001 | `crates/vb_cli/src/args.rs::parse_run_id` |
| TRACE-CLI-* | `crates/vb_cli/tests/cli_integration.rs` or `crates/workspace_tests/tests/` |

---

## Owner State and Rerun Guidance

| Obligation | owner_state | rerun_from | Notes |
|---|---|---|---|
| TRACE-VERUS-001 | 4 | 4 | Proof-writer creates Verus spec/proof; formal-verifier runs `cargo verus` |
| TRACE-VERUS-002 | 4 | 4 | Same artifact; same run as 001 |
| TRACE-ERR-001 | 4 | 4 | Clippy gate in moon ci; can run immediately |
| TRACE-CLI-* | 5 | 8 | Require implementation state 5+; moon ci gate at state 8 |
| TRACE-ERR-002/004 | 5 | 8 | Same as CLI; deferred to integration test phase |
| TRACE-PROP-001 | 6 | 8 | Optional; deferred to test-planning phase |

---

## Artifact Manifest (State 4 → 5 Handoff)

| Artifact | Path | Owner |
|---|---|---|
| proof-strategy.md | `.beads/vb-qi37.15.3/proof-strategy.md` | proof-planner |
| proof-plan-review-input.md | `.beads/vb-qi37.15.3/proof-plan-review-input.md` | proof-planner |
| proof-obligations.planned.jsonl | `.beads/vb-qi37.15.3/proof-obligations.planned.jsonl` | proof-planner |

All three artifacts written to `.beads/vb-qi37.15.3/`.
