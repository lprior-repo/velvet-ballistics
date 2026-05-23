# Proof-to-Implementation Bridge: vb-jpq7.3

## Bridge Disposition

**READY FOR PROOF-REVIEWER CONSUMPTION WITH EXPLICIT LIMITATIONS** for closure. This bridge does not self-approve; approval authority remains the independent proof-plan/proof-review artifacts.

The refreshed behavior-test bridge is accepted by proof-plan and proof-review under the scoped interpretation recorded in `.beads/vb-jpq7.3/proof-plan-review.md` (`review_state: approved`, `STATUS: APPROVED`) and `.beads/vb-jpq7.3/proof-review.md` (`APPROVED with explicit limitations`, `STATUS: APPROVED`). Current evidence includes a passing Kani inventory and a scoped Kani run: all 12 invoked harnesses verify, but only the 9 `kani_recovery_hydrate::*` harnesses map directly to vb-jpq7.3 replay/recovery obligations; the 3 `kani_admission::*` harnesses are adjacent admission evidence and must not be used to close storage replay/recovery claims. Fresh canonical `moon ci` also passes after the latest versioned slot-write extra envelope, full-journal taint, scanner, runtime encode, and supply-chain edits.

Current global readiness: latest closure **`moon ci` PASS** at `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z` (`Tasks: 25 completed (3 cached)`, `12169 tests run: 12169 passed (5 slow), 0 skipped`, test-integrity PASS, panic-surface and ignored-fallible-results `NoViolationFound`, supply-chain completed). Current Kani evidence: `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed` contains 12 successful scoped harness executions with `VERIFICATION:- SUCCESSFUL` / `Complete - 1 successfully verified harnesses, 0 failures, 1 total.` summaries; inventory artifacts are `.beads/vb-jpq7.3/kani-list.json` and `.beads/vb-jpq7.3/kani-list.md` (`54` vb_storage standard harnesses, `0` contract harnesses). Canonical schema repair is independently accepted in `.beads/vb-jpq7.3/proof-plan-review.md`: `proof-obligations.planned.jsonl` has 16 valid `proof-obligation/v1` rows, `verifier-lane-decisions.jsonl` has 72 valid lane rows, and `verifier-lane-review.jsonl` has 72 accepted lane-review rows.

Approval limitations that must be preserved: Verus is auxiliary/spec-seam evidence only, not production-bound exec proof; TLA+ is bounded abstract temporal evidence with `MaxSeq = 3`; Kani proves scoped allocation-free seams only; live Fjall, `RunFrame`, codec, range iteration, allocation, replay, and hydration behavior are closed by behavior tests, source scan evidence, and trusted-base declarations rather than by formal methods alone.

## Canonical Obligation Coverage

All 16 repaired `proof-obligation/v1` rows are represented by the bridge rows below:

- `obl-tla-recovery-001` -> `POT-REPLAY-001`, `POT-SNAPSHOT-001`; bounded temporal only (`MaxSeq = 3`).
- `obl-verus-replay-001` -> `POT-REPLAY-001`; auxiliary replay seam only.
- `obl-verus-recovery-001` -> `POT-TAINT-001`; auxiliary recovery model only.
- `obl-kani-replay-next-001` -> `POT-REPLAY-001`; scoped next-seq/tail metadata seams only.
- `obl-kani-replay-limit-001` -> `POT-REPLAY-002`; scoped checked-count decision seam only.
- `obl-kani-snapshot-metadata-001` -> `POT-SNAPSHOT-001`; scoped metadata run-mismatch seam only.
- `obl-kani-taint-001` -> `POT-TAINT-001`; scoped taint-read lattice seam only.
- `obl-kani-recovery-presence-001` -> `POT-TAINT-001`; scoped recovery-presence seam only.
- `obl-kani-admission-001` -> adjacent admission evidence only; not used to close storage replay/recovery behavior.
- `obl-test-storage-replay-001` -> `POT-REPLAY-001`, `POT-REPLAY-002`, `POT-REPLAY-003`, `POT-SNAPSHOT-001`.
- `obl-test-storage-recovery-001` -> `POT-TAINT-001`.
- `obl-test-storage-trimming-001` -> `POT-SNAPSHOT-001`.
- `obl-test-storage-durability-001` -> `POT-DURABILITY-001`.
- `obl-test-workspace-contract-001` -> all public contract behavior rows; current public contract has 11 deterministic scenarios.
- `obl-source-scan-discard-001` -> `POT-DISCARD-001`.
- `obl-moon-ci-001` -> `POT-GLOBAL-001`; latest raw evidence is `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`.

