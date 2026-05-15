# Codebase Map — vb-core-replay-divergence-recovery

bead_id: vb-core-replay-divergence-recovery
bead_title: recovery: Prove typed replay divergence and no-YAML recovery
phase: 2
updated_at: 2026-05-15T00:00:00Z
attempt: 1

## Bead Goal Summary

Prove typed replay divergence and no-YAML recovery from persisted source/artifact/journal/snapshot data.
Acceptance criteria:
- Restart/replay never reparses YAML
- Snapshot+tail hydrates full frame state
- Digest mismatch and semantic divergence produce typed errors
- Corrupt/incomplete frame recovery fails closed

## Crates in Scope

### vb_storage (primary)
Path: `crates/vb_storage/src/recovery/`

Key modules:
- `recovery/types.rs` — `RecoveryError` enum with typed variants: `WorkflowSourceDigestMismatch`, `CompiledIrDigestMismatch`, `ActionAbiMismatch`, `PolicyDigestMismatch`, `NonIdempotentActionBlocked`, `ReplayDivergence`, `NoRecoveryData`, `CorruptSnapshot`, `TerminalStateMismatch`, `FrameDimensionOverflow`
- `recovery/recover.rs` — High-level orchestration: `check_workflow_source_digest`, `check_compiled_ir_digest`, `verify_digests`, `recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_all_incomplete_runs`
- `recovery/replay/core.rs` — `replay_events` (divergence detection, action tracking), `recover_full_journal`, `load_snapshot`, `recover_snapshot_plus_tail`, `extract_terminal`, `is_terminal_event`
- `recovery/replay/summary.rs` — `summarize_recovery_events`, `recover_runtime_frame_seed_from_events`, `recover_runtime_frame_seed_from_events_with_workflow`, `RecoveryFrameSeedBuilder`, `FrameSeedAccumulator`
- `recovery/hydrate.rs` — `hydrate_run_frame` (snapshot+tail), `hydrate_run_frame_from_events` (events-only)
- `recovery/hydrate_support.rs` — `apply_tail_events`, `compute_parallel_in_flight`, `decode_snapshot_slots`, `derive_dimensions_from_snapshot_and_tail`

### vb_runtime
Path: `crates/vb_runtime/src/recovery.rs`

Key items:
- `RuntimeRecoveryBoundary` trait — `summary()`, `hydrate_run_frame()`
- `DurableFrameRecoveryBoundary` — backed by `RecoveryFrameSeed`, calls `reject_unsupported_live_frame_state`
- `SummaryRecoveryBoundary` — summary-only, rejects full frame hydration
- `recovery_boundary_from_hydration()` factory
- `hydrate_run_admission_from_events()`
- `RuntimeError::InvalidRecoveryHydration`, `RuntimeError::UnsupportedFullRecoveryHydration`

### vb_core
Path: `crates/vb_core/src/`

Key types used by recovery:
- `RunId`, `StepIdx`, `SlotIdx`, `SlotValue`, `Taint`, `WorkflowDigest`, `EventSeq`
- `RunFrame` — `new`, `write_slot_with_taint`, `mark_running`, `mark_succeeded`, `mark_failed`, `mark_waiting`, `mark_asking`, `set_pc`, `increment_executed`, `set_max_parallel_in_flight`
- `CompiledWorkflow` — `digest()` used for digest verification
- `ReplayEngine::replay_frame_through` — used for typed replay in summary

### vb_storage/journal
Path: `crates/vb_storage/src/journal/`

- `FjallJournal::events_for_run`, `FjallJournal::snapshot`, `FjallJournal::run_headers`
- `JournalEvent` enum — all event types (RunAccepted, StepStarted, SlotWrittenEvent, ActionScheduled, etc.)

## Existing Tests

### vb_storage/tests/recovery_integration.rs
- `full_round_trip_recovery_reads_all_events_in_order`
- `full_round_trip_recovery_reconstructs_summary`
- `full_round_trip_recovery_detects_slot_writes`
- `partial_write_recovery_reads_events_written_before_crash`
- `partial_write_recovery_detects_incomplete_state`
- `partial_write_with_only_run_accepted_is_recoverable`
- `strict_durability_survives_immediate_reopen`
- `journaled_durability_appears_after_flush`
- `action_replay_tracker_reconstructs_from_events`
- `action_replay_tracker_tracks_failed_actions`
- `action_replay_blocks_duplicate_scheduled_action`
- `empty_run_returns_no_recovery_data`
- `terminal_event_identification_after_recovery`
- `recovery_across_multiple_runs_is_isolated`

### vb_storage/tests/replay_resume.rs
- `resume_tail_replays_exactly_when_journal_is_reopened`
- `resume_tail_replay_is_deterministic_when_read_twice`
- `resume_tail_replay_rejects_sequence_gap_before_resume_continuation`

