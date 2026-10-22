# VB-88A03 Dispatch Test Review V4

**Candidate:** 3fca010b5f5aaaafb5f41b08b95200db001bab3e  
**Base:** dba19b95918aca7c23e91b8814afa2a9118449f8  
**Reviewer:** RTX3090 (independent test review)  
**Date:** 2026-10-22  
**Verdict:** APPROVED

## Scope Correction

This review covers **19 scoped tests**: 7 internal shard/journal tests + 9 registered-executor tests (4 new policy-dispatch + 5 retained original) + 3 fail-closed tests. Prior reports incorrectly counted 17 by conflating the retained original tests with the new policy-dispatch additions and misclassifying test boundaries.

## Internal Shard and Journal Tests (7 tests)

### crates/vb_runtime/src/journal/tests/strict_policy_port.rs

**`storage_adapter_persists_full_policy_decision_event`** (line 116)
- Constructs `DecisionRecord` with exact fixture fields (ticket run/step/seq/action/attempt/idempotency_key/capacity, admission digest [17;32], live digest [29;32], input digest [43;32], Clean taint, Allowed, Admission layer, Inactive emergency)
- Calls `adapter.append_policy_decision_strict(decision, EventSeq::new(0))`
- Reopens `FjallJournal` after drop; asserts `events_for_run` returns exactly one `PolicyDecisionEvent` with full record equality
- Kills mutation: **omission of the journal append** (would return empty events array)

**`queued_adapter_drains_prefix_before_strict_decision`** (line 140)
- Queues `RunSubmitted` for run_a, then `append_policy_decision_strict` for same run
- Asserts prefix flushed before decision via `assert_run_a_prefix_flushed`
- Submits and drains run_b to prove queue remains accepting
- Kills mutation: **ordering violation between prefix and strict append**

**`noop_and_volatile_adapters_fail_closed`** (line 169)
- Tests `NoopRuntimeJournal` and `VolatileRuntimeJournal` default trait implementations
- Asserts exact `UnsupportedOperation { operation: "append_policy_decision_strict" }` error
- Kills mutation: **silent success path in default implementations**

### crates/vb_runtime/src/shard/lifecycle_tests/strict_policy_sequence.rs

**`successful_strict_policy_port`** (line 167)
- Seeds shard journal sequence at 5 via `seed_journal_sequence`
- Calls `shard.append_policy_decision_strict(test_decision_record(ticket))`
- Asserts sequence advances to 6 (`seq_after.get() == 6`)
- Asserts recording journal captured exact one call with `vec![(expected, EventSeq::new(5))]`
- Asserts ticket unchanged via `assert_ticket_and_seq_unchanged`
- Kills mutation: **failed or missing sequence advance**

**`typed_strict_policy_port_failure`** (line 198)
- Injects `RuntimeError::UnsupportedOperation { operation: "test_policy_port_failure" }` via recording journal
- Asserts shard propagates exact typed error
- Asserts recording journal captured decision at seq before (`vec![(test_decision_record(ticket), seq_before)]`)
- Kills mutation: **ignored or swallowed append error**

**`overflow_at_u64_max`** (line 231)
- Seeds sequence at u64::MAX
- Asserts `RuntimeError::StorageJournalAppend` with source `JournalError::SequenceOverflow`
- Asserts recording journal has zero calls (`calls.len() == 0`)
- Kills mutation: **appending after overflow without detection**

**`wrong_ticket_rejected`** (line 263)
- Constructs wrong ticket with action ID 999 instead of actual
- Asserts `RuntimeError::InvalidActionCompletion`
- Asserts recording journal has zero calls (`calls.len() == 0`)
- Kills mutation: **accepting wrong ticket and proceeding to append**

## Registered-Executor Dispatch Tests (4 new + 5 retained = 9 total)

### crates/workspace_tests/tests/vb_7x0ol_probe_registered_executor/policy_dispatch/

**`allowed_policy_permits_effect_and_records_decision`** (allowed.rs line 171)
- Configures Allowed policy with digest [0x11u8; 32]
- Executes with real `FjallRegister` adapter
- Asserts receipt value equals 100
- Asserts `adapter.effect_count() == 1` and `adapter.value(scope()) == Some(100)`
- Asserts exactly one stored decision with full record equality: `stored == expected`
- Asserts stored ticket equals provided ticket
- Independent oracle: `compute_expected_input_digest(100, Clean, &ticket)` via blake3 hashing of all 8 ticket fields with domain prefix "velvet-ballistics:policy:ticket:"
- Kills mutations: **omission of decision append** (no stored record), **changed field in record** (full equality fails), **reused old current digest** (expected uses current allowed digest)

**`after_scheduling_policy_change_to_deny`** (changed_policy.rs line 195)
- Configures Allowed with [0x11u8; 32], then changes to Denied with [0x22u8; 32] after tick
- Executes; expects `RegisteredExecutionError::PolicyDenied(decision)`
- Asserts full record equality with expected (current live = [0x22u8; 32] denied digest)
- Asserts `admission != denied_digest` AND `admission != allowed_digest` (distinct pinned admission)
- Asserts `adapter.effect_count() == 0`
- Asserts exactly one stored decision matching expected
- Kills mutations: **reusing old current digest** (expected uses denied digest), **reusing old admission** (explicit distinct assertion), **changed field in record** (full equality)

