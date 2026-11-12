# Code Review: vb-0xm95 (Live Policy Evaluation and Persistence)

**Candidate:** b752db4c1a665caf90fa9e63e43d1505b0ba651d
**Base:** dba19b95918aca7c23e91b8814afa2a9118449f8
**Reviewer:** RTX3090 code review agent
**Date:** 2026-10-22

## Verdict: APPROVED

All 24 production files reviewed. No open actual behavior findings.

## Scope

Exact actual diff +35 lines context per hunk across all 24 changed production paths, plus full existing admission validator (`recovery/full/admission.rs`) and checkpoint reference (`run_checkpoint.rs`). No prior approval inherited; this is a fresh scoped CODE review.

## Changed Files (24)

| Path | Change Summary |
|------|----------------|
| `crates/vb_ipc/src/frame_types.rs` | Style: `bytes.as_mut_slice()` vs `&mut bytes[..]` |
| `crates/vb_runtime/src/journal.rs` | Added `strict_policy_port_tests` module include |
| `crates/vb_runtime/src/journal/chunk_001.rs` | Added `append_policy_decision_strict` trait method (fail-closed default) |
| `crates/vb_runtime/src/journal/chunk_002.rs` | Implemented `append_policy_decision_strict` for `StorageRuntimeJournal` |
| `crates/vb_runtime/src/journal/chunk_003.rs` | Implemented `append_policy_decision_strict` for `QueuedStorageRuntimeJournal` |
| `crates/vb_runtime/src/recovery.rs` | Removed `validate_recovered_policy_pin` from exports |
| `crates/vb_runtime/src/recovery/full.rs` | Removed `validate_recovered_policy_pin` export |
| `crates/vb_runtime/src/recovery/full/product.rs` | Added `admission` field to recovery product/parts; builder pattern |
| `crates/vb_runtime/src/runtime.rs` | Added `live_policy` field; `configure_live_policy`/`activate_emergency_deny` methods |
| `crates/vb_runtime/src/runtime/live_policy_dispatch.rs` | **New**: Policy evaluation dispatch, admission priority, persistence |
| `crates/vb_runtime/src/runtime/registered_executor.rs` | Policy evaluation before execution; `PolicyDenied` error variant |
| `crates/vb_runtime/src/shard/impl_.rs` | Added `policy_decision.rs` include |
| `crates/vb_runtime/src/shard/impl_parts/policy_decision.rs` | **New**: Shard policy decision port with sequence ownership |
| `crates/vb_runtime/src/shard/lifecycle.rs` | Added recovery admission module includes; strict policy sequence tests |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | Refactored `handle_recover` to phased validation approach |
| `crates/vb_runtime/src/shard/lifecycle/chunk_004.rs` | Made `require_pending_action_ownership` pub(super) |
| `crates/vb_runtime/src/shard/lifecycle/recovered_admission.rs` | **New**: 3-phase recovery admission validation |
| `crates/vb_runtime/src/shard/lifecycle/registered_completion.rs` | Extracted context construction into focused helpers |
| `crates/vb_runtime/src/shard/lifecycle/registered_completion_context.rs` | **New**: Artifact load, policy pin conversion, context assembly |
| `crates/vb_runtime/src/shard/lifecycle/registered_context_helpers.rs` | **New**: Input validation, encoding, output build helpers |
| `crates/vb_runtime/src/shard/types.rs` | Added `RecoveredAdmissionKind`; admission on `RecoverRunCommand` |
| `fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs` | Doc comments with fuzz command |
| `fuzz/fuzz_targets/yaml_events.rs` | Doc comments with fuzz command |
| `fuzz/src/yaml_target.rs` | `MAX_FUZZ_PAYLOAD_USIZE` 4096 → 100,000; regression test |

## Contract Verification

### 1. Existing Contracts (master25)
All changed code reuses existing types: `ActionTicket`, `RunFrame`, `RunAdmission`, `DecisionRecord`, `PinnedAdmissionPolicy`, `PolicyLayer`, `DecisionCode`, `EmergencyDenyState`, `RecoveredAdmissionContext`, `RecoveredAdmissionEvidence`. No invented abstractions.

### 2. Helper Function Size
New helper functions (`validate_action_input`, `read_slot_i64`, `resolve_action_input`, `encode_action_input`, `build_action_output`, `load_admitted_artifact`, `convert_policy_pin`, `build_registered_action_context`) are all under 25 logical lines. `evaluate_admission_and_persist` is 14 logical lines. `dispatch_policy_decision` is 18 logical lines. All compliant.

