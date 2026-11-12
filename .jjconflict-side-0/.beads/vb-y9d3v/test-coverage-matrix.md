# Test Coverage Matrix — vb-y9d3v ActionTicket Generation Fence

bead_id: vb-y9d3v
plan_state: 8
coverage_date: 2026-05-30
plan_invocation_id: vb-y9d3v-state8-test-planner-attempt1

## Contract-to-Test Traceability

Every normative contract clause from `contract.md` maps to one or more behavior tests, proof obligations, and verifier lanes. The matrix below shows complete traceability for all 22 contract clauses (ACT-001 through VER-002).

### ActionTicket Authority Clauses

| Contract ID | Behavior IDs | Unit Tests | Integration Tests | E2E Tests | Proptest | Kani | Verus | Flux | Fuzz |
|---|---|---|---|---|---|---|---|---|---|
| ACT-001 | B-001, B-008, B-009, B-010, B-011, B-043, B-044, B-045 | — | `validate_action_completion_rejects_when_step_not_running`, `validate_action_completion_rejects_when_node_missing`, `validate_action_completion_rejects_when_node_not_do_or_action_mismatch` | `handle_action_completion_returns_run_not_found_when_*` | `e2e_exact_attempt_completion_succeeds` | `prop_validate_ticket_attempt_classifies_all_attempt_relations` | PO-0001, PO-0017 | PO-0002, PO-0018 | PO-0003, PO-0019 | — |
| ACT-002 | B-008, B-009, B-010, B-011 | — | `validate_action_completion_*` (3 tests) | — | — | — | PO-0025-0028 | PO-0025-0028 | PO-0025-0028 | — |
| ACT-003 | B-004, B-005, B-006, B-017 | `validate_ticket_attempt_rejects_when_attempt_is_zero`, `validate_ticket_attempt_rejects_when_capacity_is_zero`, `validate_ticket_attempt_rejects_when_attempt_exceeds_capacity` | `record_scheduled_attempt_*` | — | `prop_validate_ticket_attempt_classifies_all_attempt_relations` | PO-0001, PO-0033-0036 | PO-0002, PO-0034 | PO-0003, PO-0035 | — |
| ACT-004 | B-035, B-036 | `reject_invalid_ticket_key_*` (2 tests) | `preflight_action_completion_*` | — | `prop_idempotency_key_is_deterministic` | PO-0013 | PO-0014 | — | — |
| ACT-005 | B-002, B-003, B-007 | `validate_ticket_attempt_returns_stale_attempt_when_attempt_lower_than_current`, `validate_ticket_attempt_rejects_future_attempt_when_attempt_exceeds_current` | — | `e2e_stale_attempt_after_retry_is_rejected`, `e2e_future_attempt_without_scheduling_is_rejected` | `prop_validate_ticket_attempt_classifies_all_attempt_relations` | PO-0005-0008 | PO-0005-0008 | PO-0005-0008 | PO-0005-0008 | — |
| ACT-006 | B-003 | `validate_ticket_attempt_rejects_future_attempt_when_attempt_exceeds_current` | — | `e2e_future_attempt_without_scheduling_is_rejected` | `prop_validate_ticket_attempt_classifies_all_attempt_relations` | PO-0005-0008 | PO-0005-0008 | PO-0005-0008 | — | — |
| ACT-007 | B-058, B-059, B-060, B-061 | — | `stale_attempt_completion_does_not_mutate_*` (4 tests) | — | `prop_invalid_authority_never_mutates_state` | PO-0013-0016 | PO-0013-0016 | — | PO-0013-0016 | — |
| ACT-008 | B-037, B-038, B-039, B-040, B-041, B-042 | — | `preflight_action_completion_*` (6 tests) | — | — | PO-0013-0016 | PO-0013-0016 | — | — |
| ACT-009 | B-018 through B-034, B-050 | `validate_retry_attempt_*` (4 tests) | `record_retry_attempt_*` (5 tests), `retry_policy_after_action_*` (6 tests), `retry_metadata_exists_*` (2 tests) | `e2e_valid_failure_with_retry_advances_to_next_attempt` | `prop_record_retry_attempt_increments_by_exactly_one_or_not_at_all` | PO-0009-0012, PO-0029-0032 | PO-0009-0012, PO-0029-0032 | PO-0009-0012, PO-0029-0032 | PO-0009-0012, PO-0029-0032 | PO-0041 |
| ACT-010 | B-025, B-026 | — | `record_retry_attempt_rejects_on_overflow_with_checked_add`, `record_retry_attempt_rejects_when_step_out_of_bounds` | — | `prop_record_retry_attempt_never_panics` | PO-0009 | PO-0010 | PO-0011 | — |
| ACT-011 | B-022, B-023, B-024 | — | `record_retry_attempt_*` (5 tests) | — | `prop_record_retry_attempt_increments_by_exactly_one_or_not_at_all` | PO-0009-0012 | PO-0010 | PO-0011 | — |
| ACT-012 | B-043, B-044, B-045, B-046 | — | `handle_action_completion_returns_run_not_found_when_*` (3 tests), `finish_run_*` (1 test) | `e2e_*` | `prop_invalid_authority_never_mutates_state` | PO-0017-0020 | PO-0017-0020 | PO-0017-0020 | PO-0017-0020 | — |

