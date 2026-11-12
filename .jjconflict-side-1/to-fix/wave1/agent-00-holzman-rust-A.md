# Wave 1 Agent 00 — Holzman-Rust (NASA/JPL Power-of-Ten) Review

Reviewer: holzman-rust agent (read-only validation, no source mods, no beads).
Scope: 10 bug IDs in `/tmp/wave1-chunk-00.txt`.

## Holzman / NASA-JPL rules applied

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`.
- No unchecked indexing, slicing, casts, or arithmetic in production.
- No YAML, JSON, or HTTP in runtime core.
- Generated Rust mode for maxperf paths (N/A — no perf hot path changes in this chunk).
- `forbid(unsafe_code)` present at file head of every touched module.
- Workspace lint: `clippy::as_conversions = deny`, `clippy::arithmetic_side_effects = deny`,
  `clippy::panic = forbid`, `clippy::unwrap_used = forbid`, `clippy::expect_used = forbid`.

## Result table

| bug-id | pri | source-fix | test | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|
| vb-06t25 | P2 | UNKNOWN — `crates/vb_storage/src/codec_miri_tests_compile_check.rs` and `scripts/build-check-codec-miri-features.sh` do not exist; current `codec_miri_tests.rs` has no `MOK` const or `include_str!` and is `#[cfg(miri)]` only (lib.rs:26-27) | n/a (test referenced files missing) | `cargo test -p vb_storage --lib codec_miri_tests` | not run (file gated to `cfg(miri)`) | UNKNOWN | closure-claimed files absent; no defective pattern in current source; cannot verify "fail-closed for unknown feature" assertion |
| vb-0qida | P2 | PATCHED — `FrameSeedAccumulator::record_action_failed` (recovery/replay/summary.rs:684-691) explicitly calls `self.pending_actions.remove(&(action, step))`; explicit `ActionFailedEvent` arm at recovery/replay/summary.rs:185-190 + 514-516 | `recovery::replay::summary::tests::action_failed_event_increments_actions_resolved_only` and 6 `apply_summary_event_*` tests | `cargo test -p vb_storage --lib action_failed_event_increments_actions_resolved_only apply_summary_event --no-fail-fast` | `1 + 6 passed; 0 failed` | PATCHED | recovery/replay/summary.rs:684-689 removes from pending set; 7 tests pass; no Holzman violation (uses `HashSet::remove`, `saturating_add`) |
| vb-0z0be | P1 | PATCHED — `StorageRuntimeJournal::append_sequenced` at runtime/journal/chunk_002.rs:293-297: maps via `Self::storage_event(event, seq)?` then persists via `self.append_storage_event(&storage_event)?`; no early return between mapping and persistence | `journal::tests::storage_runtime_journal_maps_lifecycle_events_in_sequence` + `journal::tests::queued_storage_runtime_journal_flushes_mapped_events_to_fjall` (existed pre-fix and continue to pass) | `cargo test -p vb_runtime --lib --no-fail-fast` | `1734 passed; 0 failed` | PATCHED | runtime/journal/chunk_002.rs:293-297 atomic; `?` chains both steps; `#![forbid(unsafe_code)]` at chunk_002.rs head; saturating counter in `apply_summary_event` (summary.rs:46) |
| vb-11ti1 | P1 | PATCHED — production `vb_runtime` lib has zero `as uXX`/`as fXX` casts in non-test source outside `#[allow(clippy::as_conversions)]` blocks (runtime.rs:450 explicit `#[allow]` for f32 ratio metric) and zero `arithmetic_side_effects` violations | n/a (lint gate is the test) | `cargo clippy -p vb_runtime --lib -- -D clippy::as_conversions -D clippy::arithmetic_side_effects` | `Finished` 0 errors | PATCHED | lint-clean lib; remaining `as` are in tests/kani/verification modules (allowed via `cfg_attr(test, allow)` in lib.rs:13-43); 1734 lib tests pass |
| vb-12yr3 | P3 | PATCHED — `admit_artifact_run_with_certificate_floor` now runs per-required subset check first (admission.rs:740-742), THEN gates on cardinality (admission.rs:743-750) and returns typed `AdmissionError::CapabilityCountMismatch { required_count, granted_count }` instead of fabricating a `CapabilityDenied` | `admission::tests::admit_artifact_run_count_mismatch_under_grant_returns_typed_error_not_per_cap_denial`, `admit_artifact_run_count_mismatch_returns_typed_error_not_capability_denied` (admission/tests.rs:164-218, 227-289) | `cargo test -p vb_runtime --lib count_mismatch --no-fail-fast` | `2 passed; 0 failed` | PATCHED | admission.rs:740-750 ordering correct; no `unsafe`/`unwrap`/`panic`; `forbid(unsafe_code)` at file head |
| vb-1rqz7.14 | P0 | NOT-PATCHED — `keys.rs::sequenced_run_key` (lines 416-429) and `journal_key` (412-414) encode the seq without validating against `u64::MAX`; only the decode path (`decode_storage_key` keys.rs:361-362) rejects `ReservedSeqSentinel`. `encode_key_into` (lines 156-178) has no MAX guard. | none — existing test `run_event_key_with_max_values` (keys/tests.rs:491-497) explicitly asserts encoding SUCCEEDS with `EventSeq::MAX`, which contradicts the fix intent | `cargo test -p vb_storage --lib run_event_key_with_max_values` | `1 passed; 0 failed` (but the test codifies the bug behavior) | NOT-PATCHED | keys.rs:412-429 encode path unguarded; no negative-path test asserting MAX rejection |
| vb-1rqz7.27 | P0 | PATCHED — `trim_events_for_run` returns `TrimStatus::NoOp` BEFORE `batch.commit()` when `deleted_count == 0` (trimming/logic.rs:103-110); batch.commit() only called on non-empty deletes (line 112) | `trimming::tests::trim_zero_deletes_returns_noop_when_skip_noop_disabled` (trimming/tests.rs:179-218) + `trim_given_run_already_trimmed_is_noop` (line 133-173) | `cargo test -p vb_storage --lib trim_zero_deletes_returns_noop_when_skip_noop_disabled --no-fail-fast` | `1 passed; 0 failed` | PATCHED | trimming/logic.rs:103-119 short-circuit; 1651-line trimming/tests.rs covers NoOp path; `#![forbid(unsafe_code)]` at file head |
| vb-1rqz7.28 | P0 | PATCHED — `TrimError::diagnostic_code` (trimming/mod.rs:65-73) delegates `Self::Journal(inner) => inner.diagnostic_code()` so `TrimError::Journal(JournalError::WrongRun { .. })` returns the inner code, not `FJALL_CODE` | `trimming::tests::journal_wrapped_error_delegates_to_inner_diagnostic_code` (trimming/tests.rs:1234-1254) | `cargo test -p vb_storage --lib journal_wrapped_error_delegates_to_inner_diagnostic_code --no-fail-fast` | `1 passed; 0 failed` | PATCHED | trimming/mod.rs:68 explicit `inner.diagnostic_code()` delegation; test asserts `!= FJALL_CODE` |
| vb-1rqz7.3 | P0 | NOT-PATCHED — `derive_lifecycle_state_from_events` (journal/incident.rs:146-171) STILL has the `_ => LifecycleState::Active` wildcard at line 168. Explicit handling is MISSING for: `ActionScheduledTicket`, `ActionCompletedEnvelope`, `WaitResolvedEvent`, `RunKilled` (close reason claims `RunKilled → Cancelled` but the arm is absent), `AskTimedOutEvent`. The `#[allow(unreachable_patterns)]` annotation at line 145 suppresses the compiler's exhaustive-coverage check. | none — the `journal/regression_tests_vb_1rqz7.rs` file referenced in the closure reason does not exist; only `analyze_incident_events` tests (t_001-t_013) exist, none for `derive_lifecycle_state_from_events` | `cargo test -p vb_storage --lib derive_lifecycle_state --no-fail-fast` | no tests match (verified via grep) | NOT-PATCHED | journal/incident.rs:168 wildcard + 5 missing variant arms; declared regression test file absent |
| vb-1rqz7.4 | P0 | NOT-PATCHED — `recover_full_journal` (recovery/replay/core.rs:196-219, line 203) still calls `journal.events_for_run(run)`, NOT `events_for_run_full`. The `events_for_run_full` function does not exist anywhere in the codebase; only `events_for_run` (replay.rs:53-55), `events_for_run_bounded` (lines 72-85), `events_for_run_from` (lines 88-119) exist, all of which start from `latest_durable_snapshot_seq` or a caller-supplied `start_seq` — NOT from `EventSeq::ZERO` | none — `recover_full_journal_reads_history_before_snapshot` test named in the closure reason does not exist; only `recover_full_journal_returns_no_recovery_data_when_empty` (recovery/tests.rs:1772) exists | `cargo test -p vb_storage --lib recover_full_journal --no-fail-fast` | `7 tests passed` but none validate full-history replay across a snapshot | NOT-PATCHED | recovery/replay/core.rs:203 unchanged; `events_for_run_full` not defined anywhere |

