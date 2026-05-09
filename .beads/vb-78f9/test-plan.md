# Test Plan: vb-78f9 — Action Contract Schema Validation

## 1. Scope

This plan covers the action contract schema validation system spanning:
- `vb_core::action` — types, taint propagation, idempotency validation
- `vb_runtime::action` — `ActionRegistry`, `IdempotencyTracker`, dispatch
- `vb_runtime::engine::action` — `execute_do`, `resume_action_outcome`, ticket issuance
- `vb_ui::verify::action_policy` — policy analysis and reporting

---

## 2. Test Distribution (Testing Trophy)

| Layer | Count | Ratio |
|-------|-------|-------|
| Unit tests | 45 | 70% |
| Integration tests | 10 | 15% |
| Property-based tests (proptest) | 6 | 10% |
| BDD Given-When-Then scenarios | 3 | 5% |

---

## 3. Unit Tests

### 3.1 Core ABI Types (`vb_core::action`)

#### 3.1.1 ActionContract construction and field access

```
test_action_contract_default_idempotency
Test: ActionContract::default() produces DeterministicPure idempotency
Contract: Idempotency enum defaults to DeterministicPure = 0

test_action_contract_default_side_effect
Test: ActionContract::default() produces SideEffect::None
Contract: SideEffect enum defaults to None = 0

test_action_contract_default_retry_safety
Test: ActionContract::default() produces RetrySafety::Safe
Contract: RetrySafety enum defaults to Safe = 0

test_action_contract_all_fields_accessible
Test: All ActionContract fields are readable and writable
Contract: No field access panics or truncates values

test_action_id_newtype_get
Test: ActionId::new(u16) and ActionId::get() roundtrip correctly
Contract: ActionId stores exact u16 value with no lossy conversion
```

#### 3.1.2 Idempotency enum variants

```
test_idempotency_all_variants_constructible
Test: Idempotency::{DeterministicPure, IdempotentExternal, AtLeastOnceExternal} are constructible
Contract: Each variant is accessible and maps to its discriminant value

test_idempotency_eq_equality
Test: Idempotency::DeterministicPure == Idempotency::DeterministicPure
Contract: Eq and PartialEq implementations are consistent with discriminant
```

#### 3.1.3 SideEffect enum variants

```
test_side_effect_all_variants_constructible
Test: SideEffect::{None, Writes, Sends, Creates, Destroys} are constructible
Contract: Each variant is accessible and maps to its discriminant value

test_side_effect_from_u8_roundtrip
Test: SideEffect::from_u8(variant as u8) == Some(variant)
Contract: Discriminant-to-variant roundtrip is lossless for all variants
```

#### 3.1.4 RetrySafety enum variants

```
test_retry_safety_all_variants_constructible
Test: RetrySafety::{Safe, KeyRequired, Unsafe} are constructible
Contract: Each variant is accessible and maps to its discriminant value

test_retry_safety_from_u8_roundtrip
Test: RetrySafety::from_u8(variant as u8) == Some(variant)
Contract: Discriminant-to-variant roundtrip is lossless for all variants
```

#### 3.1.5 ActionTicket field validation

```
test_action_ticket_fields_correct
Test: ActionTicket { run, step, seq, action, attempt, idempotency_key, capacity } constructs without panic
Contract: All fields store the exact values provided at construction

test_action_ticket_attempt_is_1_indexed
Test: ActionTicket::new() requires attempt >= 1
Contract: attempt field is 1-indexed per IT2 invariant

test_action_ticket_seq_monotonic
Test: SeqNo::next() returns strictly incremented value
Contract: IT3 invariant — seq is strictly monotonic within RunId
```

#### 3.1.6 ActionOutcome enum variants

```
test_action_outcome_ready_variant
Test: ActionOutcome::Ready(ActionOutputReady { output_slot, value, taint }) constructs
Contract: Ready variant carries output value and taint

test_action_outcome_suspended_variant
Test: ActionOutcome::Suspended(ActionTicket) constructs
Contract: Suspended variant carries ticket for resumption

test_action_outcome_failed_variant
Test: ActionOutcome::Failed(ActionFailure { reason, retry_policy }) constructs
Contract: Failed variant carries reason and retry policy
```

#### 3.1.7 ActionError enum variants

```
test_action_error_unknown_action_contains_action
Test: ActionError::UnknownAction { action: ActionId::new(42) } contains correct action
Contract: Error carries the unregistered ActionId for debugging

test_action_error_payload_too_large_fields
Test: ActionError::PayloadTooLarge { max_bytes: 100, actual_bytes: 200 } contains correct values
Contract: Error carries max and actual byte counts for PAYLOAD_TOO_LARGE reporting

test_action_error_output_slot_out_of_bounds_fields
Test: ActionError::OutputSlotOutOfBounds { slot: 5, max_slots: 4 } contains correct values
Contract: Error carries slot index and max for dispatch validation

test_action_error_all_variants_display
Test: All ActionError variants produce non-empty Display strings
Contract: All variants implement Display without panicking

test_action_error_invalid_ticket_variant
Test: ActionError::InvalidTicket constructs without panic
Contract: §5.1 — InvalidTicket is a valid error variant for ticket mismatch

test_action_error_non_idempotent_replay_blocked_variant
Test: ActionError::NonIdempotentReplayBlocked constructs without panic
Contract: §5.1 — NonIdempotentReplayBlocked is a valid error variant for non-idempotent replay
```