### Timer Authority Clauses

| Contract ID | Behavior IDs | Unit Tests | Integration Tests | E2E Tests | Proptest | Kani | Verus | Flux | Fuzz |
|---|---|---|---|---|---|---|---|---|---|
| TMR-001 | B-055, B-056 | — | `timer_wheel_fire_expired_fires_fresh_entry_when_generation_matches`, `timer_wheel_fire_expired_ignores_stale_entry_when_generation_mismatch` | `e2e_timer_replacement_invalidates_old_generation` | `prop_timer_insert_increments_generation_monotonically` | — | PO-0015 | PO-0015 | PO-0015 | — |
| TMR-002 | B-052 | — | `timer_wheel_insert_increments_generation_when_replacing_existing_entry`, `timer_wheel_insert_returns_generation_exhausted_on_overflow` | — | `prop_timer_insert_increments_generation_monotonically` | — | PO-0016 | PO-0016 | PO-0016 | — |
| TMR-003 | B-053, B-054, B-057 | — | `timer_wheel_cancel_*` (2 tests), `timer_wheel_fire_expired_ignores_stale_entry_after_cancel` | — | `prop_timer_cancel_is_idempotent` | — | PO-0015 | — | — |

### Verification Meta-Clauses

| Contract ID | Behavior IDs | Coverage |
|---|---|---|
| VER-001 | All Part A behaviors | All proof artifacts bind to production fresh-main functions via `proof-to-rust-map.md` bridge. No hardcoded Kani shapes, no detached Verus/Flux models. |
| VER-002 | All Part A behaviors | Prior vb-8mdp.5 evidence excluded from all evidence commands and proof claims. All references use fresh-main production paths. |

## Test Count Summary

| Layer | Planned Tests | Existing Tests (Part B) | TOTAL |
|---|---|---|---|
| Unit | 16 | 0 | 16 |
| Integration | 28 | 0 | 28 |
| E2E | 5 | 0 | 5 |
| Proptest | 6 | 2 (Part B: prop1, prop2) | 8 |
| **Subtotal Part A (new)** | **55** | **0** | **55** |
| Part B (existing) | — | 15 | 15 |
| **GRAND TOTAL** | **55** | **17** | **72** |

## Error Variant Coverage

Every variant in `RuntimeError` used in the ActionTicket fence path is covered by at least one explicit test:

| RuntimeError Variant | Triggering Scenario | Test Function |
|---|---|---|
| `RunNotFound` | Missing run on completion/failure/timer | `handle_action_completion_returns_run_not_found_when_run_missing` |
| `StaleAttempt { incoming, current }` | Lower attempt | `validate_ticket_attempt_returns_stale_attempt_when_attempt_lower_than_current` |
| `AttemptBeyondMax { attempt, max }` | Zero attempt, zero capacity, over capacity | `validate_ticket_attempt_rejects_when_attempt_is_zero`, etc. |
| `InvalidActionCompletion` | Wrong step state, missing node, wrong action, noncanonical key, output mismatch, missing step | Multiple integration tests |
| `UnsupportedOperation { operation }` | Missing retry metadata, unreadable policy slot, non-I64 slot, out-of-range max_attempts, zero max_attempts, overflow | Multiple retry tests |
| `InvalidTimerFire` | Timer from wrong generation/cancelled | `timer_wheel_fire_expired_ignores_stale_entry_after_cancel` (via fire_expired non-removal) |
| `EncodeFailed` | Postcard encode failure | `preflight_action_completion_*` (indirect) |
| `ActionTaintDowngrade { required, supplied }` | Taint downgrade in completion payload | `preflight_action_completion_rejects_taint_downgrade` |
| `ActionOutputLengthMismatch { declared, actual }` | Encoded length mismatch | `preflight_action_completion_rejects_encoded_len_mismatch` |
| `ActionOutputTooLarge { size, max }` | Contract output byte limit exceeded | `preflight_action_completion_rejects_contract_output_too_large` |
| `ActionOutputBlobTooLarge { size, max }` | Resource blob limit exceeded | `preflight_action_completion_rejects_resource_output_too_large` |
| `TimerWheelError::GenerationExhausted` | Generation overflow on timer replacement | `timer_wheel_insert_returns_generation_exhausted_on_overflow` |

## Coverage Gaps (Deferred to State 11)

| Gap ID | Description | Resolution State | Compensating Coverage |
|---|---|---|---|
| G001 | Verus tautological specs — no production binding | State 11 | Behavior tests + proptest + Kani (planned) cover all Rust invariants |
| G002 | Kani vacuous harnesses — `cover!` instead of `assert` | State 11 | Unit + integration tests exercise same invariants with concrete assertions |
| G003 | Flux false invariant on ActionTicket | State 11 | Proptest properties cover valid/invalid attempt ranges |
| G004 | GOD RULE 1 — hardcoded workflow shapes | State 11 | Hostile public ActionTicket inputs in behavior tests |
| G005 | Future-attempt rejection not yet implemented | State 11 | Test plan includes future-attempt test with documented expected behavior for both pre-fix and post-fix states |
| G006 | BLOCKED_TOOLING for Verus + Flux | State 12 | Behavior tests + proptest + Kani provide compensating behavioral evidence |
| G007 | Private function visibility for Kani harnesses | State 11 | Tests exercise private functions through public API (`validate_action_completion`, `preflight_action_completion`) |

## Boundary Coverage for Hostile Inputs

| Hostile Input Class | Coverage |
|---|---|
| `attempt == 0` | Unit test `validate_ticket_attempt_rejects_when_attempt_is_zero`, proptest |
| `capacity == 0` | Unit test `validate_ticket_attempt_rejects_when_capacity_is_zero`, proptest |
| `attempt > capacity` | Unit test `validate_ticket_attempt_rejects_when_attempt_exceeds_capacity`, proptest |
| `attempt < current` (stale) | Unit test `validate_ticket_attempt_returns_stale_attempt_*`, 2 integration tests, e2e |
| `attempt > current` (future) | Unit test `validate_ticket_attempt_rejects_future_attempt_*`, integration, e2e |
| `current == 0` (no authority) | Unit test with current=0 edge case |
| Noncanonical idempotency key | Unit test `reject_invalid_ticket_key_rejects_*`, integration, non-mutation check |
| Step index out of bounds | Unit test `validate_ticket_attempt_rejects_when_step_out_of_bounds` |
| u16::MAX boundary values | Boundary tests at max valid values for attempt, capacity, current |
| Stale timer generation (cancelled/replaced) | Integration tests `timer_wheel_fire_expired_ignores_stale_entry_*` |
| Timer generation overflow | Integration test `timer_wheel_insert_returns_generation_exhausted_on_overflow` |
| Retry attempt overflow (u16::MAX) | Integration test `record_retry_attempt_rejects_on_overflow_with_checked_add` |