## Obligation Mapping

### POT-REPLAY-001 — strict snapshot tail / sequence gap

- **Source refs**
  - `crates/vb_storage/src/journal/replay.rs:51-55` — `FjallJournal::events_for_run`
  - `crates/vb_storage/src/journal/replay.rs:57-70` — `FjallJournal::events_for_run_bounded`
  - `crates/vb_storage/src/journal/replay.rs:63-69` — latest snapshot authority read and `snapshot.seq + 1` via `codec::next_seq`
  - `crates/vb_storage/src/journal/replay.rs:73-104` — bounded range replay and validation loop
  - `crates/vb_storage/src/journal/replay.rs:108-117` — `validate_replay_sequence`
  - `crates/vb_storage/src/codec/mod.rs:46-50` — checked next-sequence overflow
  - `crates/vb_storage/src/codec/mod.rs:53-70` — `validate_replayed_event` emits `WrongRun` / `SequenceGap`
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:1696` — `events_for_run_detects_missing_first_tail_event_after_snapshot`
  - `crates/vb_storage/src/journal/tests.rs:1729` — `events_for_run_without_snapshot_rejects_missing_initial_sequence`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:125` — `given_first_tail_event_is_missing_when_replaying_run_then_sequence_gap_points_after_snapshot`
- **Refinement / formal refs**
   - `verification/verus/vb_jpq724_events_for_run_production.rs` — strengthened auxiliary seam contract proves typed snapshot-authority error propagation, checked next-seq overflow, exact first-tail equality when non-empty, run preservation, and strict ordering (`5 verified, 0 errors`).
   - `verification/tla/EngineYamlRecovery.tla` — strengthened bounded model includes MaxSeq overflow, snapshot+1 tail start, missing-first-tail `SequenceGap`, and corrupt/latest snapshot statuses (`87074 states generated`, `43531 distinct`, depth 6, no errors).
   - `crates/vb_storage/src/kani_recovery_hydrate.rs` — `replay_next_seq_overflow_boundary`, `tail_seq_scan_matches_any_metadata_batch_len_le_4`, and `tail_run_scan_matches_any_metadata_batch_len_le_4` verify bounded sequence/metadata seams under Kani.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage events_for_run`
  - `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`
  - Formal lane commands if claimed: `verus verification/verus/vb_jpq724_events_for_run_production.rs`; `tlc -workers 1 -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla`
  - Kani evidence command subset: `cargo kani --harness kani_recovery_hydrate::replay_next_seq_overflow_boundary --exact`; `cargo kani --harness kani_recovery_hydrate::tail_seq_scan_matches_any_metadata_batch_len_le_4 --exact`; `cargo kani --harness kani_recovery_hydrate::tail_run_scan_matches_any_metadata_batch_len_le_4 --exact` from `crates/vb_storage` (raw log `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`).
- **Bridge status:** behavior mapped; TLA+/Verus/Kani accepted with explicit limitations by proof-review.

### POT-REPLAY-002 — bounded replay

- **Source refs**
  - `crates/vb_storage/src/journal/core.rs:23-48` — `EventReplayLimit`
  - `crates/vb_storage/src/journal/replay.rs:57-70` — bounded replay API
  - `crates/vb_storage/src/journal/replay.rs:119-143` — checked observed count, `TooManyEvents`, `try_reserve`
  - `crates/vb_storage/src/error/mod.rs:194-213` — `TooManyEvents` / `ReplayAllocationFailed`
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:1668` — `events_for_run_bounded_rejects_over_limit`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:80` — `given_explicit_replay_limit_when_more_events_exist_then_too_many_events_and_code_are_returned`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:188` — `given_zero_replay_limit_when_constructed_then_limit_is_rejected_before_replay`
- **Refinement / formal refs:** `crates/vb_storage/src/kani_recovery_hydrate.rs::replay_push_limit_decision_matches_checked_count` verifies the allocation-free replay push limit decision over arbitrary `usize` current length and positive raw limit.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage events_for_run`
  - `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`
  - Kani evidence command: `cargo kani --harness kani_recovery_hydrate::replay_push_limit_decision_matches_checked_count --exact` from `crates/vb_storage` (raw log `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`).
- **Bridge status:** behavior mapped; Kani seam mapped for limit arithmetic/overflow; live Fjall collection remains behavior-tested.

### POT-REPLAY-003 — range starts after snapshot / bounded scan

- **Source refs**
  - `crates/vb_storage/src/journal/replay.rs:63-70` — tail start chosen from validated latest snapshot
  - `crates/vb_storage/src/journal/replay.rs:83-94` — `run_event_key(run, start_seq)` lower-bound scan and prefix termination
  - `crates/vb_storage/src/journal/replay.rs:95-102` — only tail values are decoded and collected
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:1871` — `events_for_run_skips_corrupt_pre_snapshot_event_by_key_range`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:375` — `given_snapshot_after_many_old_events_when_replaying_then_pre_snapshot_work_does_not_exhaust_limit`
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
  - `crates/vb_storage/src/journal/replay.rs:63` — replay propagates snapshot-authority errors before tail replay
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:1750` — `events_for_run_rejects_corrupt_latest_snapshot_before_skipping_events`
  - `crates/vb_storage/src/journal/tests.rs:1792` — `events_for_run_rejects_latest_snapshot_payload_digest_mismatch_before_tail_replay`
  - `crates/vb_storage/src/journal/tests.rs:1837` — `events_for_run_rejects_latest_snapshot_postcard_decode_failure_before_tail_replay`
  - `crates/vb_storage/src/trimming/tests.rs:362` — `latest_durable_snapshot_seq_rejects_payload_run_mismatch`
  - `crates/vb_storage/src/trimming/tests.rs:394` — `latest_durable_snapshot_seq_rejects_payload_seq_mismatch`
  - `crates/vb_storage/src/trimming/tests.rs:311` — `latest_durable_snapshot_seq_returns_highest_seq`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:200` — `given_snapshot_index_read_fails_when_events_for_run_starts_then_error_is_not_erased`
- **Refinement / formal refs**
   - `verification/tla/EngineYamlRecovery.tla` — strengthened bounded model includes `bad_magic`, `digest_mismatch`, `postcard_failed`, `wrong_run`, `wrong_seq`, overflow, and exact tail-start transitions.
   - `crates/vb_storage/src/kani_recovery_hydrate.rs::snapshot_metadata_rejects_run_mismatch` — auxiliary Kani seam verifies snapshot metadata run mismatch is rejected and preserves run/seq values.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage events_for_run`
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage trimming`
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage latest_durable_snapshot_seq`
  - Formal lane command if claimed: `tlc -workers 1 -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla`
  - Kani evidence command: `cargo kani --harness kani_recovery_hydrate::snapshot_metadata_rejects_run_mismatch --exact` from `crates/vb_storage` (raw log `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`).
