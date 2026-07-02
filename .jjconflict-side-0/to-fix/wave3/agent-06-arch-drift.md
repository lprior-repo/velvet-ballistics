# Wave 3 — Architectural-Drift Review (agent-06, chunk 06)

Scope: storage/recovery/codec/digest bugs. Read-only. No beads. No source modifications.
Working directory: `/home/lewis/src/velvet-ballistics` (verified `git rev-parse --show-toplevel`).

Bug chunk: `/tmp/wave3-chunk-06.txt` — 10 IDs: `vb-5fxrk, vb-6nwuq, vb-76rmw, vb-7gm7c, vb-7kucc, vb-7m2pd, vb-7ol6y, vb-7q6c9, vb-7qn3n, vb-83aqs`.

## Thresholds

- file-len flag: `> 300` lines
- fn logical lines flag: `> 25`
- DDD cohesion: fix must live inside `vb_storage` bounded context

## Per-bug table

| bug-id | pri | source-fix | test | fix-file | fix-fn-lines | file-len | drift? | targeted-cmd | result | verdict | evidence |
|-------|-----|------------|------|----------|--------------|----------|--------|--------------|--------|---------|----------|
| vb-5fxrk | P3 | `lock_admission` uses `PoisonError::into_inner()` in `vb_runtime/src/shard/types.rs:383,487` + `vb_runtime/src/action_queue.rs:133,167,177` | RA-014 regression tests (`introspection_poison_regression_tests` mod) | `crates/vb_runtime/src/shard/types.rs` (1991 lines) | registry methods ~18 each | 1991 | y | `cargo test -p vb_storage --lib lock_admission` | 0 / 1270 (filter) — out-of-crate | UNKNOWN (n/a for vb_storage; dupe of vb-zfyh5) | bead closed as dupe; fix in vb_runtime not vb_storage — outside wave3 bounded context |
| vb-6nwuq | P1 | `record_action_*` methods consolidated in `crates/vb_storage/src/recovery/replay/summary.rs:597-689`; kani-attribute bug fixed in `crates/vb_core/src/kani_workflow_arbitrary.rs:667`; module re-export bug in `crates/vb_storage/src/journal/append/mod.rs:38-43` | `recover_runtime_frame_seed_from_events_*` suite (6 tests) | `crates/vb_storage/src/recovery/replay/summary.rs` | 7-15 per method | 999 | y | `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` | 6 passed | PATCHED | bead closed as dupe of vb-y3az6; all six seed tests pass |
| vb-76rmw | P4 | `workflow_digest_bytes_equal` replaced by `expected == found` in `crates/vb_storage/src/recovery/recover.rs:21-51` (`check_workflow_source_digest`, `check_compiled_ir_digest`) | `verify_digests_returns_ok_when_all_match`, `verify_digests_returns_mismatch_when_ir_differs` | `crates/vb_storage/src/recovery/recover.rs` | 19 + 10 | 240 | n | `cargo test -p vb_storage --lib verify_digests` | 2 passed | PATCHED | SJ-004 simplification landed in commit `53614b915`; digest.rs collapsed; tests pass |
| vb-7gm7c | P2 | `derive_lifecycle_state_from_events` at `crates/vb_storage/src/journal/incident.rs:146-171` — explicit arms for 17 variants but `_ => LifecycleState::Active` wildcard retained at line 168 | none directly (function has no unit test in incident.rs tests mod) | `crates/vb_storage/src/journal/incident.rs` | 26 | 412 | y | `cargo test -p vb_storage --lib derive_lifecycle_state` | 0 filtered (no test) | PARTIAL | wildcard `_ => Active` catch-all still present at line 168 with `#[allow(unreachable_patterns)]`; close reason claims "removed wildcard arm" but code still has it; cannot verify original SJ-005 finding (bug-hunt-2026-06-21 dir absent) |
| vb-7kucc | P4 | `idempotency_evidence_from_contracts` + assignment parity in `crates/vb_storage/src/admission.rs:334-466` (Relaxed arm at 342-343 mirrors Journaled/Strict arm at 381-382) | `sa013_relaxed_carries_idempotency_evidence_from_contracts`, `sa013_relaxed_and_journaled_idempotency_evidence_parity` | `crates/vb_storage/src/admission.rs` | 16 | 540 | y | `cargo test -p vb_storage --lib sa013` | 2 passed | PATCHED | SA-013 ownership parity enforced; both Relaxed and checked arms receive identical `idempotency_evidence.keyed`/`.attested` fields |
| vb-7m2pd | P1 | kani attribute removed at `crates/vb_core/src/kani_workflow_arbitrary.rs:667`; module re-export fixed at `crates/vb_storage/src/journal/append/mod.rs:38-43` | full vb_storage lib suite (1270 tests) | `crates/vb_storage/src/recovery/replay/summary.rs` (downstream of compile fix) | n/a (compile fix) | 999 | y | `cargo test -p vb_storage --lib` | 1270 passed | PATCHED | root cause was 2-line kani attribute + module re-export fix; all fuzz-smoke lanes green; frame_seed split was downstream symptom |
| vb-7ol6y | P0 | hydration fail-closed: corrupt taint metadata + legacy collect_frame_extra + silent erasure fixed in `crates/vb_storage/src/recovery/hydrate_support.rs:17-57` + `crates/vb_storage/src/recovery/types.rs:90` (`SlotTaintReadObservation`, `SlotTaintResolution`, `resolve_slot_taint_read`) + `crates/vb_storage/src/recovery/hydrate.rs:181-260` (hydrate_run_frame validation chain) | 3 targeted tests: `hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata`, `hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar`, `apply_tail_events_fails_closed_when_taint_read_fails` (+ 34 hydrate_run_frame tests) | `crates/vb_storage/src/recovery/hydrate_support.rs` + `hydrate.rs` + `types.rs` | 9 + 11 + ≤20 | 484 + 536 + 606 | y | `cargo test -p vb_storage --lib hydrate_run_frame` | 37 passed (incl. all 3 fail-closed tests) | PATCHED | all three P0 fail-closed bugs fixed; `read_taint(*slot).unwrap_or(Taint::Clean)` erased, now uses `SlotTaintResolution::FailClosed` |
| vb-7q6c9 | P3 | `trim_events_for_run` at `crates/vb_storage/src/trimming/logic.rs:58-116` — keys < 17 bytes return `Err(TrimError::IncompleteTrim)` (line 75-77) | `trim_events_for_run_fails_closed_on_malformed_event_key` | `crates/vb_storage/src/trimming/logic.rs` | 45 | 383 | y | `cargo test -p vb_storage --lib trim_events_for_run` | 1 passed | PATCHED | SC-006 fail-closed path active; SC-008 heap-alloc fix at line 94 also applied |
| vb-7qn3n | P2 | `recover_snapshot_plus_tail` at `crates/vb_storage/src/recovery/replay/core.rs:239-260` — enforces `event.seq() > snapshot_seq` (line 247) returning `RecoveryError::ReplayDivergence`; `validate_tail_seq_after_snapshot` in `hydrate.rs:140-152` | `hydrate_run_frame_rejects_tail_event_before_snapshot_seq`, `hydrate_run_frame_rejects_tail_event_at_same_seq_as_snapshot`, `hydrate_run_frame_rejects_tail_event_seq_less_than_snapshot` | `crates/vb_storage/src/recovery/replay/core.rs` + `hydrate.rs` | 20 + 11 | 290 + 536 | y | `cargo test -p vb_storage --lib recover_snapshot_plus_tail` (0 filtered, covered by hydrate_run_frame suite) | 37 hydrate tests pass | PATCHED | SR-006 cross-snapshot contiguity enforced at both `recover_snapshot_plus_tail` (line 247) and `validate_tail_seq_after_snapshot` (hydrate.rs:144) |
| vb-83aqs | P2 | **NONE** — `JournalWriteBatch::commit` at `crates/vb_storage/src/batch.rs:324-330` still returns `Ok(())` for an aborted batch | `e2e_aborted_batch_commit_succeeds_with_no_persist` (line 1841) asserts the buggy behavior | `crates/vb_storage/src/batch.rs` | 7 | 2005 | y | `cargo test -p vb_storage --lib aborted` | 1 passed (validates buggy behavior) | **NOT-PATCHED** | bead claims `JournalError::BatchAborted` was added — it does NOT exist in source; commit() at line 325-327 literally `return Ok(())` when `self.aborted == true`; the test at line 1860 explicitly `expect("aborted batch commit must succeed")` — bug still live; close reason is misleading |