#### 3.1.8 IdempotencyViolation enum variants

```
test_idempotency_violation_missing_key_contains_side_effect
Test: IdempotencyViolation::MissingKey(SideEffect::Writes) contains SideEffect
Contract: MissingKey carries side effect type for MissingKey(SideEffect) classification

test_idempotency_violation_secret_in_key_contains_slot
Test: IdempotencyViolation::SecretInKey(3) contains slot index 3
Contract: SecretInKey carries the violating slot index

test_idempotency_violation_random_in_key_contains_slot
Test: IdempotencyViolation::RandomInKey(2) contains slot index 2
Contract: RandomInKey carries the violating slot index

test_idempotency_violation_time_in_key_contains_slot
Test: IdempotencyViolation::TimeInKey(1) contains slot index 1
Contract: TimeInKey carries the violating slot index
```

### 3.2 Taint Propagation (`propagate_action_taint`)

```
test_propagate_deterministic_pure_preserves_clean
Test: propagate_action_taint(DeterministicPure, Taint::Clean) == Taint::Clean
Contract: Postcondition — pure/idempotent returns input_taint unchanged (join identity)

test_propagate_deterministic_pure_preserves_secret
Test: propagate_action_taint(DeterministicPure, Taint::Secret) == Taint::Secret
Contract: TT1 invariant — no downgrade of Secret

test_propagate_deterministic_pure_preserves_derived_from_secret
Test: propagate_action_taint(DeterministicPure, Taint::DerivedFromSecret) == Taint::DerivedFromSecret
Contract: TT2 invariant — no downgrade of DerivedFromSecret

test_propagate_at_least_once_upgrades_secret
Test: propagate_action_taint(AtLeastOnceExternal, Taint::Secret) == Taint::DerivedFromSecret
Contract: TT3 — AtLeastOnce upgrades Secret to DerivedFromSecret

test_propagate_at_least_once_preserves_clean
Test: propagate_action_taint(AtLeastOnceExternal, Taint::Clean) == Taint::Clean
Contract: TT4 — AtLeastOnce preserves Clean

test_propagate_idempotent_external_preserves_all
Test: propagate_action_taint(IdempotentExternal, Taint::DerivedFromSecret) == Taint::DerivedFromSecret
Contract: IdempotentExternal acts as identity for taint propagation

test_propagate_deterministic_pure_preserves_random
Test: propagate_action_taint(DeterministicPure, Taint::Random) == Taint::Random
Contract: Pure computation preserves Random taint
```

### 3.3 Idempotency Verification (`verify_idempotency`)

```
test_verify_idempotency_side_effect_none_always_passes
Test: verify_idempotency(SideEffect::None, Safe, &[]) == Ok(())
Contract: C1 — side_effect == None always returns Ok

test_verify_idempotency_retry_safe_always_passes
Test: verify_idempotency(SideEffect::Writes, Safe, &[]) == Ok(())
Contract: C2 — retry_safety == Safe always returns Ok

test_verify_idempotency_key_required_with_clean_slots_passes
Test: verify_idempotency(Writes, KeyRequired, &[Taint::Clean, Taint::Clean]) == Ok(())
Contract: C3 — KeyRequired with only Clean slots returns Ok

test_verify_idempotency_unsafe_always_fails
Test: verify_idempotency(SideEffect::Writes, Unsafe, &[]) == Err(MissingKey(Writes))
Contract: E1 — Unsafe returns Err(MissingKey(side_effect))

test_verify_idempotency_key_required_with_secret_slot_fails
Test: verify_idempotency(Writes, KeyRequired, &[Taint::Secret]) == Err(SecretInKey(0))
Contract: E2 — KeyRequired with Secret slot returns Err(SecretInKey(slot_index))

test_verify_idempotency_key_required_with_derived_secret_slot_fails
Test: verify_idempotency(Writes, KeyRequired, &[Taint::DerivedFromSecret]) == Err(SecretInKey(0))
Contract: E2 — KeyRequired with DerivedFromSecret slot returns Err(SecretInKey(slot_index))

test_verify_idempotency_key_required_with_mixed_slots_reports_first_violation
Test: verify_idempotency(Writes, KeyRequired, &[Taint::Clean, Taint::Secret, Taint::Clean]) == Err(SecretInKey(1))
Contract: First violating slot index is reported in SecretInKey

test_verify_idempotency_key_required_empty_key_slots_fails
Test: verify_idempotency(Writes, KeyRequired, &[]) == Err(MissingKey(Writes))
Contract: Empty key_slots with KeyRequired triggers MissingKey
```

### 3.4 ActionRegistry (`vb_runtime::action`)

