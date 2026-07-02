# Hazard Analysis — vb-0x1cb

- bead_id: vb-0x1cb
- phase: 3 (contract)
- attempt: 1-of-1
- captured_at: 2026-07-01T15:55:00Z
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- controller: femdation
- scope_kind: hazard_analysis
- lane_profile: rust_local_concurrency_empty
- status: contract drafted

This document enumerates the hazards — temporal, state, refinement, observability, performance, and release — that the dual-failure contract introduces or engages. Each hazard is paired with a guard, a witness, and an indication of which downstream lane owns the verification.

## 1. Temporal hazards

### H-TEMPORAL-1 — Primary-after-secondary ordering

| Field | Value |
|-------|-------|
| Description | If the rolled-back `run_state_insert` appears to succeed (returns `Ok(_)`) but the primary journal event has not yet been durably accepted, an operator may mistake a recovered rollback for a successful close. |
| Guard | The contract requires `append_journal_event` to return `Err(primary)` BEFORE `observe_run_state_rollback` is invoked. Ordering is enforced by the control flow at `transitions.rs:94-101` (the `if let Err(error)` branch). |
| Witness | `proptest finish_run_returns_primary_when_journal_rejects` (proof seed S2) — must observe `Err(primary)` regardless of the rollback outcome. |
| Lane | `proptest` (default Rust profile). |

### H-TEMPORAL-2 — `Arc` clone cost in tight loops

| Field | Value |
|-------|-------|
| Description | `Arc::new(secondary)` followed by `Arc::clone(&secondary)` for the trace event payload creates two heap allocations per dual-failure event. The trace ring is bounded but the failure-mode throughput is bounded by slot-exhaustion rate. |
| Guard | `Arc::clone` is `O(1)`; not in any hot path (this code is OFF the happy path). |
| Witness | `kani: size_of::<TraceEvent::RunRollbackFailed>() <= 256` (proof seed S3) — Flux refinement that the per-event memory cost is bounded. |
| Lane | `kani`, `flux-rs`. |

## 2. Rust-core invariant hazards

### H-INV-1 — Lost primary error

| Field | Value |
|-------|-------|
| Description | The most direct hazard: a buggy implementation could swap the primary error for the secondary one (`Err(secondary)`). This violates invariant I1. |
| Guard | `Result::Err(p)` is set by `return Err(error)` BEFORE the rollback branch exits; `observe_run_state_rollback` returns a non-Result value; the caller (`finish_run`, `fail_run_state`) decides what to `return`. The pattern is exhaustive: only `Err(primary)` is returned to callers. |
| Witness | `cargo test -p vb_runtime` tests `chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append` and `chunk_008.rs::fail_run_state_rollback_surfaces_primary_storage_journal_append` (mirrored from `LegacyStepFailsJournal` pattern). |
| Lane | `proptest` (default Rust profile) + behavior tests under `cargo test`. |

### H-INV-2 — Lost secondary error (the bead's core hazard)

| Field | Value |
|-------|-------|
| Description | A buggy implementation could (a) keep the `let _ = ...` shape, (b) `match { Ok(_) | Err(_) => {} }`, (c) `drop(secondary)` after binding. Any of these would still pass the source-gate silent drop the secondary. |
| Guard | (a) replace `let _ = ...` with `match self.observe_run_state_rollback(...)`; (b) `match { Ok(_) | Err(_) => {} }` does not appear in the new code; (c) the helper is `#[must_use]` so dropping the `ObservedRollbackOutcome` is its own source-lint violation. The `let _ = ...` is removed entirely. |
| Witness | `scripts/check-ignored-fallible-results.sh` exits 0 for `transitions.rs` without an allow row. The proptest `finish_run_emits_run_rollback_failed_when_both_journal_and_slot_fail` proves the dual-failure event was pushed. |
| Lane | Source-gate (`verify-standard`) + `proptest`. |

### H-INV-3 — Rollback slot exhaustion masking the primary error

| Field | Value |
|-------|-------|
| Description | If `run_state_insert` returns `Err(secondary)`, the contract records the secondary via `trace_ring.push` but the function STILL returns `Err(primary)`. A naive bug might `return Err(secondary)` to short-circuit. |
| Guard | The contract enforces this by the explicit return statement at `transitions.rs:101` (post-repair): `return Err(error);` is OUTSIDE the rollback helper and cannot be reordered. |
| Witness | `Verus: spec finish_run_returns_primary_storage_journal_append_regardless_of_rollback_outcome` (proof seed S5). |
| Lane | `verus` (default Rust profile). |

### H-INV-4 — Terminal fence divergence

| Field | Value |
|-------|-------|
| Description | The pre-repair code had `let _ = self.run_state_insert(run, state);` followed by `return Err(error)`. If the rollback failed, the run's terminal state was lost from `runtime_states` — a divergent state. This mirrors the `LegacyStepFailsJournal` invariant at `chunk_004.rs:240-319`. |
| Guard | The new contract binds `Err(secondary)` into `trace_ring.push(TraceEvent::RunRollbackFailed { run, site, primary, secondary })`. Operators observing `RunRollbackFailed` can rebuild the run from the journal. |
| Witness | `proptest finish_run_rollback_preserves_runtime_states_when_recovered` and `…_diverges_when_dual_failed` (mirrored from `chunk_004.rs`). |
| Lane | `proptest`. |