## Summary

- bugs-checked: 10
- pass-count (PATCHED): 5 — vb-0qida, vb-0z0be, vb-11ti1, vb-12yr3, vb-1rqz7.27, vb-1rqz7.28 (6)
- fail-count (NOT-PATCHED): 3 — vb-1rqz7.14, vb-1rqz7.3, vb-1rqz7.4
- partial-count: 0
- unknown-count: 1 — vb-06t25

Correction: PATCHED = 6 (vb-0qida, vb-0z0be, vb-11ti1, vb-12yr3, vb-1rqz7.27, vb-1rqz7.28); NOT-PATCHED = 3; UNKNOWN = 1.

## Top-3 NOT-PATCHED IDs with one-line reasons

1. **vb-1rqz7.14 (SC-002)** — `keys.rs::sequenced_run_key` (lines 412-429) does NOT validate `EventSeq::MAX` before encoding; only the decode path rejects the sentinel; the existing test `run_event_key_with_max_values` codifies the bug behavior by asserting encoding succeeds.
2. **vb-1rqz7.3 (SJ-005)** — `derive_lifecycle_state_from_events` at `journal/incident.rs:146-171` still has the `_ => LifecycleState::Active` wildcard at line 168; explicit arms are missing for `ActionScheduledTicket`, `ActionCompletedEnvelope`, `WaitResolvedEvent`, `RunKilled`, `AskTimedOutEvent`; the closure-cited regression test file `journal/regression_tests_vb_1rqz7.rs` does not exist.
3. **vb-1rqz7.4 (SR-001)** — `recover_full_journal` at `recovery/replay/core.rs:203` still calls `journal.events_for_run(run)` (snapshot-tail optimized); `events_for_run_full` is not defined anywhere in the codebase; the closure-cited regression test `recover_full_journal_reads_history_before_snapshot` does not exist.