```
test_registry_register_single_contract
Test: registry.register(contract) then resolve_compile_time(contract.id) returns Ok(&contract)
Contract: IR1 — register stores contract and resolve returns it

test_registry_register_returns_registered_contract
Test: After register(contract), resolve_compile_time returns the exact same contract reference
Contract: IR1 — contract.id matches exactly in resolve result

test_registry_register_idempotent_empty_slot
Test: Re-registering same ActionId with identical contract on Empty slot returns Ok
Contract: IR2 — idempotent when slot is Empty

test_registry_register_duplicate_on_occupied_slot_fails
Test: register(contract1) then register(contract2) with same ActionId returns Err(DispatchFailed)
Contract: IR2 — re-registering occupied slot returns DispatchFailed

test_registry_register_action_id_at_max_u16_boundary
Test: register with ActionId(65534) succeeds
Contract: Postcondition — id.get() >= 65535 returns UnknownAction

test_registry_register_action_id_at_max_u16_plus_one_fails
Test: register with ActionId(65535) returns Err(UnknownAction)
Contract: Postcondition — id.get() >= 65535 returns UnknownAction

test_registry_len_after_single_register
Test: After register(contract), len() == 1
Contract: IR3 — len() equals max(action_id.get()) + 1

test_registry_len_with_sparse_ids
Test: Register ActionId(0) and ActionId(100), len() == 101
Contract: IR3 — sparse array with gap slots accounted in len

test_registry_registered_contracts_returns_ascending_order
Test: Register contracts with ActionId values in random order, iterate registered_contracts
Contract: IR4 — returned contracts in strictly ascending ActionId order

test_registry_resolve_unknown_action_returns_error
Test: resolve_compile_time(ActionId(999)) returns Err(UnknownAction { action: ActionId(999) })
Contract: Precondition U — unregistered action returns UnknownAction error

test_registry_resolve_registered_returns_ok
Test: register(contract) then resolve_compile_time(contract.id) returns Ok(&contract)
Contract: Precondition C — registered action with matching id returns Ok

test_registry_dispatch_unknown_action_returns_error
Test: dispatch(input { action: ActionId(42), .. }) returns Err(UnknownAction { action: ActionId(42) })
Contract: E2 — dispatch on unregistered action returns UnknownAction

test_registry_dispatch_with_zero_max_bytes_and_nonzero_slots_fails
Test: dispatch with contract.max_input_bytes=0 and contract.input_slot_count=1 returns PayloadTooLarge
Contract: E1 — max_input_bytes==0 with input_slot_count>0 returns PayloadTooLarge { max_bytes: 0, actual_bytes: 0 }
```

### 3.5 ActionRegistry dispatch path

```
test_registry_dispatch_returns_suspended_ticket
Test: dispatch on registered action returns Ok(ActionOutcome::Suspended(ticket))
Contract: Postcondition C — returns Suspended with ticket from input

test_registry_dispatch_ticket_idempotency_key_matches
Test: dispatch returns ticket with idempotency_key == compute_idempotency_key(run, seq, action)
Contract: Postcondition C — ticket.idempotency_key matches expected computation

test_registry_dispatch_ticket_attempt_is_one
Test: dispatch returns ticket with attempt == 1
Contract: IT2 — attempt is always 1-indexed on first dispatch
```

### 3.6 IdempotencyTracker (`vb_runtime::action`)

```
test_tracker_mark_completed_new_key_succeeds
Test: mark_completed(ticket) on never-seen key returns Ok(())
Contract: C — new key records completion and returns Ok

test_tracker_is_completed_after_mark
Test: After mark_completed(ticket), is_completed(&ticket) == true
Contract: IIT1 — is_completed returns true after mark_completed

test_tracker_mark_completed_duplicate_key_fails
Test: mark_completed(ticket) twice returns Err(CompletionAlreadyRecorded)
Contract: IIT2 — duplicate completion returns CompletionAlreadyRecorded

test_tracker_at_capacity_evicts_oldest
Test: Fill tracker to capacity, mark new key, oldest entry is evicted
Contract: IIT3 — oldest by insertion order evicted before new insert

test_tracker_eviction_wraps_at_capacity
Test: Fill, evict, fill again — cursor wraps and overwrites oldest position
Contract: IIT4 — eviction is FIFO by order vector, wrapping at capacity via cursor

test_tracker_len_respects_capacity
Test: Tracker with capacity N never exceeds N entries
Contract: IIT3 — eviction ensures len() <= capacity at all times

test_tracker_is_completed_false_for_unseen_key
Test: is_completed(&ticket) for never-marked key returns false
Contract: IIT1 — is_completed only true after explicit mark_completed
```

### 3.7 Execute Do (`vb_runtime::engine::action`)