**`reconcile_ignores_policy_change_after_effect`** (reconcile.rs line 173)
- Executes Allowed → effect committed
- Changes policy to Denied
- Reconciles; reads same receipt
- Asserts reconciled receipt equals original
- Asserts `adapter.effect_count() == 1` (no new effect)
- Asserts exactly one stored decision (no new decision recorded)
- Kills mutations: **re-evaluating policy on reconcile** (would store second decision)

**`unconfigured_policy_denies_execution`** (unconfigured.rs line 143)
- No policy configured (`CurrentPolicyIdentity::Unconfigured`)
- Executes; expects `PolicyDenied` with `DecisionCode::Denied`
- Asserts full record equality with expected (layer = Admission, identity = Unconfigured)
- Asserts `adapter.effect_count() == 0`
- Asserts exactly one stored decision
- Kills mutations: **treating unconfigured as allowed** (would succeed), **missing decision record** (no stored event)

### Original Registered-Executor Tests (5 retained)

From `crates/workspace_tests/tests/vb_7x0ol_probe_registered_executor.rs`:
- `execute_registered_action_commits_one_effect_and_completes_run`
- `reconcile_registered_action_reads_receipt_without_new_effect`
- `execute_registered_action_rejects_mismatched_grant_artifact`
- `recovery_of_pending_registered_action_blocked_pack_rejects`
- `recovery_of_pending_registered_action_reconcile_pack_is_resumable`

These retain the original registered-executor contract: execute/reconcile behavior, grant artifact validation, and recovery pack semantics. Not altered by this bead.

## Fail-Closed Tests (3 tests)

### crates/workspace_tests/tests/vb_policy_dispatch_fail_closed/

**`suspended_policy_denies_execution_with_full_record`** (suspended.rs line 229)
- Configures Suspended policy with digest [0x99u8; 32]
- Executes; expects `PolicyDenied` with `DecisionCode::Suspended`
- Asserts `decision.layer == PolicyLayer::Live`
- Asserts current live identity is Configured with suspended digest [0x99u8; 32]
- Asserts full record equality including ticket, input digest, Clean taint, Inactive emergency, NotEvaluated compat
- Asserts `adapter.effect_count() == 0`
- Captures pending boundary snapshot before execution; asserts unchanged after
- Asserts exact ticket identity preserved (`ticket_after == ticket`)
- Asserts exactly one stored decision matching expected
- Kills mutations: **treating Suspended as Allowed**, **missing decision record**, **altered pending state on failure**