- **Bridge status:** behavior mapping repaired and accepted; TLA/Kani bridge accepted with explicit limitations by proof-review.

### POT-TAINT-001 — taint read fail-closed

- **Source refs**
  - `crates/vb_storage/src/recovery/hydrate_support.rs:169-173` — `apply_tail_events`
  - `crates/vb_storage/src/recovery/hydrate_support.rs:245-268` — slot write tail event path
   - `crates/vb_storage/src/recovery/hydrate_support.rs:253-259` — `read_taint` failure maps to `RecoveryError::SlotTaintReadFailed`; only `SlotUninitialized` defaults to `Clean`
   - `crates/vb_storage/src/slot_extra.rs` — versioned slot-write extra envelope distinguishes current taint metadata from legacy frame extra bytes
   - `crates/vb_storage/src/recovery/replay/summary.rs:428-470` — full-journal slot writes decode versioned durable taint envelope metadata and return `RecoveryError::CorruptSlotTaint { slot }` on corrupt prefixed envelope payloads; legacy frame extra bytes are classified separately and do not become corrupt taint
   - `crates/vb_storage/src/recovery/types.rs:69-74` — `RecoveryError::SlotTaintReadFailed`
   - `crates/vb_storage/src/recovery/types.rs:75-80` — `RecoveryError::CorruptSlotTaint`
   - `crates/vb_runtime/src/journal/chunk_002.rs:227-235` — runtime slot writes encode taint plus optional frame extra into the versioned envelope
   - `crates/vb_runtime/src/primitives/collect.rs:232-294` — collect hydration unwraps current envelopes and preserves legacy collect frame-extra bytes