```
test_execute_do_registered_deterministic_pure_with_clean_input
Test: execute_do(clean_input, DeterministicPure action, Taint::Clean) returns AwaitingAction with attempt=1
Contract: HA1 — Clean input on DeterministicPure returns AwaitingAction(ticket) with attempt=1

test_execute_do_at_least_once_with_secret_input_propagates_taint
Test: execute_do(tainted, AtLeastOnceExternal, Taint::Secret) returns AwaitingAction with DerivedFromSecret
Contract: HA2 — AtLeastOnce with Secret input propagates to DerivedFromSecret

test_execute_do_idempotency_key_matches_compute
Test: execute_do returns ticket with idempotency_key == compute_idempotency_key(run, seq, action)
Contract: Postcondition C — ticket idempotency_key deterministic match

test_execute_do_unregistered_action_returns_unknown_action_error
Test: execute_do with unknown ActionId returns Err(RuntimeEngineError::Action(UnknownAction))
Contract: E3 — unknown action returns UnknownAction error

test_execute_do_deterministic_pure_with_secret_input_fails_taint
Test: execute_do(tainted, DeterministicPure, Taint::Secret) returns Err(TaintViolation)
Contract: E1 — DeterministicPure with tainted input returns TaintViolation

test_execute_do_deterministic_pure_with_derived_secret_fails_taint
Test: execute_do(tainted, DeterministicPure, Taint::DerivedFromSecret) returns Err(TaintViolation)
Contract: E1 — DeterministicPure with non-Clean taint returns TaintViolation

test_execute_do_missing_capability_returns_capability_denied
Test: execute_do with required capability not in granted returns Err(CapabilityDenied)
Contract: E2 — required capability not in granted returns CapabilityDenied error
```

### 3.8 Resume Action Outcome (`vb_runtime::engine::action`)

```
test_resume_ready_writes_output_slot
Test: resume_action_outcome(Ready { output_slot, value, taint }) writes to output_slot
Contract: HA5 — Ready outcome writes value and taint to ready.output_slot

test_resume_ready_output_in_bounds
Test: resume_action_outcome(Ready) with output_slot < contract.output_slot_count succeeds
Contract: C1 — Ready with valid output_slot returns Continue

test_resume_ready_output_out_of_bounds
Test: resume_action_outcome(Ready) with output_slot >= contract.output_slot_count returns Err(OutputSlotOutOfBounds)
Contract: EA13 — out-of-bounds output_slot propagates SlotOutOfBounds error from write_slot_with_taint

test_resume_failed_retryable_below_capacity_returns_retry
Test: resume_action_outcome(Failed { retryable: Retryable }) below capacity returns AwaitingAction
Contract: C2 — Failed with Retryable and attempt < capacity returns retry ticket

test_resume_failed_retryable_increments_attempt_and_seq
Test: resume with Retryable returns ticket with attempt+1 and seq+1
Contract: C2 — retry_ticket.attempt = attempt + 1, retry_ticket.seq = seq + 1

test_resume_failed_non_retryable_returns_error
Test: resume_action_outcome(Failed { retryable: NonRetryable }) returns Err(UnsupportedPrimitive)
Contract: E1 — NonRetryable returns UnsupportedPrimitive

test_resume_failed_at_capacity_returns_exhausted
Test: resume_action_outcome(Failed { retryable: Retryable }) at capacity returns Err(RetryExhausted)
Contract: E1 — exhausted attempts returns RetryExhausted
```

### 3.9 Validate Action Dispatch (`vb_runtime::action`)

```
test_validate_action_dispatch_valid_slots
Test: validate_action_dispatch with valid input_slot and output_slot < slot_count returns Ok(())
Contract: C — valid slots return Ok

test_validate_action_dispatch_uninitialized_input_slot
Test: validate_action_dispatch with uninitialized input_slot returns Err(DispatchFailed)
Contract: E1 — uninitialized input slot returns DispatchFailed

test_validate_action_dispatch_out_of_bounds_input_slot
Test: validate_action_dispatch with input_slot >= slot_count returns Err(DispatchFailed)
Contract: E1 — out of bounds input slot returns DispatchFailed

test_validate_action_dispatch_out_of_bounds_output_slot
Test: validate_action_dispatch with output_slot >= slot_count returns Err(DispatchFailed)
Contract: E2 — output_slot >= slot_count returns DispatchFailed
```

### 3.10 Compute Idempotency Key (`vb_runtime::engine::action`)

```
test_compute_idempotency_key_deterministic
Test: compute_idempotency_key(run, seq, action) called twice returns identical u128
Contract: IT1 — identical inputs produce identical idempotency_key

test_compute_idempotency_key_different_inputs_different_keys
Test: compute_idempotency_key for different (run, seq, action) tuples produces different keys
Contract: Deterministic key computation with no collision on distinct inputs

test_compute_idempotency_key_zero_seq
Test: compute_idempotency_key(run, SeqNo(0), action) produces valid u128
Contract: Key computation handles zero seq without panic

test_compute_idempotency_key_max_action_id
Test: compute_idempotency_key(run, seq, ActionId(u16::MAX)) produces valid u128
Contract: Key computation handles max ActionId without overflow
```

### 3.11 ActionPolicyReport (`vb_ui::verify::action_policy`)