## Summary

- **bugs-checked**: 10 / 10
- **pass (PATCHED)**: 6 (vb-6nwuq, vb-76rmw, vb-7kucc, vb-7m2pd, vb-7ol6y, vb-7q6c9, vb-7qn3n) — actually 7 if you count vb-5fxrk as out-of-scope PASS
- **partial**: 1 (vb-7gm7c)
- **unknown / out-of-scope**: 1 (vb-5fxrk — dupe pointing to vb_runtime fix)
- **NOT-PATCHED**: 1 (vb-83aqs — and by extension vb-2eprq — SA-002 bug is live in `batch.rs:324-330`; the close reason is incorrect)

## Drift-introduced cases

Files exceeding the 300-line Scott-Wlaschin ceiling among the fix landing zone:

| file | lines | over by |
|------|------:|--------:|
| `crates/vb_storage/src/batch.rs` | 2005 | +1705 |
| `crates/vb_storage/src/recovery/replay/summary.rs` | 999 | +699 |
| `crates/vb_storage/src/recovery/types.rs` | 606 | +306 |
| `crates/vb_storage/src/recovery/hydrate.rs` | 536 | +236 |
| `crates/vb_storage/src/admission.rs` | 540 | +240 |
| `crates/vb_storage/src/recovery/hydrate_support.rs` | 484 | +184 |
| `crates/vb_storage/src/journal/incident.rs` | 412 | +112 |
| `crates/vb_storage/src/trimming/logic.rs` | 383 | +83 |
| `crates/vb_runtime/src/shard/types.rs` | 1991 | +1691 |

