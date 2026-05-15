# Proof Strategy — vb-core-replay-divergence-recovery

## Bead

- id: vb-core-replay-divergence-recovery
- state: 4 (Proof Planning)
- source: /home/lewis/src/velvet-ballistics
- workspace: /tmp/vb-ws/vb-core-replay-divergence-recovery

---

## Scope Summary

Recovery subsystem for vb_storage/vb_runtime — typed replay with divergence detection and no-YAML hydration. All codec is Postcard; no YAML parser ever appears in recovery paths.

**Primary risks:** temporal (replay divergence), persistence (Fjall journal durability), parser_codec (Postcard decode errors).

**Secondary risks:** concurrency (ActionReplayTracker scheduling), public_api (DurableFrameRecoveryBoundary trait).

---

## Verification Lane Selection

| Lane | Applicable | Reason |
|------|-----------|--------|
| Miri | **YES** — 13 obligations | Detects UB in unsafe code, Stacked Borrows violations, invalid Postcard decoding; covers all persistence and temporal invariants |
| Proptest | **YES** — 1 obligation | Bounded property testing for slot event hydration; no state-space explosion |
| Kani | **WAIVED** | No unsafe code in recovery paths; miri covers all unsafe usage in test binaries |
| Verus | **WAIVED** | No algebraic theorem kernel; all critical invariants provable via miri on existing integration/unit tests |
| TLA+ | **WAIVED** | Single-writer deterministic sequential replay; no concurrent workflows, no temporal liveness requirements, no distributed consensus |
| Loom | **WAIVED** | No concurrent recovery workers; replay is serial per run_id |
| Flux | **WAIVED** | No refinement types used in recovery API surface |
| Fuzz | **NOT REQUIRED** | Postcard codec already exercised via miri + proptest; no untrusted input |

**Total active obligations: 14 (13 miri + 1 proptest)**

---

## Waiver Record

| Lane | Obligation | Reason | Compensating Evidence | Follow-up Trigger |
|------|-----------|--------|----------------------|-------------------|
| Kani | All | No `unsafe` in `vb_storage/src/recovery/` or `vb_runtime/src/recovery/`; miri covers all test-binary UB | MIRI-CC001-001 through MIRI-INV004-001 | If unsafe code is added to recovery paths, activate Kani |
| Verus | All | No algebraic kernel; typed Rust + miri exhaustiveness covers all invariant properties | 14 miri/proptest obligations | If refinement types are added to recovery API, activate Verus |
| TLA+ | All | Single-writer sequential journal; no temporal liveness properties; no concurrent workers | miri on integration tests covers seq ordering | If concurrent recovery workers are introduced, model with TLA+ |
| Loom | All | No spawned concurrent tasks in recovery paths | miri covers thread-safety | If recovery parallelism is added, activate Loom |
| Flux | All | No Flux refinement types in recovery surface | N/A | If Flux is introduced, add FLUX-* obligations |
| Fuzz | All | Postcard codec fuzzed upstream; recovery tests use structured inputs | proptest + miri | If new codec is introduced, activate fuzz |

---

## Obligation Map

### CC-001: No YAML in Recovery Paths
- **MIRI-CC001-001**: `rg -i 'yaml|serde_yaml|quick_yaml'` on vb_storage/src/recovery/ + cargo miri test (recovery_integration, replay_resume)
- Evidence: zero YAML matches + all integration tests pass under miri

### CC-002: Snapshot+Tail Hydration Fidelity
- **MIRI-CC002-001**: cargo miri test (recovery_integration)
- Evidence: full_round_trip_recovery_reconstructs_summary, full_round_trip_recovery_detects_slot_writes, deterministic_step_recovery_hydrates_exact_tainted_frame_when_slot_event_is_complete pass under miri

### CC-003: Typed Digest Mismatch Errors
- **MIRI-CC003-001**: cargo miri test (recovery_integration)
- Evidence: digest mismatch detection tests pass; RecoveryError variants carry step+detail