```
test_action_policy_report_missing_contract_has_timeout_issue
Test: analyze_action_policies on Do node with no contract includes MissingTimeout issue
Contract: AP2 — missing contract implies MissingTimeout issue

test_action_policy_report_missing_contract_has_missing_idempotency_issue
Test: analyze_action_policies on Do node with no contract includes MissingIdempotency issue
Contract: AP3 — missing contract implies MissingIdempotency issue

test_action_policy_report_unsafe_retry_contract_has_unsafe_retry_issue
Test: analyze_action_policies on contract with Unsafe returns UnsafeRetry issue
Contract: AP4 — Unsafe retry_safety implies UnsafeRetry issue

test_action_policy_report_strict_eligible_requires_all_conditions
Test: Report with strict_eligible == true has idempotency == DeterministicPure, has_timeout == true, issues empty
Contract: AP1 — strict_eligible true implies all conditions met

test_action_policy_report_duplicate_dos_deduplicated
Test: analyze_action_policies on multiple Do nodes with same action produces single report
Contract: AP5 — duplicate Do nodes deduplicated to one report

test_action_policy_report_timeout_zero_implies_missing_timeout
Test: Report with timeout_ms == 0 includes MissingTimeout issue
Contract: AP2 — zero timeout implies MissingTimeout

test_action_policy_report_strict_eligible_false_when_issues_present
Test: Report with issues non-empty has strict_eligible == false
Contract: AP1 — issues present blocks strict_eligible
```

---

## 4. Integration Tests

### 4.1 Registry-Dispatch-Complete Flow

```
test_full_action_lifecycle: register -> dispatch -> resume (Ready)
Test: Full cycle from contract registration through dispatch, completion, and ticket finalization.
Steps:
  1. Register ActionContract for ActionId(1) as DeterministicPure, Safe, timeout=100ms
  2. dispatch(input { action: ActionId(1), slots: [Clean], bytes: 50 })
  3. resume_action_outcome(Ready { output_slot: 0, value: V, taint: Clean })
  4. IdempotencyTracker.is_completed(ticket) == true
Contract: HA3, HA4, HA5 — dispatch, completion, tracking all succeed end-to-end

test_retry_flow_within_capacity
Test: Action fails with Retryable, resumes, fails again, resumes successfully.
Steps:
  1. Register ActionContract for ActionId(2) as AtLeastOnceExternal, KeyRequired, capacity=3
  2. dispatch -> Failed(Retryable) -> resume returns retry ticket with attempt=2
  3. dispatch -> Failed(Retryable) -> resume returns retry ticket with attempt=3
  4. dispatch -> Ready -> completion recorded
Contract: HA6 — retry increments attempt and seq correctly

test_dispatch_queue_full_rejects
Test: ActionRegistry dispatch returns QueueFull when internal queue at capacity.
Steps:
  1. Fill dispatch queue to capacity (N concurrent in-flight actions)
  2. Attempt additional dispatch returns Err(QueueFull)
Contract: EA10 — QueueFull error when capacity reached
```

### 4.2 Capability Checking

```
test_execute_do_capability_check_blocks_ungranted
Test: Action requires Capability("network"), granted set only has Capability("disk").
Steps:
  1. Register ActionContract requiring Capability("network")
  2. execute_do with granted={Capability("disk")} returns Err(CapabilityDenied)
Contract: E2 — capability check returns CapabilityDenied with required/granted sets

test_execute_do_capability_check_passes_with_matching_grant
Test: Action requires Capability("network"), granted set includes Capability("network").
Steps:
  1. Register ActionContract requiring Capability("network")
  2. execute_do with granted={Capability("network")} returns Ok(AwaitingAction)
Contract: Postcondition C — capability satisfied allows execution
```

### 4.3 Taint Propagation Chain

```
test_taint_propagates_through_multiple_at_least_once_actions
Test: Secret input to AtLeastOnceExternal action produces DerivedFromSecret, which blocks next DeterministicPure.
Steps:
  1. execute_do(DeterministicPure, Taint::Clean) -> Clean output
  2. execute_do(AtLeastOnceExternal, Taint::Secret) -> DerivedFromSecret output
  3. execute_do(DeterministicPure, Taint::DerivedFromSecret) -> Err(TaintViolation)
Contract: TT3, TT4 — AtLeastOnce upgrades Secret to DerivedFromSecret; next DeterministicPure blocks

test_secret_input_blocks_pure_action
Test: Secret taint on input to DeterministicPure action returns TaintViolation.
Steps:
  1. execute_do(DeterministicPure, Taint::Secret) returns Err(TaintViolation { step })
Contract: EA1 — DeterministicPure blocks Secret taint input

test_clean_input_passes_through_pure_action
Test: Clean taint on input to DeterministicPure action succeeds.
Steps:
  1. execute_do(DeterministicPure, Taint::Clean) returns AwaitingAction with Clean
Contract: HA10 — Clean input on DeterministicPure passes through unchanged
```

### 4.4 UI Policy Analysis Integration

