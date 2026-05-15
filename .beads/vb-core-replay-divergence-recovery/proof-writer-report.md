# Proof-Writer Report — vb-core-replay-divergence-recovery

## Bead
- id: `vb-core-replay-divergence-recovery`
- workspace: `/tmp/vb-ws/vb-core-replay-divergence-recovery`
- state: 5 (Proof Writing)
- attempt: 1

---

## Scope

14 obligations (13 miri + 1 proptest) covering recovery subsystem for vb_storage/vb_runtime:
- Typed replay with `ReplayDivergence` and `NonIdempotentActionBlocked`
- Postcard-only codec (zero YAML in recovery paths)
- Snapshot+tail hydration fidelity
- Events-only hydration
- `DurableFrameRecoveryBoundary` fail-closed boundary

---

## Artifact Inventory

| Obligation | Artifact | Test(s) Mapped |
|---|---|---|
| MIRI-CC001-001 | `crates/vb_storage/src/recovery/` | grep static scan + miri test |
| MIRI-CC002-001 | `crates/vb_storage/src/recovery/hydrate.rs::hydrate_run_frame` | `full_round_trip_recovery_reconstructs_summary`, `full_round_trip_recovery_detects_slot_writes`, `deterministic_step_recovery_hydrates_exact_tainted_frame_when_slot_event_is_complete` |
| MIRI-CC003-001 | `crates/vb_storage/src/recovery/recover.rs::verify_digests` | `digest_mismatch_detection_tests` in recovery_integration |
| MIRI-CC004-001 | `crates/vb_storage/src/recovery/replay/core.rs::replay_events` | `action_replay_tracker_reconstructs_from_events`, `action_replay_tracker_tracks_failed_actions`, `action_replay_blocks_duplicate_scheduled_action` |
| MIRI-CC005-001 | `crates/vb_storage/src/recovery/replay/core.rs::load_snapshot` | `corrupt_slot_value_blocks_both_values_and_taint`, `missing_slot_value_blocks_both_values_and_taint` |
| MIRI-CC005-002 | `crates/vb_runtime/src/recovery.rs::reject_unsupported_live_frame_state` | `durable_frame_recovery_boundary_rejects_unsupported_action_payloads`, `durable_frame_recovery_boundary_rejects_inconsistent_seed` |
| MIRI-CC006-001 | `crates/vb_storage/src/recovery/replay/core.rs::recover_snapshot_plus_tail` | `recovered_object_slots_are_explicitly_unsupported`, `recovered_list_slots_are_explicitly_unsupported` |
| MIRI-CC007-001 | `crates/vb_storage/src/recovery/hydrate.rs::hydrate_run_frame_from_events` | `event_only_recovery_returns_secret_i64_when_durable_taint_is_secret`, `event_only_recovery_keeps_slot_taint_supported_when_value_bytes_are_valid` |
| MIRI-CC008-001 | `crates/vb_runtime/src/recovery.rs::DurableFrameRecoveryBoundary` | `recovery_boundary_factory_frame_seed_round_trips_summary` |
| MIRI-INV001-001 | `crates/vb_storage/src/recovery/replay/core.rs` | `resume_tail_replay_rejects_sequence_gap_before_resume_continuation` |
| MIRI-INV002-001 | `crates/vb_storage/src/recovery/replay/core.rs::ActionReplayTracker` | `action_replay_blocks_duplicate_scheduled_action` |
| MIRI-INV003-001 | `crates/vb_runtime/src/recovery.rs` | `recovery_boundary_factory_frame_seed_round_trips_summary`, `supported_seed_hydrates_exact_secret_taint`, `supported_seed_hydrates_exact_derived_taint` |
| MIRI-INV004-001 | `crates/vb_runtime/src/recovery.rs::DurableFrameRecoveryBoundary::hydrate_run_frame` | `durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint`, `durable_frame_recovery_boundary_rejects_unsupported_action_payloads` |
| PROPTEST-CC007-001 | `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs` | `proptest_event_only_slot_recovery_preserves_secret_taint`, `proptest_valid_slot_events_are_fully_hydrateable`, `proptest_no_output_success_never_creates_slot_zero` |

---

## Verification Commands

```bash
# CC-001: YAML exclusion static scan
rg -i 'yaml|serde_yaml|quick_yaml' crates/vb_storage/src/recovery/ --files-with-matches

# Miri — vb_storage recovery_integration (CC002-CC007, INV002)
cargo miri test --package vb_storage --test recovery_integration -- --nocapture 2>&1 | tail -30

# Miri — vb_storage replay_resume (INV001)
cargo miri test --package vb_storage --test replay_resume -- --nocapture 2>&1 | tail -20

# Miri — vb_runtime unit tests (CC005-CC008, INV003-INV004)
cargo miri test --package vb_runtime -- --nocapture 2>&1 | tail -20

# Proptest — CC007 slot hydration properties
cargo test --package workspace_tests --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture 2>&1 | tail -40
```

---

## Proptest Property Detail (PROPTEST-CC007-001)

Three properties defined in `vb_qi37_1_1_red_recovery_contract_test.rs`:

1. **`proptest_event_only_slot_recovery_preserves_secret_taint`**: For any valid `SlotValue` with secret taint, events-only hydration preserves taint classification through `recover_runtime_frame_seed_from_events`.

2. **`proptest_valid_slot_events_are_fully_hydrateable`**: For any well-formed sequence of slot events (bounded depth ≤16, slots ≤32), `recover_runtime_frame_seed_from_events` succeeds without error.

3. **`proptest_no_output_success_never_creates_slot_zero`**: A run that calls `StepSucceeded { output: SlotIdx::ZERO }` never results in a recovered slot at index zero.

All three properties use `proptest::proptest!` with bounded strategy inputs. No external I/O in property runners.

---

## No New Production Code Written

This skill writes NO production code. All test coverage exists in the pre-existing test binaries:
- `crates/vb_storage/tests/recovery_integration.rs`
- `crates/vb_storage/tests/replay_resume.rs`
- `crates/vb_storage/src/recovery/tests.rs`
- `crates/vb_runtime/src/recovery.rs` (unit tests)
- `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs`

The proof-writer confirms these artifacts provide complete test coverage for all 14 obligations.

---

## Next Action

Pass to formal-verifier skill to execute miri runs and proptest, record PASS/FAIL_LOCAL per obligation, and produce formal-verifier-report.md.

---

**Proof-writer report generated:** 2025-01-01 (workspace clock not available)
