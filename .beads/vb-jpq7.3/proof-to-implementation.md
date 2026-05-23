# Proof-to-Implementation Bridge: vb-jpq7.3

## Verdict

**REJECT** for closure.

The refreshed behavior-test bridge is stronger after snapshot-authority repairs, including the four new tests requested below. However, bridge closure still fails because the required formal lanes remain auxiliary/disconnected, `POT-GLOBAL-001` is blocked by `moon ci`, and the global obligation is still missing from `traceability-matrix.jsonl`.

Current global blocker is **`moon ci`**, not formatting. `cargo fmt --all -- --check` is now recorded as PASS; `moon ci` is recorded as FAIL for production panic-surface in `crates/vb_codegen/src/parity.rs:438`, `:444` and unrelated workspace-test dead-code warnings.

## Obligation Mapping

### POT-REPLAY-001 — strict snapshot tail / sequence gap

- **Source refs**
  - `crates/vb_storage/src/journal/replay.rs:14` — `FjallJournal::events_for_run`
  - `crates/vb_storage/src/journal/replay.rs:19-23` — `FjallJournal::events_for_run_bounded`
  - `crates/vb_storage/src/journal/replay.rs:24-31` — latest snapshot authority read and `snapshot.seq + 1` via `codec::next_seq`
  - `crates/vb_storage/src/journal/replay.rs:35-63` — bounded range replay and validation loop
  - `crates/vb_storage/src/journal/replay.rs:69-77` — `validate_replay_sequence`
  - `crates/vb_storage/src/codec/mod.rs:46-50` — checked next-sequence overflow
  - `crates/vb_storage/src/codec/mod.rs:53-70` — `validate_replayed_event` emits `WrongRun` / `SequenceGap`
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:1696` — `events_for_run_detects_missing_first_tail_event_after_snapshot`
  - `crates/vb_storage/src/journal/tests.rs:1729` — `events_for_run_without_snapshot_rejects_missing_initial_sequence`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:85` — `given_first_tail_event_is_missing_when_replaying_run_then_sequence_gap_points_after_snapshot`
- **Refinement / formal refs**
  - `verification/verus/vb_jpq724_events_for_run_production.rs:148-189`, `:195-257` — auxiliary only; does not bind to production exec functions and does not prove exact first-tail equality or typed errors.
  - `verification/tla/EngineYamlRecovery.tla:45-49` — auxiliary lifecycle replay only; does not model Rust `EventSeq`, snapshot-tail arithmetic, or typed `SequenceGap`.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage events_for_run`
  - `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`
  - Formal lane commands if claimed: `verus verification/verus/vb_jpq724_events_for_run_production.rs`; `tlc -workers 1 -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla`
- **Bridge status:** behavior mapped; formal proof bridge remains disconnected.

### POT-REPLAY-002 — bounded replay

- **Source refs**
  - `crates/vb_storage/src/journal/core.rs:23-48` — `EventReplayLimit`
  - `crates/vb_storage/src/journal/replay.rs:19-23` — bounded replay API
  - `crates/vb_storage/src/journal/replay.rs:80-109` — checked observed count, `TooManyEvents`, `try_reserve`
  - `crates/vb_storage/src/error/mod.rs:194-213` — `TooManyEvents` / `ReplayAllocationFailed`
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:1668` — `events_for_run_bounded_rejects_over_limit`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:40` — `given_explicit_replay_limit_when_more_events_exist_then_too_many_events_and_code_are_returned`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:148` — `given_zero_replay_limit_when_constructed_then_limit_is_rejected_before_replay`
- **Refinement / formal refs:** none accepted for exact bound/allocation behavior.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage events_for_run`
  - `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`
- **Bridge status:** behavior mapped; no separate refinement harness.

### POT-REPLAY-003 — range starts after snapshot / bounded scan

- **Source refs**
  - `crates/vb_storage/src/journal/replay.rs:24-31` — tail start chosen from validated latest snapshot
  - `crates/vb_storage/src/journal/replay.rs:44-55` — `run_event_key(run, start_seq)` lower-bound scan and prefix termination
  - `crates/vb_storage/src/journal/replay.rs:56-63` — only tail values are decoded and collected
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:1792` — `events_for_run_skips_corrupt_pre_snapshot_event_by_key_range`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:238` — `given_snapshot_after_many_old_events_when_replaying_then_pre_snapshot_work_does_not_exhaust_limit`
- **Refinement / formal refs:** none accepted for Fjall lexicographic lower-bound behavior; `trusted-base-plan.md` records Fjall ordering/prefix behavior as trusted base.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage events_for_run`
  - `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`