### vb_runtime/src/recovery.rs (unit tests)
- `summary_recovery_boundary_exposes_summary`
- `summary_recovery_boundary_rejects_full_frame_hydration`
- `durable_frame_recovery_boundary_hydrates_minimal_frame_state`
- `durable_frame_recovery_boundary_rejects_inconsistent_seed`
- `durable_frame_recovery_boundary_rejects_unsupported_action_payloads`
- `durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint`
- `recovery_boundary_factory_selects_summary_for_summary_variant`
- `recovery_boundary_factory_selects_frame_for_frame_seed_variant`
- `recovery_boundary_factory_frame_seed_round_trips_summary`

### vb_qi37_1_1_red_recovery_contract_test.rs (workspace_tests)
- `event_only_recovery_returns_secret_i64_when_durable_taint_is_secret`
- `event_only_recovery_returns_derived_bool_when_durable_taint_is_derived`
- `action_completion_records_exact_secret_taint_when_action_writes_output`
- `ask_answer_records_exact_clean_taint_when_answer_writes_output`
- `runtime_to_storage_mapping_preserves_taint_for_slot_write`
- `event_only_recovery_keeps_slot_taint_supported_when_value_bytes_are_valid`
- `deterministic_step_recovery_hydrates_exact_tainted_frame_when_slot_event_is_complete`
- `recovery_does_not_default_missing_durable_taint_to_clean`
- `no_output_step_does_not_fabricate_slot_zero_dimension`
- `no_output_step_summary_reports_zero_slots_written`
- `no_output_step_recovery_has_no_recovered_slot_entries`
- `corrupt_slot_value_blocks_both_values_and_taint`
- `missing_slot_value_blocks_both_values_and_taint`
- `supported_seed_hydrates_exact_secret_taint`
- `supported_seed_hydrates_exact_derived_taint`
- `drain_report_contract_requires_three_drained_and_three_written`
- proptest: `proptest_event_only_slot_recovery_preserves_secret_taint`, `proptest_no_output_success_never_creates_slot_zero`, `proptest_valid_slot_events_are_fully_hydrateable`

## Key Observations

1. **No YAML in recovery paths**: The recovery system uses only `postcard` binary codec (not YAML). The `durability_matrix.rs` mentions YAML primitives but the recovery layer itself uses `JournalEvent` enums with Postcard encoding. Confirmed: NO yaml parsing occurs in `vb_storage/src/recovery/` (grep returned no matches).

2. **Typed divergence errors**: `RecoveryError::ReplayDivergence` carries `step: StepIdx` and `detail: String`. Additional typed errors exist for digest mismatches.

3. **Snapshot+tail hydration**: `hydrate_run_frame` in `hydrate.rs` enforces run_id match, seq ordering, and zero-step-count guard.

4. **Digest verification**: `verify_digests` in `recover.rs` checks workflow source digest and compiled IR digest at configurable levels (`DigestCheck` enum).

5. **Action blocking**: `ActionReplayTracker` prevents re-execution of non-idempotent actions during recovery via `NonIdempotentActionBlocked` error.

6. **Unsupported state tracking**: `UnsupportedRecoveryState` tracks 4 categories: `slot_values`, `slot_taint`, `action_payloads`, `pending_actions`. `DurableFrameRecoveryBoundary::hydrate_run_frame` fails closed if any are true.

7. **Object/list slots**: `recovered_object_slots_are_explicitly_unsupported` and `recovered_list_slots_are_explicitly_unsupported` tests confirm Object/List SlotValues are not recoverable via events (require typed replay from CompiledWorkflow).

## Risk Tags

- `temporal` — recovery ordering invariants (seq, attempt filtering)
- `concurrency` — Fjall multi-writer isolation
- `persistence` — crash recovery, corrupt snapshot, partial writes
- `parser/codec` — Postcard decode failures mapping to typed errors
- `performance` — full journal replay for large runs
- `public_API` — `RuntimeRecoveryBoundary`, `hydrate_run_frame`, `verify_digests`

## Verified File List

All paths below verified to exist in the isolated workspace:

- `crates/vb_storage/src/recovery/types.rs`
- `crates/vb_storage/src/recovery/recover.rs`
- `crates/vb_storage/src/recovery/replay/core.rs`
- `crates/vb_storage/src/recovery/replay/summary.rs`
- `crates/vb_storage/src/recovery/replay/mod.rs`
- `crates/vb_storage/src/recovery/hydrate.rs`
- `crates/vb_storage/src/recovery/hydrate_support.rs`
- `crates/vb_storage/src/recovery/mod.rs`
- `crates/vb_runtime/src/recovery.rs`
- `crates/vb_storage/tests/recovery_integration.rs`
- `crates/vb_storage/tests/replay_resume.rs`
- `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs`

## Downstream Owners

- `rust-contract` → requirements, invariants, contract clauses
- `proof-planner` → verifier lane selection (Kani, Miri, proptest)
- `test-planner` → test strategy from contracts
- `holzman-rust` → implementation of any recovery fixes
- `formal-verifier` → proof execution