- **Behavior tests**
   - `crates/vb_storage/src/recovery/tests.rs:2149` — `apply_tail_events_fails_closed_when_taint_read_fails`
   - `crates/vb_storage/src/recovery/tests.rs:1821` — `hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata`
   - `crates/vb_storage/src/recovery/tests.rs:1852` — `hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar`
   - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:221` — `given_public_hydration_tail_slot_cannot_be_dimensioned_when_recovery_runs_then_clean_taint_is_not_defaulted`
   - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:262` — `given_full_journal_slot_taint_metadata_is_corrupt_when_hydrating_then_recovery_fails_closed`
   - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:298` — `given_legacy_collect_frame_extra_when_hydrating_full_journal_then_extra_is_not_corrupt_taint`
   - `crates/vb_runtime/src/collect_tests.rs` — `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page`
   - `crates/vb_runtime/src/journal/tests/chunk_002.rs` — `storage_runtime_journal_maps_action_wait_and_ask_events`
- **Refinement / formal refs**
   - `verification/verus/recovery_hydration_contracts.rs:76-79`, `:116-132`, `:195-202` — auxiliary abstract model; does not include `RecoveryError::SlotTaintReadFailed` or bind to `apply_tail_events`.
   - `crates/vb_storage/src/kani_recovery_hydrate.rs` — `slot_taint_resolution_fails_closed_on_read_failure`, `slot_taint_resolution_defaults_clean_only_for_uninitialized`, and `slot_taint_resolution_preserves_existing_taint` verify the production taint-resolution seam used by `apply_tail_events`.
- **Exact commands**
   - `rustup run nightly-2026-04-28 cargo test -p vb_storage apply_tail_events_fails_closed_when_taint_read_fails`
   - `rustup run nightly-2026-04-28 cargo test -p vb_storage hydrate_run_frame_from_events`
   - `rustup run nightly-2026-04-28 cargo test -p vb_runtime collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page`
   - `rustup run nightly-2026-04-28 cargo test -p vb_runtime storage_runtime_journal_maps_action_wait_and_ask_events`
   - `rustup run nightly-2026-04-28 cargo test -p vb_storage recovery`
   - `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`
  - Formal lane command if claimed: `verus verification/verus/recovery_hydration_contracts.rs`
  - Kani evidence command subset: `cargo kani --harness kani_recovery_hydrate::slot_taint_resolution_fails_closed_on_read_failure --exact`; `cargo kani --harness kani_recovery_hydrate::slot_taint_resolution_defaults_clean_only_for_uninitialized --exact`; `cargo kani --harness kani_recovery_hydrate::slot_taint_resolution_preserves_existing_taint --exact` from `crates/vb_storage` (raw log `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`).
- **Bridge status:** behavior mapped; Kani taint seam is production-linked through `resolve_slot_taint_read`; Verus artifact remains auxiliary.
- **Full-journal taint/extra status:** behavior mapped to typed `RecoveryError::CorruptSlotTaint { slot }` for corrupt prefixed taint envelopes, and to legacy frame-extra compatibility for pre-envelope collect extras. Corrupt durable taint sidecar bytes are no longer erased into legacy `Clean`, and valid legacy collect/frame extra bytes are not misclassified as taint corruption.

### POT-DURABILITY-001 — explicit close persist result

- **Source refs**
  - `crates/vb_storage/src/journal/core.rs:140-152` — `FjallJournal::close` returns `persist_strict()` result
  - `crates/vb_storage/src/journal/core.rs:154-162` — test-only persist failure hook
  - `crates/vb_storage/src/journal/core.rs:165-170` — `Drop` does not call/discard `close()`
  - `crates/vb_storage/src/journal/append.rs:26-34` — `persist_strict` returns storage/test-hook errors
  - `crates/vb_storage/src/error/mod.rs:191-193` — `JournalError::StrictDurabilityFailed`
- **Behavior tests**
  - `crates/vb_storage/src/journal/tests.rs:2416` — `close_propagates_persist_errors`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:161` — `given_close_after_unpersisted_append_when_reopened_then_event_is_observable`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:355` — `given_journal_shutdown_when_durability_barrier_fails_then_drop_does_not_discard_result`
- **Refinement / formal refs:** none accepted for close/persist error propagation or `Drop` non-discard behavior.
- **Exact commands**
  - `rustup run nightly-2026-04-28 cargo test -p vb_storage close_propagates_persist_errors`
  - `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract`
- **Bridge status:** behavior mapped; no separate refinement harness.

### POT-DISCARD-001 — no silent discard

- **Source refs**
   - `scripts/check-ignored-fallible-results.sh` — static production source scan
   - `crates/vb_storage/src/slot_extra.rs` — envelope encode/decode result paths
   - `crates/vb_storage/src/journal/core.rs:165-170` — no persistence discard in `Drop`
   - `crates/vb_storage/src/journal/replay.rs:63`, `:90-100`, `:125-140` — replay propagates snapshot/range/decode/validation errors
   - `crates/vb_storage/src/recovery/hydrate_support.rs:253-259` — taint read failures are not erased
   - `crates/vb_storage/src/recovery/replay/summary.rs:428-470` — corrupt full-journal prefixed taint envelope metadata is not erased via `.ok()` and legacy frame extra is disambiguated
   - `crates/vb_runtime/src/journal/chunk_002.rs` and `chunk_003.rs` — runtime journal sidecar encoding/storage event conversion returns typed errors instead of silently dropping fallible encode failures
- **Behavior/static refs**
   - `scripts/check-ignored-fallible-results.sh` — source-scan evidence
   - `scripts/check-ignored-fallible-results.sh` fixtures — embedded `.ok()` and split-chain `.ok()` on recognized fallible sources fail the scanner
   - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:200` — `given_snapshot_index_read_fails_when_events_for_run_starts_then_error_is_not_erased`
   - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:355` — `given_journal_shutdown_when_durability_barrier_fails_then_drop_does_not_discard_result`
   - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:262` — `given_full_journal_slot_taint_metadata_is_corrupt_when_hydrating_then_recovery_fails_closed`
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
- **Evidence status:** `verification-ledger.jsonl:35` reports **PASS / GLOBAL_PASS** with latest raw Moon output path `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`.
- **Bridge status:** mapped and green for global readiness.