## UNKNOWN

- **vb-06t25** — closure reason names `crates/vb_storage/src/codec_miri_tests_compile_check.rs` and `scripts/build-check-codec-miri-features.sh`; neither file exists. The current `codec_miri_tests.rs` has no `MOK` const or `include_str!` target and is `#[cfg(miri)]` only — there is no defective pattern to verify against, so the verdict cannot be assigned without the original failing input.

## Holzman cross-checks (all green where PATCHED)

- `#![forbid(unsafe_code)]` present at every fix-site file head (keys.rs, recovery/replay/summary.rs, runtime/journal/chunk_002.rs, runtime/admission.rs, trimming/logic.rs, trimming/mod.rs).
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg!` introduced in any PATCHED path.
- `saturating_add` used for all counter increments (summary.rs:34-67, trimming/logic.rs:99, 158, 196, 240).
- Index/slice access (`[9..17]`, `[1..9]`, `[9..]`) uses `.get(...).ok_or(...)?` with explicit bound checks (trimming/logic.rs:26-32, 87-92, 231-237).
- Casts `as` in production runtime use `#[allow(clippy::as_conversions)]` only for documented f32 metric narrowing (runtime.rs:449-450) — single residual.
- No YAML/JSON/HTTP in the production paths touched by this chunk.

Output file path: `/home/lewis/src/velvet-ballistics/to-fix/wave1/agent-00-holzman-rust-A.md`
