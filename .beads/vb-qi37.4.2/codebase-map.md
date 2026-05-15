# vb-qi37.4.2 Codebase Map

## Bead
**Title**: runtime: Enforce admission gate before run creation
**State**: 2 (Explore and Scope)

## Executive Summary

The shard's `handle_submit_with_inputs_contracts_and_header_mode` (lifecycle/chunk_001.rs:68-136) already calls `build_admission` before frame allocation and run state insertion. However, the `RunSubmitted` journal event is written after `build_admission` returns (lines 91-100), meaning the sequencing is already correct: admission gate is evaluated before `RunSubmitted` is journaled. The bead likely aims to **verify** this sequencing is correct, add integration tests that confirm rejection behavior, or refine the error-to-journal ordering.

---

## Key Files and Symbols

### vb_runtime admission layer
| File | Key Symbols | Notes |
|------|-------------|-------|
| `crates/vb_runtime/src/admission.rs` | `RunAdmission`, `AdmissionError`, `admit_run`, `admit_artifact_run`, `admit_run_with_budget`, `check_capability`, `REQUIRED_GATE_COUNT = 15`, `ArtifactEnvelopeError`, `ArtifactStore`, `AcceptedArtifactStore`, `StorageArtifactStore`, `AlwaysPresentArtifactStore` | Full admission gate logic. `admit_artifact_run` is the main function used by shard. `REQUIRED_GATE_COUNT = 15` — storage layer uses 2, known mismatch from vb-qi37.6. |
| `crates/vb_runtime/src/runtime.rs` | `Runtime::submit_direct`, `Runtime::submit_direct_with_grants`, `Runtime::submit_direct_with_grants_and_contracts` | Top-level submit APIs on multi-shard runtime. All delegate to `shard.enqueue(ShardCommand::Submit/SubmitWithInputs/SubmitWithContracts)`. No direct admission enforcement here. |
| `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` | `Shard::new_with_journal_and_artifact_store` (line 8), `Shard::enqueue` (line 44), `Shard::tick` (line 139) | Shard constructor takes `artifact_store: SharedAcceptedArtifactStore`. `enqueue` probes journal health before accepting submit variants. `tick` dispatches to `handle_submit*`. |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `handle_submit` (line 2), `handle_submit_with_inputs_contracts_and_header_mode` (line 68), `build_admission` (line 138) | **Critical path**: `build_admission` called at line 86 BEFORE frame allocation (line 87) and BEFORE journal events (lines 91-111). RunState inserted at line 125 AFTER admission. Admission gate is already enforced before run creation. |
| `crates/vb_runtime/src/shard/types.rs` | `ShardCommand::Submit`, `SubmitPrePersisted`, `SubmitWithInputs`, `SubmitWithContracts` | Command variants enqueued by Runtime. `Submit` and `SubmitPrePersisted` differ only in `persist_header` flag. |
| `crates/vb_runtime/src/error/mod.rs` | `RuntimeError::AdmissionArtifactNotFound`, `AdmissionCapabilityDenied`, `AdmissionArtifactInvalid` | Errors returned when `build_admission` fails. |

### vb_storage admission layer
| File | Key Symbols | Notes |
|------|-------------|-------|
| `crates/vb_storage/src/admission.rs` | `AcceptedArtifact`, `VerificationProof`, `submit_artifact`, `ADMISSION_GATE_COUNT = 2` | Storage-level admission. `submit_artifact` writes `gate_count=2` for Journaled/Strict. **Contract mismatch**: runtime requires 15. |
| `crates/vb_storage/src/journal/admission.rs` | `verify_content_digest` | Content digest verification at admission time. |

---

## Admission Gate Sequencing (current)

In `handle_submit_with_inputs_contracts_and_header_mode`:

```
Line 77: if self.runs.contains_key(&run) → RunAlreadyExists
Line 80: if self.runs.len() >= self.max_active_runs → ActiveRunCapacityExceeded
Line 86: build_admission(run, digest, caps)?  ← ADMISSION GATE (called first)
Line 87: take_frame_for(run, &workflow)?       ← Frame allocated after admission
Line 89: trace_ring.push(RunSubmitted)
Line 91-100: journal RunSubmitted
Line 102-111: journal RunAdmission (if Some)
Line 113: counters.inc_submitted()
Line 116-124: build RunState
Line 125: self.runs.insert(run, state)         ← Run created after admission
Line 127: drive_run(run)
```

**Finding**: Admission gate (line 86) is already evaluated BEFORE run creation (line 125), frame allocation (line 87), and journal events (lines 91-111). The sequencing is correct.

---

## Known Contract Issue (Pre-existing from vb-qi37.6)

`vb_runtime::admission::REQUIRED_GATE_COUNT = 15` but `vb_storage::admission::submit_artifact` writes `gate_count=2` for Journaled/Strict. This means artifacts accepted by storage will fail runtime admission gate count check. This is tracked in vb-qi37.6 as `BLOCKER_GATE_COUNT_ALIGNMENT`.

---

## Existing Test Coverage

| Test | Location | Coverage |
|------|----------|----------|
| `admission_rejection_does_not_insert_run_state` | `lifecycle_tests/chunk_003.rs:53` | **Insufficient**: Uses `Shard::new(small_config())` (Relaxed policy). Admission always succeeds. Test asserts `active_run_count() == 1`, confirming run IS inserted. Does NOT test rejection. |
| `admit_artifact_run_rejects_*` | `admission.rs:704-784` | Unit tests for `admit_artifact_run` function directly. Use `FixedAcceptedStore`. |
| `admission_admit_run_strict_without_artifact_rejected` | `admission.rs:882-896` | Uses `NeverPresentStore`. Tests `admit_run` (not `admit_artifact_run`). |

**Gap**: No lifecycle-level integration test that submits with Strict/Journaled policy and a missing/invalid artifact and verifies run is NOT inserted.

---

## Risk Tags

- `persistence`: Journal event ordering relative to admission — current code looks correct but needs verification
- `concurrency`: Single-shard run (no cross-shard state), but multi-shard runtime routes to correct shard before admission
- `public_api`: Runtime submit APIs are public; admission enforcement affects external callers
- `user_visible_behavior`: Admission rejection returns typed errors (`AdmissionArtifactNotFound`, `AdmissionCapabilityDenied`, `AdmissionArtifactInvalid`)

---

## Recommended Verifier Modes

1. **Miri**: For shard lifecycle tests with admission rejection paths (tighten existing test to use Strict policy + never-present store)
2. **Kani**: `kani_capability_harnesses.rs` — add harness for `build_admission` with invalid artifact under Strict policy
3. **Integration tests**: New lifecycle tests confirming run NOT inserted when admission fails under Strict/Journaled with `NeverPresentStore`

---

## Open Questions

1. Is the bead goal to ADD new enforcement (which already exists) or to VERIFY the existing enforcement via tests?
2. Does the `RunSubmitted` journal event (lines 91-100) need to move AFTER `build_admission` succeeds, or is the current ordering (admission first, then RunSubmitted) sufficient?
3. The existing test `admission_rejection_does_not_insert_run_state` at line 53 of `lifecycle_tests/chunk_003.rs` asserts `active_run_count() == 1` which means it expects the run to be inserted. Should this test be renamed or modified to actually test rejection?