**`emergency_deny_overlay_cannot_be_loosened_by_later_allowed_config`** (emergency.rs line 269)
- Configures Allowed, submits and ticks
- Activates emergency deny overlay via dedicated store persist
- Executes twice with two different Allowed digests ([0x44u8; 32] then [0x55u8; 32])
- Both executions denied with `DecisionCode::Denied`, `PolicyLayer::Emergency`, `InfluenceState::Active`
- First expected uses current live = [0x44u8; 32], second uses [0x55u8; 32] (distinct digests prove overlay doesn't freeze policy)
- Asserts `adapter.effect_count() == 0`
- Asserts pending state and ticket unchanged between both executions
- Asserts exactly two stored decisions matching respective expected records
- Kills mutations: **overlay loosened by later Allowed config**, **frozen current digest under overlay**, **missing second decision record**

**`strict_port_failure_returns_typed_runtime_error_with_zero_effects`** (strict_failure.rs line 4)
- Test wrapper overrides only `append_policy_decision_strict` on real journal/catalog/grants
- Executes; expects `RegisteredExecutionError::Runtime` wrapping `UnsupportedOperation { operation: "test_policy_decision_failure" }`
- Asserts `adapter.effect_count() == 0` (ZERO actual effects on injected append failure)
- Asserts pending state unchanged and ticket identity preserved
- Asserts zero stored policy decisions (`decision_events.len() == 0`)
- Asserts port was called exactly once with expected record and correct seq
- Kills mutation: **moving effect commit before strict append** (would produce nonzero effects on append failure); this is the definitive kill for this mutation, not a success-path test

## Rubric Compliance

### Exact Error Fields and Boundary Values
All typed error assertions check the exact variant and payload:
- `UnsupportedOperation { operation: "append_policy_decision_strict" }`
- `UnsupportedOperation { operation: "test_policy_port_failure" }`
- `RuntimeError::StorageJournalAppend { source: JournalError::SequenceOverflow }`
- `InvalidActionCompletion`
- `ReceiptMissing`
- `PolicyLayer::Admission` for Unverified/Unconfigured, `PolicyLayer::Live` for current policy, `PolicyLayer::Emergency` for overlay
- `InfluenceState::Active` for emergency, `InfluenceState::Inactive` for non-emergency
- Boundary: u64::MAX sequence overflow explicitly tested

### Real Public API / Scoped Gates
All tests use real production APIs:
- `Runtime::execute_registered_action`, `Runtime::reconcile_registered_action`
- `Runtime::configure_live_policy`, `Runtime::activate_emergency_deny`
- `Runtime::pending_boundary_snapshot`, `Runtime::lookup_pending_action_ticket`
- Real `FjallRegister` with `effect_count()` and `value(scope())`
- Real `FjallJournal` with `events_for_run` and `read_run_events`
- Real `ExecutorCatalog` with `submit_artifact` and `issue_register_grant`
- Test-only wrapper (`FailingPolicyPortJournal`) overrides only `append_policy_decision_strict`, delegates all real methods

### No Ignored Cases
All 12 required policy-dispatch cases are covered by the 7 new tests (4 registered dispatch + 3 fail-closed). The 7 internal tests cover the shard/journal port behavior. Total: 19 scoped tests.

### Deterministic Fixtures
All fixtures use explicit, deterministic values:
- Fixed i64 constants: 42, 7, 100, 99, 55, 77, 111, 222
- Fixed policy digests: [0x11u8; 32], [0x12u8; 32], [0x13u8; 32], [0x22u8; 32], [0x33u8; 32], [0x44u8; 32], [0x55u8; 32], [0x99u8; 32]
- Fixed ticket fields: step 3, seq 7, action 12, run_attempt 1, attempt 1, idempotency_key 0x0a0b0c0d0e0f1011u128, capacity 3
- No randomization, no time dependence

### Independent Expected Values / Full Record Equality
Test-side oracle `compute_expected_input_digest` independently computes expected input digest:
- Serializes input value as I64 bytes
- Uses byte 0 for Clean taint
- Hashes all 8 ticket fields with domain separation prefix "velvet-ballistics:policy:ticket:"
- Does NOT call `LivePolicy.evaluate` or any production encoding function

Full record equality verified in all 7 new policy-dispatch tests via `assert_eq!(stored, expected)` or `assert_eq!(decision.as_ref(), expected)`.

### Named Mutation Thought Experiments

| Mutation | Killed By | Why |
|----------|-----------|-----|
| Omit journal append | `allowed_policy_permits_effect_and_records_decision` | Success path checks exactly one stored decision with full record equality |
| Move effect commit before strict append | `strict_port_failure_returns_typed_runtime_error_with_zero_effects` | Injected append failure asserts ZERO actual effects on real FjallRegister |
| Reuse old current digest after policy change | `after_scheduling_policy_change_to_deny` | Expected record uses new denied digest; full equality fails if old used |
| Reuse old admission pin | `after_scheduling_policy_change_to_deny` | Explicit assertion `admission != allowed_digest` and `admission != denied_digest` |
| Ignore/swallow append error | `typed_strict_policy_port_failure` | Exact typed error must propagate to caller |
| Change any field in DecisionRecord | All 7 new policy-dispatch tests | Full record equality `assert_eq!(stored, expected)` catches any single-field change |
| Skip sequence advance | `successful_strict_policy_port` | Asserts exact sequence advance from 5 to 6 |
| Accept wrong ticket | `wrong_ticket_rejected` | Asserts `InvalidActionCompletion` and zero journal calls |
| Append after u64::MAX overflow | `overflow_at_u64_max` | Asserts `SequenceOverflow` and zero journal calls |
| Treat Suspended as Allowed | `suspended_policy_denies_execution_with_full_record` | Asserts exact `DecisionCode::Suspended` and full record |
| Loosen emergency overlay with later Allowed | `emergency_deny_overlay_cannot_be_loosened_by_later_allowed_config` | Two executions both denied despite different later Allowed configs |
| Treat Unconfigured as Allowed | `unconfigured_policy_denies_execution` | Asserts exact `PolicyDenied` with Unconfigured identity |
| Re-evaluate policy on reconcile | `reconcile_ignores_policy_change_after_effect` | Asserts exactly one stored decision after policy change and reconcile |
| Silent success in noop/volatile adapters | `noop_and_volatile_adapters_fail_closed` | Asserts exact `UnsupportedOperation` error from defaults |
| Prefix not flushed before strict append | `queued_adapter_drains_prefix_before_strict_decision` | Asserts prefix events persisted before decision |

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

## Explicitly Unreviewed Here

**Admission-pin rejection (wrong pin used at execution):** The `after_scheduling_policy_change_to_deny` test does NOT test this. The admission authority in that test is the valid artifact digest; the test verifies that the pin differs from the current live policy (allowed vs denied), not that a wrong pin is rejected. Rejection of wrong admission pins is covered by separate recovery/forged-admission tests (e.g., vb_c4hzw_legacy_admission, vb_c4hzw_forged_admission) that are outside the scope of this dispatch review.

## Findings

No findings. All 19 scoped tests compile and pass (per root prior execution logs; reviewer has no runner and does not execute tests). All rubric criteria satisfied. Test design demonstrates thorough coverage of the policy dispatch behavior with exact-typed assertions, independent oracles, and comprehensive fail-closed verification.

## Command Provenance

Tests compiled and verified via workspace build infrastructure. No test runner invocations made by reviewer; compilation and prior CI execution confirmed from root artifacts. Reviewer read-only inspection of test source and assertions only.
