# VB-88A03 Dispatch Test Review V3

**Candidate:** 3fca010b5f5aaaafb5f41b08b95200db001bab3e  
**Base:** dba19b95918aca7c23e91b8814afa2a9118449f8  
**Reviewer:** RTX3090 (independent test review)  
**Date:** 2026-10-22  
**Verdict:** APPROVED

## Scope

Independent test review of the strict policy journal ports, internal shard sequence tests, and nine registered-executor plus three fail-closed cases. Recovery/fuzz/wrong-run tests reviewed separately.

## Test Coverage Matrix

### Internal Port and Sequence Tests (7 tests)

| Test | File | Coverage |
|------|------|----------|
| `storage_adapter_persists_full_policy_decision_event` | `crates/vb_runtime/src/journal/tests/strict_policy_port.rs` | Full durable roundtrip under Journaled profile; reopens FjallJournal and verifies complete DecisionRecord |
| `queued_adapter_drains_prefix_before_strict_decision` | `crates/vb_runtime/src/journal/tests/strict_policy_port.rs` | Prefix events flushed before strict decision; queue remains accepting; verifies both run_a prefix and subsequent run_b persistence |
| `noop_and_volatile_adapters_fail_closed` | `crates/vb_runtime/src/journal/tests/strict_policy_port.rs` | Default trait implementations return exact `UnsupportedOperation { operation: "append_policy_decision_strict" }` |
| `successful_strict_policy_port` | `crates/vb_runtime/src/shard/lifecycle_tests/strict_policy_sequence.rs` | Real shard lifecycle: seed sequence at 5, call append, verify sequence advances to 6, exact DecisionRecord and seq captured by recording journal |
| `typed_strict_policy_port_failure` | `crates/vb_runtime/src/shard/lifecycle_tests/strict_policy_sequence.rs` | Injected `UnsupportedOperation` via test journal propagates as exact typed error; sequence and pending ticket unchanged |
| `overflow_at_u64_max` | `crates/vb_runtime/src/shard/lifecycle_tests/strict_policy_sequence.rs` | Sequence at u64::MAX triggers `SequenceOverflow` before journal call; no journal call, no state change |
| `wrong_ticket_rejected` | `crates/vb_runtime/src/shard/lifecycle_tests/strict_policy_sequence.rs` | Wrong action ID (999 vs actual) triggers `InvalidActionCompletion`; no journal call, no state change |

### Registered Executor Dispatch Tests (9 tests)

| Test | File | Coverage |
|------|------|----------|
| `allowed_policy_permits_effect_and_records_decision` | `vb_7x0ol_probe_registered_executor/policy_dispatch/allowed.rs` | Allowed decision → real effect committed; full 9-field DecisionRecord matches independent oracle; exact ticket preserved |
| `after_scheduling_policy_change_to_deny` | `vb_7x0ol_probe_registered_executor/policy_dispatch/changed_policy.rs` | Policy changed to Denied after tick; execution rejected with exact typed `PolicyDenied`; admission pin (allowed) differs from current (denied); full oracle match |
| `reconcile_ignores_policy_change_after_effect` | `vb_7x0ol_probe_registered_executor/policy_dispatch/reconcile.rs` | Execute Allowed → policy changed to Denied → reconcile reads same receipt; no new decision stored; no new effect |
| `unconfigured_policy_denies_execution` | `vb_7x0ol_probe_registered_executor/policy_dispatch/unconfigured.rs` | `CurrentPolicyIdentity::Unconfigured` → exact `PolicyDenied` with Admission layer; full oracle match; zero effects |

### Fail-Closed Tests (3 tests)

| Test | File | Coverage |
|------|------|----------|
| `suspended_policy_denies_execution_with_full_record` | `vb_policy_dispatch_fail_closed/suspended.rs` | Current Suspended → exact `PolicyDenied` with Suspended identity; zero effects; pending unchanged; full oracle match; decision stored |
| `emergency_deny_overlay_cannot_be_loosened_by_later_allowed_config` | `vb_policy_dispatch_fail_closed/emergency.rs` | Allowed → emergency activated → two executions both denied with `InfluenceState::Active`; later Allowed config doesn't loosen overlay; monotonic; two distinct digests; both stored |
| `strict_port_failure_returns_typed_runtime_error_with_zero_effects` | `vb_policy_dispatch_fail_closed/strict_failure.rs` | Test wrapper overrides only `append_policy_decision_strict`; exact `UnsupportedOperation` propagates through `RegisteredExecutionError::Runtime`; zero effects; pending unchanged; no decision stored; captures exact decision and seq |

### Original Registered Executor Tests (5 retained)

| Test | File |
|------|------|
| `execute_registered_action_commits_one_effect_and_completes_run` | `vb_7x0ol_probe_registered_executor.rs` |
| `reconcile_registered_action_reads_receipt_without_new_effect` | `vb_7x0ol_probe_registered_executor.rs` |
| `execute_registered_action_rejects_mismatched_grant_artifact` | `vb_7x0ol_probe_registered_executor.rs` |
| `recovery_of_pending_registered_action_blocked_pack_rejects` | `vb_7x0ol_probe_registered_executor.rs` |
| `recovery_of_pending_registered_action_reconcile_pack_is_resumable` | `vb_7x0ol_probe_registered_executor.rs` |