```
test_analyze_policies_on_fully_covered_workflow
Test: Workflow where all Do nodes have contracts passes strict_eligible checks.
Steps:
  1. Analyze workflow with all Do nodes registered, DeterministicPure, timeouts set, no unsafe retries
  2. All ActionPolicyReport have strict_eligible == true
Contract: HA8 — fully-covered workflow has all reports strict_eligible

test_analyze_policies_reports_missing_contracts
Test: Workflow with unregistered Do actions generates MissingTimeout and MissingIdempotency issues.
Steps:
  1. Analyze workflow with one Do node lacking contract registration
  2. Report for missing contract has MissingTimeout + MissingIdempotency issues
Contract: EA14 — missing contract generates correct issue set

test_analyze_policies_reports_unsafe_retry
Test: Workflow with Unsafe retry_safety contract generates UnsafeRetry issue.
Steps:
  1. Register ActionContract with RetrySafety::Unsafe
  2. Analyze workflow using that action
  3. Report has UnsafeRetry issue
Contract: EA15 — unsafe retry generates UnsafeRetry issue
```

### 4.5 Encoding/Decoding Roundtrip

```
test_action_error_encoding_roundtrip
Test: ActionError -> postcard bytes -> ActionError recovers exact variant and fields.
Steps:
  1. Encode ActionError::PayloadTooLarge { max_bytes: 100, actual_bytes: 200 }
  2. Decode bytes back to ActionError
  3. Result equals original error exactly
Contract: Section 17 code mapping preserved through encode/decode

test_action_ticket_encoding_roundtrip
Test: ActionTicket -> postcard bytes -> ActionTicket recovers exact field values.
Steps:
  1. Encode ActionTicket { run, step, seq, action, attempt: 2, idempotency_key, capacity: 5 }
  2. Decode bytes back to ActionTicket
  3. All fields equal original including idempotency_key
Contract: Ticket invariants preserved through serialization

test_action_outcome_encoding_roundtrip
Test: ActionOutcome -> postcard bytes -> ActionOutcome recovers exact variant.
Steps:
  1. Encode ActionOutcome::Failed { reason: ..., retry_policy: NonRetryable }
  2. Decode bytes back to ActionOutcome
  3. Result equals original variant and fields
Contract: EncodingFailed only on postcard failure, not on valid roundtrip
```

---

## 5. Property-Based Tests (Proptest)

### 5.1 Idempotency Key Determinism

```
proptest_compute_idempotency_key_deterministic_across_runs
Property: For all RunId, SeqNo, ActionId triples, calling compute_idempotency_key twice returns identical u128.
Strategy: Random sample of 1000 (RunId, SeqNo, ActionId) combinations.
Contract: IT1 — idempotency_key deterministic across invocations

proptest_idempotency_key_no_collision_on_adjacent_seq
Property: compute_idempotency_key(run, seq, action) != compute_idempotency_key(run, seq+1, action)
Strategy: Random (RunId, ActionId) with seq ranging across boundary.
Contract: IT1 — distinct seq values produce distinct keys
```

### 5.2 Registry Invariants

```
proptest_registry_resolve_returns_what_was_stored
Property: For all ActionContract with valid id < 65535, register then resolve returns identical contract.
Strategy: 500 random ActionContract instances with valid ids.
Contract: IR1 — resolve returns exact stored contract

proptest_registry_len_consistency
Property: After registering N contracts with ids {id_0,...,id_{N-1}}, len() == max(id_i) + 1.
Strategy: Random sparse registration patterns, N from 1 to 100.
Contract: IR3 — len matches sparse array semantics

proptest_registry_duplicate_registration_consistency
Property: Re-registering same ActionId with identical contract on occupied slot returns Err(DispatchFailed).
Strategy: Random contract, register twice.
Contract: IR2 — occupied slot rejects re-registration
```

### 5.3 Taint Propagation Invariants

```
proptest_taint_propagation_never_downgrades
Property: For all Idempotency, Taint combinations, propagate_action_taint never reduces secrecy level.
Seccinctness ordering: Clean < DerivedFromSecret < Secret; Random is incomparable.
Strategy: 1000 random (Idempotency, Taint) pairs.
Contract: TT1, TT2 — DeterministicPure never downgrades Secret/DerivedFromSecret

proptest_at_least_once_upgrades_secret
Property: propagate_action_taint(AtLeastOnceExternal, Secret) == DerivedFromSecret
Strategy: 100 samples with Secret input.
Contract: TT3 — AtLeastOnce upgrades Secret

proptest_pure_preserves_non_clean_taints
Property: For all non-Clean Taint values, propagate_action_taint(DeterministicPure, t) == t.
Strategy: 100 samples each for Secret, DerivedFromSecret, Random.
Contract: TT1, TT2 — pure preserves non-Clean taints
```

### 5.4 IdempotencyTracker Eviction

```
proptest_tracker_eviction_fifo_order
Property: After filling to capacity N, then inserting N new entries, the oldest N entries are evicted.
Strategy: Fill to capacity with keys K0..K{N-1}, insert KN..K{2N-1}, verify K0..K{N-1} evicted.
Contract: IIT3, IIT4 — FIFO eviction by insertion order

proptest_tracker_capacity_never_exceeded
Property: After any sequence of mark_completed calls, tracker.len() <= tracker.capacity().
Strategy: 100 random sequences of insert/evict operations.
Contract: IIT3 — capacity invariant maintained
```

