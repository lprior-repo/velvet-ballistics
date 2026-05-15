# Proof Plan Review Input — vb-core-replay-divergence-recovery

## For Reviewer: contract-verification-reviewer

---

## Plan Summary

14 proof obligations covering typed replay divergence detection and no-YAML recovery hydration. Lane distribution: **13 miri + 1 proptest**. TLA+, Verus, Kani, Loom, Flux fully waived.

---

## Key Decisions for Review

### 1. TLA+ Waiver — Correct?

**Claim**: Sequential single-writer deterministic replay. No concurrent workers. No temporal liveness properties. No distributed consensus.

**Evidence**: delivery-scope.jsonl shows no `spawn`, `tokio`, `Mutex`, `RwLock`, `Atomic` in recovery paths. replay_events is serial per run_id.

**Request**: Confirm TLA+ waiver is sound or demand a PlusCal model.

### 2. Verus Waiver — Correct?

**Claim**: No algebraic theorem kernel. All critical invariants provable via miri on existing integration tests (type exhaustiveness + UB coverage).

**Evidence**: RecoveryError enum variants are proven exhaustive by miri (not Verus). No `proof fn` or `spec fn` in recovery surface.

**Request**: Confirm Verus waiver is sound or demand Verus specs for key invariants.

### 3. Kani Waiver — Correct?

**Claim**: No `unsafe` code in vb_storage/src/recovery/ or vb_runtime/src/recovery/. Miri covers all test-binary UB including Postcard decoding.

**Evidence**: grep confirms zero `unsafe` in recovery module paths.

**Request**: Confirm Kani waiver is sound or demand kani harnesses for Postcard decode paths.

### 4. CC-004 (ReplayDivergence) — Is miri Sufficient?

**Claim**: Typed ReplayDivergence { step, detail } produced on semantic divergence. miri on integration tests exercises replay_paths with divergence injected.

**Risk**: temporal — divergence is a runtime state transition property.

**Request**: Review whether miri on integration tests is sufficient for typed replay divergence, or if Kani/Verus is needed for formal coverage.

### 5. CC-005 (Fail-Closed) — Is UnsupportedRecoveryState Sufficient?

**Claim**: 4 categories tracked; DurableFrameRecoveryBoundary::hydrate_run_frame fails if any true. miri exercises reject_unsupported_live_frame_state with all 4 category combinations.

**Request**: Review whether miri integration tests cover all 4 UnsupportedRecoveryState categories or if bounded model checking is needed.

---

## Obligation Status Snapshot

| ID | Clause | Risk | Verifier | Required | Status |
|----|--------|------|----------|----------|--------|
| MIRI-CC001-001 | CC-001 | parser_codec | miri | yes | planned |
| MIRI-CC002-001 | CC-002 | persistence,temporal | miri | yes | planned |
| MIRI-CC003-001 | CC-003 | persistence,temporal | miri | yes | planned |
| MIRI-CC004-001 | CC-004 | temporal | miri | yes | planned |
| MIRI-CC005-001 | CC-005 | persistence | miri | yes | planned |
| MIRI-CC005-002 | CC-005 | persistence | miri | yes | planned |
| MIRI-CC006-001 | CC-006 | persistence | miri | yes | planned |
| MIRI-CC007-001 | CC-007 | persistence,temporal | miri | yes | planned |
| PROPTEST-CC007-001 | CC-007 | persistence | proptest | yes | planned |
| MIRI-CC008-001 | CC-008 | persistence | miri | yes | planned |
| MIRI-INV001-001 | INV-001 | temporal | miri | yes | planned |
| MIRI-INV002-001 | INV-002 | temporal | miri | yes | planned |
| MIRI-INV003-001 | INV-003 | persistence | miri | yes | planned |
| MIRI-INV004-001 | INV-004 | persistence | miri | yes | planned |

**Total: 14 active obligations. 0 deferred. 0 waived-without-reason.**

---

## Waivers Requesting Confirmation

| Lane | Count | Reason |
|------|-------|--------|
| TLA+ | 1 | Single-writer sequential replay; no temporal liveness requirements |
| Verus | 1 | No algebraic kernel; miri exhaustiveness sufficient |
| Kani | 1 | No unsafe in recovery paths |
| Loom | 1 | No concurrent recovery workers |
| Flux | 1 | No refinement types in recovery API |
| Fuzz | 1 | Postcard codec fuzzed upstream |

---

## Contract Clause Traceability

| Clause | Obligations | Tests | Waiver? |
|--------|-------------|-------|---------|
| CC-001 | MIRI-CC001-001 | grep_yaml_free, full_round_trip_recovery_reads_all_events_in_order | No |
| CC-002 | MIRI-CC002-001 | full_round_trip_recovery_reconstructs_summary, etc. | No |
| CC-003 | MIRI-CC003-001 | action_replay_tracker_reconstructs_from_events, etc. | No |
| CC-004 | MIRI-CC004-001 | action_replay_blocks_duplicate_scheduled_action, etc. | No |
| CC-005 | MIRI-CC005-001, MIRI-CC005-002 | corrupt_slot_value_blocks_both_values_and_taint, etc. | No |
| CC-006 | MIRI-CC006-001 | recovered_object_slots_are_explicitly_unsupported, etc. | No |
| CC-007 | MIRI-CC007-001, PROPTEST-CC007-001 | event_only_recovery_returns_secret_i64..., etc. | No |
| CC-008 | MIRI-CC008-001 | recovery_boundary_factory_frame_seed_round_trips_summary | No |
| INV-001 | MIRI-INV001-001 | resume_tail_replay_rejects_sequence_gap_before_resume_continuation | No |
| INV-002 | MIRI-INV002-001 | action_replay_blocks_duplicate_scheduled_action | No |
| INV-003 | MIRI-INV003-001 | recovery_boundary_factory_frame_seed_round_trips_summary | No |
| INV-004 | MIRI-INV004-001 | durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint | No |
| INV-005 | (covered by MIRI-CC001-001) | grep_yaml_free, etc. | No |

**All 13 contract clauses covered. Zero orphaned obligations.**