## Preserved Limitations / Non-Formal Closure Boundaries

1. TLA+ lane is approved only as bounded abstract temporal evidence (`MaxSeq = 3`) for fail-closed status transitions, snapshot authority failures, typed errors, overflow, and `snapshot.seq + 1`; it is not live Fjall replay evidence.
2. Verus replay and recovery artifacts are approved only as auxiliary/spec-seam evidence. They must not be cited as implementation-bound exec proofs of `FjallJournal`, `RunFrame`, Fjall, postcard, codec internals, or live replay/hydration behavior.
3. Kani passes for scoped allocation-free seams only. It does not model live Fjall handles, live range iteration, codec internals, allocation behavior, or full `RunFrame` hydration. Kani inventory reports `0` contract harnesses, so these are standard proof harnesses, not Kani function-contract proofs.
4. Live Fjall / `RunFrame` / codec behavior is intentionally closed by behavior tests, source-scan evidence, and trusted-base declarations. Rows with no separate refinement harness (`POT-REPLAY-003`, `POT-DURABILITY-001`) remain accepted under that proof-review limitation.
5. Durable raw evidence exists for the latest canonical Moon pass (`/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`) and the scoped Kani run (`/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`); several scoped cargo-test rows remain summarized in `verification-ledger.jsonl` as prior/current session output and are accepted by the final proof review as behavior-test/source-scan closure.

## Proof-Reviewer Handoff Inputs

- `.beads/vb-jpq7.3/proof-obligations.planned.jsonl`
- `.beads/vb-jpq7.3/proof-plan-review.md`
- `.beads/vb-jpq7.3/proof-to-implementation.md`
- `.beads/vb-jpq7.3/proof-review.md`
- `.beads/vb-jpq7.3/traceability-matrix.jsonl`
- `.beads/vb-jpq7.3/verification-ledger.jsonl`
- `.beads/vb-jpq7.3/verifier-lane-decisions.jsonl`
- `.beads/vb-jpq7.3/global-readiness-report.md`
- `.beads/vb-jpq7.3/kani-list.json`
- `.beads/vb-jpq7.3/kani-list.md`
- `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`
- `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`
- `verification/tla/EngineYamlRecovery.tla`
- `verification/tla/EngineYamlRecovery.cfg`
- `verification/verus/vb_jpq724_events_for_run_production.rs`
- `verification/verus/recovery_hydration_contracts.rs`