### 5.5 ActionContract Field Limits

```
proptest_action_contract_max_bytes_bounds
Property: For all ActionContract, max_input_bytes and max_output_bytes are within u32 range.
Strategy: 500 random contracts with byte limit fields.
Contract: Schema validation — byte fields fit in u32 without truncation

proptest_action_contract_timeout_ms_bounds
Property: For all ActionContract, timeout_ms fits in u64 without overflow.
Strategy: 500 random contracts with timeout_ms field.
Contract: Schema validation — timeout field fits in u64
```

### 5.6 ActionError Variant Exhaustiveness

```
proptest_action_error_all_variants_encode_decode
Property: All ActionError variants survive postcard encode/decode roundtrip.
Strategy: Generate one instance of each ActionError variant, verify roundtrip equality.
Contract: Section 17 code mapping stable for all error variants

proptest_idempotency_violation_all_variants_encode_decode
Property: All IdempotencyViolation variants survive postcard encode/decode roundtrip.
Strategy: Generate one instance of each IdempotencyViolation variant, verify roundtrip equality.
Contract: Encoding stable for all violation variants
```

---

## 6. BDD Given-When-Then Scenarios

### 6.1 Action Dispatch with Retry

```gherkin
Feature: Action Retry Safety

  Scenario: Retryable action recovers from transient failure
    Given an ActionContract for "process_payment" with:
      | field | value |
      | idempotency | AtLeastOnceExternal |
      | retry_safety | KeyRequired |
      | capacity | 3 |
      | timeout_ms | 5000 |
    And a Clean input slot containing payment_data
    And the action is registered in the ActionRegistry
    When execute_do is called with the input
    Then the runtime returns AwaitingAction with attempt=1
    And idempotency_key matches compute_idempotency_key(run, seq, action)
    When the action returns Failed with retry_policy=Retryable
    And attempt < capacity
    Then resume_action_outcome returns AwaitingAction with attempt=2
    And retry_ticket.seq = original_seq + 1
    When the retried action returns Ready with output_slot=0
    Then the output slot contains the ready value
    And IdempotencyTracker.mark_completed returns Ok(())

  Scenario: Non-retryable action fails permanently after first attempt
    Given an ActionContract for "send_email" with:
      | field | value |
      | idempotency | AtLeastOnceExternal |
      | retry_safety | Unsafe |
    And the action is registered
    When execute_do is called
    Then the runtime returns AwaitingAction with attempt=1
    When the action returns Failed with retry_policy=NonRetryable
    Then resume_action_outcome returns Err(UnsupportedPrimitive)

  Scenario: Retry exhaustion after maximum attempts
    Given an ActionContract with capacity=2
    And the action is registered
    When execute_do is called
    Then attempt=1
    When Failed with Retryable is returned
    Then resume returns retry ticket with attempt=2
    When Failed with Retryable is returned again
    Then resume returns Err(RetryExhausted { action, attempts: 2 })
```

### 6.2 Taint Propagation Blocking Non-Pure Actions

```gherkin
Feature: Taint Propagation Enforcement

  Scenario: Secret-tainted input blocks DeterministicPure action
    Given an ActionContract for "hash_secret" with idempotency=DeterministicPure
    And an input slot with Taint::Secret
    When execute_do is called with the tainted input
    Then the runtime returns Err(RuntimeEngineError::TaintViolation { step })

  Scenario: DerivedFromSecret blocks DeterministicPure action
    Given an input slot with Taint::DerivedFromSecret
    And an ActionContract with idempotency=DeterministicPure
    When execute_do is called
    Then the runtime returns Err(TaintViolation)

  Scenario: Clean input passes through DeterministicPure action
    Given an ActionContract with idempotency=DeterministicPure
    And an input slot with Taint::Clean
    When execute_do is called
    Then the runtime returns AwaitingAction
    And the output taint is Clean

  Scenario: Secret input is upgraded to DerivedFromSecret by AtLeastOnceExternal
    Given an ActionContract with idempotency=AtLeastOnceExternal
    And an input slot with Taint::Secret
    When execute_do is called
    Then the runtime returns AwaitingAction
    And the propagated taint is Taint::DerivedFromSecret
    When the action completes
    Then the output slot has Taint::DerivedFromSecret
```

### 6.3 ActionPolicyReport Generation

```gherkin
Feature: Action Policy Verification UI

  Scenario: Fully-covered workflow passes strict eligibility
    Given a workflow with all Do nodes registered
    And all contracts have idempotency=DeterministicPure
    And all contracts have timeout_ms > 0
    And all contracts have retry_safety != Unsafe
    When analyze_action_policies is called on the workflow
    Then all ActionPolicyReport have strict_eligible=true
    And all reports have issues=[]

  Scenario: Missing contract generates MissingTimeout and MissingIdempotency issues
    Given a workflow with one Do node referencing an unregistered action
    When analyze_action_policies is called
    Then the report for that Do has MissingTimeout issue
    And the report has MissingIdempotency issue
    And the report does not have UnsafeRetry issue
    And strict_eligible=false

  Scenario: Unsafe retry safety generates UnsafeRetry issue
    Given a workflow with a Do node using registered action with RetrySafety::Unsafe
    When analyze_action_policies is called
    Then the report has UnsafeRetry issue
    And strict_eligible=false

  Scenario: Duplicate Do nodes with same action produce single deduplicated report
    Given a workflow with two Do nodes referencing the same unregistered action
    When analyze_action_policies is called
    Then exactly one report is generated for that action
    And the report contains MissingTimeout and MissingIdempotency issues
```