## Evidence Map for State 12 Execution

The test-writer in State 9 will produce executable tests. The formal verifier in State 12 will execute:

```bash
# Unit + Integration (Part A)
cargo test -p vb_runtime -- validate_ticket_attempt validate_action_completion normalize_scheduled_ticket record_retry_attempt retry_policy_after_action retry_metadata_exists -- --nocapture
cargo test -p vb_runtime -- preflight_action_completion reject_invalid_ticket_key handle_action_completion handle_action_failure -- --nocapture
cargo test -p vb_runtime -- timer_wheel_insert timer_wheel_cancel timer_wheel_fire_expired -- --nocapture
cargo test -p vb_runtime -- stale_attempt_completion_does_not_mutate future_attempt_completion_does_not_mutate -- --nocapture

# Proptest
cargo test -p vb_runtime -- prop_validate_ticket_attempt prop_idempotency_key prop_record_retry_attempt prop_timer prop_invalid_authority -- --nocapture

# E2E
cargo test -p workspace_tests -- e2e_exact e2e_stale e2e_future e2e_timer_replacement e2e_valid_failure -- --nocapture

# Existing Part B confirmation (must all pass)
cargo test -p vb_proof_kernels test_invalid_transitions test_terminal_immutable -- --nocapture
cargo test -p vb_core -- state_transition_cancelled_terminal_rejects_pending frame_mark_succeeded_on_pending_step_allows_overwrite -- --nocapture
cargo test -p vb_runtime jump_to_body vb_y4pa_001 vb_y4pa_002 vb_y4pa_003 vb_y4pa_004 vb_y4pa_005 vb_y4pa_006 gwt_re1 -- --nocapture

# Fuzz (smoke)
cargo fuzz run fuzz_retry_codec -- -max_len=64 -runs=1000
```

## Mutation Kill Rate Tracking

| Metric | Target | Measurement Method |
|---|---|---|
| Line coverage | >= 85% | `cargo llvm-cov --html` |
| Branch coverage | >= 80% | Same report |
| Mutation kill rate | >= 90% | `cargo mutants --list-files crates/vb_runtime/src/shard/helpers.rs crates/vb_runtime/src/shard/lifecycle/chunk_003.rs crates/vb_runtime/src/shard/timer_wheel.rs` |
| Test determinism | 100% non-flaky | Repeat runs (3x) on same seed must produce identical results |

## Handoff to State 9 (Test Writer)

The test-writer must:

1. Write all 55 Part A behavior tests as specified in `test-plan.md` Section 3 (BDD Scenarios) and Section 8 (Combinatorial Coverage Matrix).
2. Write all 6 Part A proptest properties as specified in `test-plan.md` Section 4.
3. Ensure every assertion specifies the exact error variant and payload values — reject `is_ok()`/`is_err()` without payload inspection.
4. Verify all 15 Part B existing tests pass (run the Part B verification commands).
5. Produce `test-evidence.md` with raw `cargo test` output showing all tests pass.
6. Produce `coverage-report.md` with `cargo llvm-cov` output showing >= 85% line coverage on the fence paths.
7. Do NOT modify production code — if a test fails, report it as a regression to State 11, do not fix implementation.
8. If the future-attempt test fails (expected, due to G005 gap), document the failure in `test-evidence.md` with the test name, actual output, and expected output. Tag as `G005-expected-failure` so it does not block the test pass threshold.