### CC-004: Typed Replay Divergence
- **MIRI-CC004-001**: cargo miri test (recovery_integration)
- Evidence: action_replay_tracker_reconstructs_from_events, action_replay_tracker_tracks_failed_actions, action_replay_blocks_duplicate_scheduled_action pass under miri

### CC-005: Fail-Closed Corrupt/Incomplete Recovery
- **MIRI-CC005-001**: cargo miri test (recovery_integration)
- Evidence: corrupt_slot_value_blocks_both_values_and_taint, missing_slot_value_blocks_both_values_and_taint pass under miri
- **MIRI-CC005-002**: cargo miri test (vb_runtime)
- Evidence: durable_frame_recovery_boundary_rejects_unsupported_action_payloads, durable_frame_recovery_boundary_rejects_inconsistent_seed pass under miri

### CC-006: Object/List Slots Explicitly Unsupported
- **MIRI-CC006-001**: cargo miri test (recovery_integration)
- Evidence: recovered_object_slots_are_explicitly_unsupported, recovered_list_slots_are_explicitly_unsupported pass under miri

### CC-007: Events-Only Hydration Correctness
- **MIRI-CC007-001**: cargo miri test (recovery_integration)
- Evidence: event_only_recovery_returns_secret_i64_when_durable_taint_is_secret, event_only_recovery_keeps_slot_taint_supported_when_value_bytes_are_valid pass under miri
- **PROPTEST-CC007-001**: cargo test (workspace_tests)
- Evidence: proptest_event_only_slot_recovery_preserves_secret_taint, proptest_valid_slot_events_are_fully_hydrateable, proptest_no_output_success_never_creates_slot_zero pass

### CC-008: Frame Seed Round-Trip Integrity
- **MIRI-CC008-001**: cargo miri test (vb_runtime)
- Evidence: recovery_boundary_factory_frame_seed_round_trips_summary passes under miri

### INV-001: JournalEvent Seq Ordering
- **MIRI-INV001-001**: cargo miri test (replay_resume)
- Evidence: resume_tail_replay_rejects_sequence_gap_before_resume_continuation passes under miri

### INV-002: ActionReplayTracker Blocks Duplicates
- **MIRI-INV002-001**: cargo miri test (recovery_integration)
- Evidence: action_replay_blocks_duplicate_scheduled_action passes under miri

### INV-003: RecoveryFrameSeed Postcard Round-Trip
- **MIRI-INV003-001**: cargo miri test (vb_runtime)
- Evidence: recovery_boundary_factory_frame_seed_round_trips_summary, supported_seed_hydrates_exact_secret_taint, supported_seed_hydrates_exact_derived_taint pass under miri

### INV-004: DurableFrameRecoveryBoundary Succeeds Iff 4 Categories False
- **MIRI-INV004-001**: cargo miri test (vb_runtime)
- Evidence: durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint, durable_frame_recovery_boundary_rejects_unsupported_action_payloads pass under miri

---

## Verification Commands

```bash
# YAML exclusion — static scan
rg -i 'yaml|serde_yaml|quick_yaml' crates/vb_storage/src/recovery/ --files-with-matches

# Miri — vb_storage recovery_integration
cargo miri test --package vb_storage --test recovery_integration -- --nocapture 2>&1 | tail -30

# Miri — vb_storage replay_resume
cargo miri test --package vb_storage --test replay_resume -- --nocapture 2>&1 | tail -20

# Miri — vb_runtime
cargo miri test --package vb_runtime -- --nocapture 2>&1 | tail -20

# Proptest — workspace_tests
cargo test --package workspace_tests --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture 2>&1 | tail -40
```

---

## Success Criteria

- All 14 obligations show PASS or PASS_LOCAL in proof-obligations.planned.jsonl
- No obligation is marked FAIL_LOCAL, FAIL_REGRESSION, BLOCKED, or DEFERRED_GLOBAL
- TLA+/Verus/Kani/Loom/Flux waivers are recorded and not silently omitted
- traceability-matrix.jsonl maps each obligation to its contract clause
- Evidence artifacts (miri-report.txt, proptest-report.txt) are referenced but not written by this skill