### 3. Strict Append Durability
`append_policy_decision_strict` uses `journal.append_strict()` on the storage journal, providing a durability barrier for policy decisions. The queued variant drains all pending writes before the strict append, ensuring no lost decision on crash. Correct.

### 4. Queue Prefix / Sequence Ownership
`Shard::append_policy_decision_strict` validates `require_pending_action_ownership(decision.ticket)` before any append. Sequence is computed with `checked_next_seq` (overflow-safe) and updated only after successful journal append. Capacity is reserved before append. Fail-closed on any step. Correct.

### 5. Typed Errors
`RegisteredExecutionError::PolicyDenied(Box<DecisionRecord>)` provides a typed policy rejection error carrying the full decision record. `RunCheckpointV1Error` provides 16 distinct typed errors. `RuntimeError` variants used throughout. Correct.

### 6. Fail-Closed Before Effect
In `execute_registered_action`, policy evaluation and persistence (`evaluate_admission_and_persist`) occurs BEFORE the adapter executes (`adapter.execute`). In `reconcile_registered_action`, no policy evaluation occurs (recovery-only path). Correct per contract.

### 7. Identity Checks
Identity checks are performed in all recovery paths:
- Run identity: `admission.run_id() != run` (Phase A)
- Artifact digest: `admission_digest != expected_artifact_digest` (Phase A)
- Workflow digest: verified in `handle_recover`
- Checkpoint validation: `validate_against` checks all identities before restore
- Ticket ownership: `require_pending_action_ownership` in all paths

Correct.

### 8. Caps / Policy / Pins in All Paths
Historical admission is restored verbatim (not rebuilt from current policy). Unverified admission is built with `RunPolicyPin::Unverified` explicitly. Policy pin validation happens in `validate_recovered_admission_evidence`. Checkpoint policy pin validated in Phase B. Correct.

### 9. Unverified Never Promotes to Pinned
`RecoveredAdmissionKind` enum explicitly distinguishes `Historical` from `Unverified`. `RecoverRunCommand::new(7 args)` defaults to `Unverified`. `for_retry` preserves checkpoint's admission or creates `Unverified`. `build_unverified_admission` forces `RunPolicyPin::Unverified`. No path promotes Unverified to Pinned. Correct.

### 10. Reconcile-Only Receipt (No NEW Live Effects)
`reconcile_registered_action` calls `registered_context` (which builds the context) but does NOT call `evaluate_admission_and_persist`. The adapter reconciles (reads existing receipt) rather than executing. Correct per contract.

### 11. Public 7-Args and for_retry Bound Helpers
`RecoverRunCommand::new(7 args)` is preserved as public. `RecoverRunCommand::for_retry(checkpoint, next_seq)` is preserved as public. Both construct valid recovery commands. Correct.

### 12. Fuzz Bound Alignment
`MAX_FUZZ_PAYLOAD_USIZE` changed from 4096 to 100,000, matching the actual prepush bound. Regression test `large_yaml_exceeds_4096_bound` would fail with the old bound (RED) and passes with the new bound (GREEN). The test validates >4096 events and ≤100,000 events (actual bound). Correct.

## Risk Assessment

### Low Risk
- IPC style change is purely cosmetic
- Journal trait default methods are fail-closed
- Recovery validation is additive (more checks, fewer surprises)
- Fuzz bound increase aligns with reality

### Medium Risk
- Live policy infrastructure is new attack surface (but well-structured)
- Emergency deny state has global effect (by design)
- Policy decision persistence adds I/O path (strict durability required)

### Residual
- `evaluate_admission_and_persist` is called in `execute_registered_action` but not in `reconcile_registered_action` - intentional per contract (recovery reads receipts, doesn't execute)
- Emergency overlay activation in `dispatch_policy_decision` mirrors durable state but is not persisted back (only read) - correct behavior

## What Was NOT Covered

- Test suite (separate review)
- Documentation/changelog updates (not in scope)
- Benchmark results (not in scope)
- Formal proof artifacts (still open per master)
- CI status (V10 all 14, V11 1989 lib, V12 140 focused, V35 3 fuzz - all reported as passing)

## Conclusion

The implementation correctly separates recovery admission validation into distinct phases, preserves the historical vs unverified admission distinction, evaluates and persists policy decisions before execution effects, and aligns the fuzz bound with the actual prepush bound. All 12 rubric criteria are satisfied. No open actual behavior findings.

**APPROVED** for canonical/main integration.