## 3. Bounded-state hazards

### H-STATE-1 — Allow-file ledger accumulation

| Field | Value |
|-------|-------|
| Description | If the `DISCARD-006` allow row at `scripts/ignored-fallible-results.allow:4` is left in place, the gate is happy but the permit shadows future regressions. The original bead note labels the `follow_up=vb-ttki3` field incorrect (per `codebase-map.md` §2). |
| Guard | The contract requires the row to be REMOVED in this bead, NOT replaced. The `follow_up` field, if reintroduced, must point at the new repair follow-up bead (`vb-0x1cb` itself closes this; no follow-up). |
| Witness | `diff scripts/ignored-fallible-results.allow` shows 0 substantive rows (allowing 3 header comment lines). |
| Lane | Source-gate postcondition: `bash scripts/check-ignored-fallible-results.sh` exits 0. |

### H-STATE-2 — `ObservedRollbackOutcome::DualFailed` observability drift

| Field | Value |
|-------|-------|
| Description | If `trace_ring` is saturated, the `RunRollbackFailed` event may be silently dropped. The contract does not change `TraceRing::push` semantics, so a saturated ring falls back to its pre-existing behavior. |
| Guard | The new contract pins `Arc::clone` for `primary` and `secondary` so the dual-failure-event memory is bounded. TraceRing saturation is a separate (pre-existing) concern. |
| Witness | Not in scope for this bead; the test planner may add a saturation test later but it is independent of this fix. |
| Lane | Out of scope. |

## 4. Refinement hazards

### H-REFINE-1 — `RuntimeError` enum coverage

| Field | Value |
|-------|-------|
| Description | The repair does NOT add a new variant but DOES touch `match` arms at `transitions.rs:100` and `:202`. The `RuntimeError` enum is `#[non_exhaustive]` (`error/mod.rs:6`); the repair must be exhaustive. |
| Guard | The new code matches on `ObservedRollbackOutcome::RollbackRecovered | DualFailed { .. }`. The helper is `#[must_use]` to prevent silent drop. |
| Witness | `cargo check -p vb_runtime` exits 0; `cargo clippy ... -W clippy::let_underscore_must_use` exits 0. |
| Lane | Source-lint + `verus` refinement on `observe_run_state_rollback`. |

### H-REFINE-2 — `trace_ring.push` event-size boundedness

| Field | Value |
|-------|-------|
| Description | A faulty implementation could put a large `String` reason into the trace event, blowing the bounded ring budget. |
| Guard | The contract pins payload as `Arc<RuntimeError>` (pointer-sized) + `RunId` (8 bytes) + `RollbackSite` (1 byte). No `String`. |
| Witness | `flux-rs: size_of_run_rollback_failed_event <= 256_bytes` (proof seed S4). |
| Lane | `flux-rs`. |

## 5. Concurrency hazards

The bead claims **concurrency-empty**. Per `codebase-map.md` §Hazard Tags and §Required Verifier Modes:

### H-CONC-1 — Single-shard sequential rollback

| Field | Value |
|-------|-------|
| Description | Because `Shard::finish_run` and `Shard::fail_run_state` are called from a single-threaded dispatcher (`Shard::tick`), no inter-leaving hazard exists. The audit hygiene is the existing `Arc<SharedRuntimeJournal>` that allows the journal pointer to be shared across threads, but the rollback invariant is local to one shard invocation. |
| Guard | The contract does NOT introduce new parallelism or atomics. The `Arc<RuntimeError>` is the only concurrency primitive. |
| Witness | Out of scope for `loom` (not in lane profile). |
| Lane | None — concurrency-empty profile. |

## 6. Unsafe / provenance hazards

### H-UNSAFE-1 — Forbidden `unsafe`

| Field | Value |
|-------|-------|
| Description | The repair uses only safe Rust (`#![forbid(unsafe_code)]` at `transitions.rs:1`). No `unsafe` blocks; no `transmute`; no raw pointer arithmetic. |
| Guard | AGENTS.md forbids `unsafe` overall. |
| Witness | `cargo clippy` lint: `clippy::undocumented_unsafe_blocks` exits 0. |
| Lane | Source-lint. |

## 7. Hostile input hazards

### H-HOSTILE-1 — Parser / codec attack surface

| Field | Value |
|-------|-------|
| Description | This bead introduces no parsers and no codecs. `RollbackSite` is constructed from a discriminant, not parsed from external input. `Arc<RuntimeError>` carries only pre-validated types. |
| Guard | No `FromStr` / `From<&[u8]>` impls introduced. |
| Witness | n/a — out of scope; `cargo-fuzz` is not in this lane profile. |
| Lane | None. |