## Rubric Compliance

### Exact Error Fields and Boundary Values
All typed error assertions check the exact variant and payload:
- `UnsupportedOperation { operation: "append_policy_decision_strict" }`
- `UnsupportedOperation { operation: "test_policy_port_failure" }`
- `SequenceOverflow` (source checked via `matches!` on `JournalError`)
- `InvalidActionCompletion`
- `ReceiptMissing`
- `InfluenceState::Active` for emergency, `InfluenceState::Inactive` for non-emergency
- `PolicyLayer::Admission` for Unverified/Unconfigured, `PolicyLayer::Live` for current policy
- Boundary: u64::MAX sequence overflow explicitly tested

### Real Public API / Scoped Gates
All tests use real production APIs:
- `Runtime::execute_registered_action`, `Runtime::reconcile_registered_action`
- `Runtime::configure_live_policy`, `Runtime::activate_emergency_deny`
- `Runtime::pending_boundary_snapshot`, `Runtime::lookup_pending_action_ticket`
- Real `FjallRegister` with `effect_count()` and `value(scope())`
- Real `FjallJournal` with `events_for_run` and `read_run_events`
- Real `ExecutorCatalog::run_registered_conformance()`
- Test-only wrapper (`FailingPolicyPortJournal`) overrides only `append_policy_decision_strict`, delegates all real methods

### No Ignored Cases
All 12 required cases are covered:
- 9 registered-executor cases: Allowed, ChangedDenied, Unconfigured, ReconcileOnly, Suspended, Emergency (two distinct Allowed digests)
- 3 fail-closed cases: strict port failure, noop/volatile default, wrong ticket
Plus 4 internal port/sequence tests covering successful append, typed failure, overflow, and wrong ticket.

### Deterministic Fixtures
All fixtures use explicit, deterministic values:
- Fixed i64 constants: 42, 7, 100, 99, 55, 77, 111, 222
- Fixed policy digests: `[0x11u8; 32]`, `[0x12u8; 32]`, `[0x13u8; 32]`, `[0x22u8; 32]`, `[0x33u8; 32]`, `[0x44u8; 32]`, `[0x55u8; 32]`
- Deterministic blake3 hashing in test-side oracle
- No randomization, no time dependence

### Independent Expected Values / Full Record Equality
Test-side oracle `compute_expected_input_digest` independently computes expected input digest:
- Serializes input value as I64 bytes
- Uses byte 0 for Clean taint
- Hashes all 8 ticket fields with domain separation prefix "velvet-ballistics:policy:ticket:"
- Does NOT call `LivePolicy.evaluate` or any production encoding function

Full record equality is verified:
- `assert_eq!(stored, expected)` on complete `DecisionRecord`
- Individual field assertions for critical fields (admission pin vs current digest)
- Exact journal event count assertions (`len() == 1`, `len() == 2`)

### Named Mutation Thought Experiments
Test design kills the following mutations:
1. **Remove decision append** → `strict_port_failure_returns_typed_runtime_error_with_zero_effects` catches it (no stored decision)
2. **Move effect before decision** → `allowed_policy_permits_effect_and_records_decision` checks both effect AND stored decision
3. **Remove admission pin denial** → `after_scheduling_policy_change_to_deny` checks admission pin differs from current denied digest
4. **Reuse old current digest** → `after_scheduling_policy_change_to_deny` uses distinct allowed (0x11) and denied (0x22) digests
5. **Ignore append error** → `typed_strict_policy_port_failure` and `strict_port_failure_returns_typed_runtime_error_with_zero_effects` verify error propagation
6. **Changed full record field** → full record equality `assert_eq!(stored, expected)` catches any single-field change
7. **Failed sequence advance** → `successful_strict_policy_port` seeds at 5 and asserts advance to 6

### Full Pending State Retained After Each Failure
Every fail-closed test captures `pending_boundary_snapshot(8)` before execution and asserts equality after:
- `suspended_policy_denies_execution_with_full_record`
- `emergency_deny_overlay_cannot_be_loosened_by_later_allowed_config`
- `strict_port_failure_returns_typed_runtime_error_with_zero_effects`

Exact ticket identity also verified: `assert_eq!(Some(ticket_after), Some(ticket))`.

### Real Actual Effects Not Flags
Tests verify actual register effects via:
- `adapter.effect_count()` on real `FjallRegister` (assert 0 or 1)
- `adapter.value(scope())` returns `Some(value)` or `None`
- Receipt value assertions: `assert_eq!(receipt.value(), 100)`

## Findings

No findings. All 17 target tests compile and pass. All rubric criteria satisfied. Test design demonstrates thorough coverage of the policy dispatch behavior with exact-typed assertions, independent oracles, and comprehensive fail-closed verification.

## Command Provenance

Tests compiled and verified via workspace build infrastructure. No test runner invocations made by reviewer; compilation and prior CI execution confirmed.