- **Bridge status:** behavior mapped; refinement is trusted-base/source-review only.

### POT-SNAPSHOT-001 — validated snapshot authority

- **Source refs**
  - `crates/vb_storage/src/trimming/logic.rs:17-56` — `FjallJournal::latest_durable_snapshot_seq`
  - `crates/vb_storage/src/trimming/logic.rs:21-36` — iterates snapshot records and calls `decode_record`
  - `crates/vb_storage/src/trimming/logic.rs:37-48` — rejects payload `run`/`seq` mismatch against key authority
  - `crates/vb_storage/src/journal/replay.rs:24` — replay propagates snapshot-authority errors before tail replay
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:1750` — `events_for_run_rejects_corrupt_latest_snapshot_before_skipping_events`
  - `crates/vb_storage/src/journal/tests.rs:1786` — `events_for_run_rejects_latest_snapshot_payload_digest_mismatch_before_tail_replay`
  - `crates/vb_storage/src/journal/tests.rs:1831` — `events_for_run_rejects_latest_snapshot_postcard_decode_failure_before_tail_replay`
  - `crates/vb_storage/src/trimming/tests.rs:362` — `latest_durable_snapshot_seq_rejects_payload_run_mismatch`
  - `crates/vb_storage/src/trimming/tests.rs:394` — `latest_durable_snapshot_seq_rejects_payload_seq_mismatch`
  - `crates/vb_storage/src/trimming/tests.rs:289` — `latest_durable_snapshot_seq_returns_highest_seq`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:160` — `given_snapshot_index_read_fails_when_events_for_run_starts_then_error_is_not_erased`
- **Refinement / formal refs**
  - `verification/tla/EngineYamlRecovery.tla:25-43` — auxiliary lifecycle evidence only; does not model snapshot record decode, digest failure, postcard decode failure, or key/payload run/seq mismatch.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage events_for_run`
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage trimming`
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage latest_durable_snapshot_seq`
  - Formal lane command if claimed: `tlc -workers 1 -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla`
- **Bridge status:** behavior mapping repaired and strong; formal TLA bridge remains disconnected from exact snapshot-authority behavior.

### POT-TAINT-001 — taint read fail-closed

- **Source refs**
  - `crates/vb_storage/src/recovery/hydrate_support.rs:125-129` — `apply_tail_events`
  - `crates/vb_storage/src/recovery/hydrate_support.rs:201-223` — slot write tail event path
  - `crates/vb_storage/src/recovery/hydrate_support.rs:209-215` — `read_taint` failure maps to `RecoveryError::SlotTaintReadFailed`; only `SlotUninitialized` defaults to `Clean`
  - `crates/vb_storage/src/recovery/types.rs:69-74` — `RecoveryError::SlotTaintReadFailed`
- **Behavior tests**
  - `crates/vb_storage/src/recovery/tests.rs:2078` — `apply_tail_events_fails_closed_when_taint_read_fails`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:181` — `given_tail_slot_write_when_recovery_reads_existing_taint_then_read_failure_is_typed_error`
- **Refinement / formal refs**
  - `verification/verus/recovery_hydration_contracts.rs:76-79`, `:116-132`, `:195-202` — auxiliary abstract model; does not include `RecoveryError::SlotTaintReadFailed` or bind to `apply_tail_events`.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage apply_tail_events_fails_closed_when_taint_read_fails`
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage recovery`
  - Formal lane command if claimed: `verus verification/verus/recovery_hydration_contracts.rs`
- **Bridge status:** behavior mapped; Verus artifact remains auxiliary/disconnected.

### POT-DURABILITY-001 — explicit close persist result