`summary.rs` holds the consolidated `FrameSeedAccumulator` (997+ lines for one struct) — borderline god-file. `batch.rs` is the most flagrant drift case (2005 lines, contains its own `mod tests` and SA-002 bug site).

Function-level drift:

| function | file | logical lines | over by |
|----------|------|--------------:|--------:|
| `trim_events_for_run` | `trimming/logic.rs:58` | 45 | +20 |
| `derive_lifecycle_state_from_events` | `journal/incident.rs:146` | 26 | +1 |

DDD cohesion: all 7 in-crate fixes stay inside the `vb_storage` bounded context. The FrameSeedAccumulator consolidation in `summary.rs` crosses the `recovery::replay::summary` submodule but does not leak into other bounded contexts.

## Top NOT-PATCHED with one-line reason

1. **vb-83aqs (and vb-2eprq)** — SA-002 is live: `batch.rs:325-327` returns `Ok(())` for aborted batches, the test at `batch.rs:1860` enshrines the bug as expected behavior, and `JournalError::BatchAborted` does not exist anywhere in the source tree.
2. **vb-7gm7c** — SJ-005 fix is partial: `derive_lifecycle_state_from_events` still terminates with `_ => LifecycleState::Active` wildcard at `journal/incident.rs:168` (mitigated only by `#[allow(unreachable_patterns)]`).

## Evidence trail

- `bd show <id>` for all 10 IDs (single batched bash call).
- `wc -l` per fix file (single batched bash call).
- Logical-line counts via `awk … | grep -v` excluding blank lines and doc comments.
- `cargo test -p vb_storage --lib <test>` for every targeted regression test (10 invocations).
- `cargo test -p vb_storage --lib --no-fail-fast` (full suite): **1270 passed; 0 failed**.
- Bug-hunt source paths (`bug-hunt-2026-06-21/findings/...`) and several intermediate refactor files (`recovery/digest.rs`, `journal/incident/lifecycle.rs`, `journal/incident/`, `recovery/event_replay/`, `recovery/replay/summary/slots/`, `recovery/replay/summary/frame_seed*`) are absent from the working tree; verdict for `vb-7gm7c` is constrained by that loss.

## File path written

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-06-arch-drift.md`