---

## 7. Test Naming Convention

All tests follow the pattern:

```
test_<module>_<concept>_<scenario>
proptest_<module>_<concept>_<property_description>
bdd_<feature>_<scenario>
```

Examples:
- `test_registry_register_duplicate_on_occupied_slot_fails`
- `proptest_taint_propagation_never_downgrades`
- `bdd_action_dispatch_with_retry_attempt_2_succeeds`

---

## 8. Test Dependencies

| Module | External Dependencies | Mock Points |
|--------|---------------------|-------------|
| vb_core::action | None | None |
| vb_runtime::action | vb_core | ActionContract, Idempotency |
| vb_runtime::engine::action | vb_runtime::action | ActionRegistry |
| vb_ui::verify::action_policy | vb_core, vb_runtime | ActionRegistry |

---

## 9. Acceptance Criteria Mapping

| Acceptance Test | Unit Test(s) | Integration Test | BDD Scenario |
|-----------------|--------------|------------------|-------------|
| HA1 | test_execute_do_registered_deterministic_pure_with_clean_input | test_full_action_lifecycle | bdd_taint_propagation_clean_input_passes |
| HA2 | test_execute_do_at_least_once_with_secret_input_propagates_taint | test_taint_propagates_through_multiple | bdd_taint_propagation_secret_upgrades |
| HA3 | test_registry_dispatch_returns_suspended_ticket | test_full_action_lifecycle | bdd_action_dispatch_with_retry_retryable_recovery |
| HA4 | test_tracker_mark_completed_new_key_succeeds | test_full_action_lifecycle | bdd_action_dispatch_with_retry_completion |
| HA5 | test_resume_ready_writes_output_slot | test_full_action_lifecycle | bdd_action_dispatch_with_retry_completion |
| HA6 | test_resume_failed_retryable_below_capacity_returns_retry | test_retry_flow_within_capacity | bdd_action_dispatch_with_retry_retryable_recovery |
| HA7 | test_verify_idempotency_retry_safe_always_passes | — | — |
| HA8 | test_action_policy_report_strict_eligible_requires_all_conditions | test_analyze_policies_on_fully_covered_workflow | bdd_action_policy_fully_covered_workflow |
| HA9 | test_compute_idempotency_key_deterministic | proptest_compute_idempotency_key_deterministic_across_runs | — |
| HA10 | test_propagate_deterministic_pure_preserves_clean | test_clean_input_passes_through_pure_action | bdd_taint_propagation_clean_input_passes |
| EA1 | test_execute_do_deterministic_pure_with_secret_input_fails_taint | test_secret_input_blocks_pure_action | bdd_taint_propagation_secret_blocks_pure |
| EA2 | test_registry_dispatch_unknown_action_returns_error | — | — |
| EA3 | test_tracker_mark_completed_duplicate_key_fails | — | — |
| EA4 | test_verify_idempotency_unsafe_always_fails | — | — |
| EA5 | test_verify_idempotency_key_required_with_secret_slot_fails | — | — |
| EA6 | test_resume_failed_non_retryable_returns_error | test_retry_flow_within_capacity | bdd_action_dispatch_with_retry_non_retryable_fails |
| EA7 | test_resume_failed_at_capacity_returns_exhausted | test_retry_flow_within_capacity | bdd_action_dispatch_with_retry_exhaustion |
| EA8 | test_validate_action_dispatch_uninitialized_input_slot | — | — |
| EA9 | test_validate_action_dispatch_out_of_bounds_output_slot | — | — |
| EA10 | test_registry_dispatch_with_zero_max_bytes_and_nonzero_slots_fails | test_dispatch_queue_full_rejects | — |
| EA11 | test_registry_register_duplicate_on_occupied_slot_fails | — | — |
| EA12 | test_registry_register_action_id_at_max_u16_plus_one_fails | — | — |
| EA13 | test_resume_ready_output_out_of_bounds | — | — |
| EA14 | test_action_policy_report_missing_contract_has_timeout_issue | test_analyze_policies_reports_missing_contracts | bdd_action_policy_missing_contract |
| EA15 | test_action_policy_report_unsafe_retry_contract_has_unsafe_retry_issue | test_analyze_policies_reports_unsafe_retry | bdd_action_policy_unsafe_retry |
| EA16 | test_execute_do_deterministic_pure_with_derived_secret_fails_taint | test_taint_propagates_through_multiple_at_least_once_actions | bdd_taint_propagation_derived_blocks_pure |