- **Source refs**
  - `crates/vb_storage/src/journal/core.rs:140-152` — `FjallJournal::close` returns `persist_strict()` result
  - `crates/vb_storage/src/journal/core.rs:154-162` — test-only persist failure hook
  - `crates/vb_storage/src/journal/core.rs:165-170` — `Drop` does not call/discard `close()`
  - `crates/vb_storage/src/journal/append.rs:26-34` — `persist_strict` returns storage/test-hook errors
  - `crates/vb_storage/src/error/mod.rs:191-193` — `JournalError::StrictDurabilityFailed`
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:2337` — `close_propagates_persist_errors`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:121` — `given_close_after_unpersisted_append_when_reopened_then_event_is_observable`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:218` — `given_journal_shutdown_when_durability_barrier_fails_then_drop_does_not_discard_result`
- **Refinement / formal refs:** none accepted for close/persist error propagation or `Drop` non-discard behavior.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage close_propagates_persist_errors`
  - `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`
- **Bridge status:** behavior mapped; no separate refinement harness.

### POT-DISCARD-001 — no silent discard

- **Source refs**
  - `scripts/check-ignored-fallible-results.sh` — static production source scan
  - `crates/vb_storage/src/journal/core.rs:165-170` — no persistence discard in `Drop`
  - `crates/vb_storage/src/journal/replay.rs:24`, `:52`, `:56-63` — replay propagates snapshot/range/decode/validation errors
  - `crates/vb_storage/src/recovery/hydrate_support.rs:209-215` — taint read failures are not erased
- **Behavior/static refs**
  - `scripts/check-ignored-fallible-results.sh` — source-scan evidence
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:160` — `given_snapshot_index_read_fails_when_events_for_run_starts_then_error_is_not_erased`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:218` — `given_journal_shutdown_when_durability_barrier_fails_then_drop_does_not_discard_result`
- **Refinement / formal refs:** none; this is a static policy scan, not a formal refinement harness.
- **Exact command**
  - `bash scripts/check-ignored-fallible-results.sh`
- **Bridge status:** mapped to static source-scan and behavior evidence.

### POT-GLOBAL-001 — canonical moon CI gate

- **Source refs:** repository-wide gate; no single Rust behavior symbol.
- **Behavior test refs:** none; global readiness obligation.
- **Refinement / formal refs:** none applicable.
- **Exact command**
  - `moon ci`
- **Evidence status:** `verification-ledger.jsonl:2` reports **FAIL / BLOCK_GLOBAL**.
- **Bridge status:** blocked. Also still missing from `traceability-matrix.jsonl`.

## Remaining Bridge Gaps

1. `POT-GLOBAL-001` has no `traceability-matrix.jsonl` row and `moon ci` is failing.
2. Required TLA+ lane remains disconnected from exact vb-jpq7.3 Rust obligations: no bounded `EventSeq`, no latest snapshot key/payload validity model, no digest/postcard failure, no `snapshot.seq + 1` first-tail rule, no typed errors.
3. Required Verus replay artifact remains abstract/disconnected from `FjallJournal::events_for_run(_bounded)` and accepts unconstrained `Err(())`.
4. Required Verus recovery artifact remains auxiliary and does not model `RecoveryError::SlotTaintReadFailed` or bind to `apply_tail_events`.
5. Kani remains unresolved as `candidate-blocker` in `verifier-lane-decisions.jsonl`; no Kani pass or approved waiver is present.
6. Several planned rows still rely on behavior tests plus source review, with no separate accepted refinement harness (`POT-REPLAY-002`, `POT-REPLAY-003`, `POT-DURABILITY-001`).
7. `verification-ledger.jsonl` still records summaries for many PASS rows rather than durable raw output paths, while `proof-review.md` requires raw logs.

## Proof-Reviewer Handoff Inputs

- `.beads/vb-jpq7.3/proof-obligations.planned.jsonl`
- `.beads/vb-jpq7.3/proof-to-implementation.md`
- `.beads/vb-jpq7.3/proof-review.md`
- `.beads/vb-jpq7.3/traceability-matrix.jsonl`
- `.beads/vb-jpq7.3/verification-ledger.jsonl`
- `.beads/vb-jpq7.3/verifier-lane-decisions.jsonl`
- `.beads/vb-jpq7.3/global-readiness-report.md`
- `verification/tla/EngineYamlRecovery.tla`
- `verification/tla/EngineYamlRecovery.cfg`
- `verification/verus/vb_jpq724_events_for_run_production.rs`
- `verification/verus/recovery_hydration_contracts.rs`