## 8. Performance / release hazards

### H-PERF-1 — Added allocation per failed call

| Field | Value |
|----------- |-------|
| Description | A dual-failure event adds `Arc::new(secondary)` + `Arc::clone(&secondary)` per occurrence. Rollback failures should be extremely rare (slot exhaustion under journal rejection); the cost is dwarfed by the journal write cost itself. |
| Guard | The cost is not on the happy path; it is OFF the steady state. |
| Witness | Optional: `bench fail_run_state_dual_failed_overhead` against `cargo bench` baseline — out of scope for this bead. |
| Lane | Performance is NOT in this lane profile. |

### H-REL-1 — Public API addition

| Field | Value |
|-------|-------|
| Description | The repair adds `RollbackSite` (new enum) and `TraceEvent::RunRollbackFailed` (new enum variant). Both are `#[non_exhaustive]` so downstream matches are NOT exhaustive on `TraceEvent`. |
| Guard | Existing `#[non_exhaustive]` attributes + `_ =>` wildcards in matches elsewhere are preserved by the new code (the helper internally matches with `match self.run_state_insert(...) { Ok(_) => ..., Err(secondary) => ... }` — exhaustive). |
| Witness | `cargo check -p vb_runtime` exits 0; `cargo doc -p vb_runtime --no-deps` exits 0. |
| Lane | Source-lint + cargo check. |

## 9. Diagnostic-code drift

### H-DIAG-1 — Loss of `STORAGE_JOURNAL_APPEND_FAILED_CODE` mapping

| Field | Value |
|-------|-------|
| Description | The repair does NOT change `RuntimeError::diagnostic_code()`. The match arm at `diagnostics.rs:57` for `StorageJournalAppend { .. } → STORAGE_JOURNAL_APPEND_FAILED_CODE` is preserved. A naive bug could move the `Err(primary)` to a different variant and break the diagnostic surface. |
| Guard | The repair's `return Err(error)` is the same expression as before — `error` is the `Err(_)` value from `append_journal_event`, already typed as `RuntimeError::StorageJournalAppend { .. }`. |
| Witness | `cargo test -p vb_runtime -- error::tests_diagnostics` passes; `runtime_error_diagnostic_code_catalog` passes. |
| Lane | Source-lint + `cargo test`. |

## 10. Customer-facing behavior hazards

### H-CUST-1 — User-visible error change

| Field | Value |
|-------|-------|
| Description | The contract returns `Err(primary)` for the same set of failure modes as before. The secondary error becomes visible via TraceEvent observability, which is consumed by diagnostic dashboards but not by direct user-facing payloads. No customer API change. |
| Guard | Caller-visible `Result::Err(_)` is unchanged. Trace events are extension-only. |
| Witness | `cargo test` for `RuntimeError` variants unchanged. |
| Lane | Source-lint + behavior tests. |

## 11. Hazard summary table

| Hazard | Severity | Lane | Status |
|--------|----------|------|--------|
| H-INV-1 Lost primary error | blocker | proptest + verus | guard in place |
| H-INV-2 Lost secondary error (bead core) | blocker | source-gate + proptest | guarded by removal of `let _ = ...` |
| H-INV-3 Rollback masking | blocker | verus | guarded by `return Err(error)` after the helper |
| H-INV-4 Terminal fence divergence | high | proptest (mirror `LegacyStepFailsJournal`) | guarded by `RunRollbackFailed` event |
| H-TEMPORAL-1 Primary-after-secondary | high | proptest | guarded by control flow |
| H-TEMPORAL-2 Arc clone cost | low | flux-rs / kani | bounded under `#![forbid(unsafe_code)]` |
| H-STATE-1 Allow-file accumulation | high | source-gate | row removed |
| H-STATE-2 Ring saturation | low | out of scope | inherited |
| H-REFINE-1 Exhaustive match | medium | source-lint + verus | pin via `#[must_use]` |
| H-REFINE-2 Trace event size | medium | flux-rs | bounded by `Arc` |
| H-CONC-1 Single-shard rollback | low | n/a | concurrency-empty |
| H-UNSAFE-1 Forbidden unsafe | low | source-lint | forbid already on file |
| H-HOSTILE-1 Parser attack | n/a | none | no parser surface |
| H-PERF-1 Dual-failure alloc | low | none | off the hot path |
| H-REL-1 Public API add | medium | cargo check + clippy | `#[non_exhaustive]` preserved |
| H-DIAG-1 Diagnostic-code drift | blocker | cargo test | unchanged |
| H-CUST-1 User-visible change | low | cargo test | unchanged for caller |

## 12. Cross-references

- `domain-model.md` §Invariants, §Forbidden.
- `type-contracts.md` §2.2 (helper signature), §2.3 (derive).
- `workflow-model.md` §1, §2 (state transitions).
- `error-taxonomy.md` §2, §3.
- `boundary-map.md` §2.1–2.4.
- `contract.md` clauses C-1 through C-7.
- `proof-seeds.jsonl` rows S1–S7.
