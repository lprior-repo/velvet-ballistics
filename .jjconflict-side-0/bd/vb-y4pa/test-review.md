# Test Review: vb-y4pa for_each/repeat/reduce/collect body re-entry fix

## BEAD: vb-y4pa
## STATE: 9 test-reviewer
## DATE: 2026-05-19

---

## Summary

**STATUS: APPROVED**

All 14 unit tests (TC-001 to TC-014) + 6 BDD Given/When/Then scenarios pass.
The test suite provides comprehensive coverage of the `jump_to_body` re-entry fix
for `for_each`, `repeat`, `reduce`, and `collect` primitives.

---

## Test Execution Evidence

### Phase A: helpers unit tests (jump_to_body coverage)
```
cargo test -p vb_runtime -- helpers::tests
Result: 102 passed, 1549 filtered out
```
All `jump_to_body` state transition tests pass:
- `tc001_jump_to_body_succeeded_to_pending`: Succeeded→Pending via `jump_to_body` ✓
- `tc002_jump_to_body_pending_idempotent`: Pending→Pending (idempotent) ✓
- `tc003_jump_to_body_succeeded_also_idempotent`: Succeeded→Pending (confirms fix) ✓
- `tc004_jump_to_body_waiting_is_invalid`: Waiting→error (correct rejection) ✓
- `tc005_jump_to_body_asking_is_invalid`: Asking→error (correct rejection) ✓

### Phase B: reentry unit tests (existing + new TC-005 to TC-014)
```
cargo test -p vb_runtime -- reentry_tests
Result: 22 passed, 1629 filtered out
```

#### Existing 6 tests (vb_y4pa_001 to vb_y4pa_006) ✓
| Test | Primitive | Coverage |
|------|-----------|----------|
| vb_y4pa_001 | for_each_next | 2-item list: body executes item1, re-entry for item2 |
| vb_y4pa_002 | reduce_next | 2-item list: first body Succeeded, re-entry for second |
| vb_y4pa_003 | collect_next | 4 items, page_size=2: page1 Succeeded, re-entry for page2 |
| vb_y4pa_004 | collect_page | page body Succeeded, re-entry |
| vb_y4pa_005 | repeat_attempt | body attempt 1 Succeeded, re-entry for attempt 2 |
| vb_y4pa_006 | repeat_check | attempt 1 Succeeded, check loops back to body_entry |

#### New 10 tests (TC-005 to TC-014) ✓
| Test | Primitive | Coverage |
|------|-----------|----------|
| TC-005 | for_each_three_item_reentry | 3-item list: all 3 iterations re-enter correctly |
| TC-006 | for_each_empty_list_does_not_reenter | Empty list: jumps directly to done |
| TC-007 | reduce_three_item_accumulator | 3-item accumulator fold, all iterations |
| TC-008 | reduce_body_succeeded_resets_on_reentry | Succeeded body resets to Pending |
| TC-009 | collect_four_page_reentry | 8 items, page_size=2: 4 pages, all re-enter |
| TC-010 | collect_page_body_succeeded_resets | Body step Pending after re-entry |
| TC-011 | repeat_max_attempts_exhausted | 3 attempts → repeat_check routes to done |
| TC-012 | repeat_body_state_resets_on_each_attempt | 3 sequential re-entries all succeed |
| TC-013 | for_each_next_jumps_to_done_when_iterator_empty | Empty iterator takes precedence |
| TC-014 | reduce_next_jumps_to_done_when_remaining_empty | Empty remaining → done |

### Phase C: BDD Given/When/Then Scenarios ✓
All 6 GWT scenarios pass:

| Scenario | Description | Status |
|----------|-------------|--------|
| GWT-RE-1 | for_each body re-entry after Succeeded | ✓ Continue, PC=body, body=Pending |
| GWT-RE-2 | reduce body re-entry after Succeeded | ✓ Continue, PC=body, body=Pending |
| GWT-RE-3 | collect_page re-entry after page body Succeeded | ✓ Continue, body=Pending |
| GWT-RE-4 | repeat_attempt re-entry after attempt Succeeded | ✓ Continue, PC=body, body=Pending |
| GWT-RE-5 | repeat_check loops back after attempt Succeeded | ✓ Continue, PC=next_body |
| GWT-RE-6 | Succeeded→Running rejected by state machine | ✓ Transition invalid (negative test) |

### Phase D: Full vb_runtime test suite ✓
```
cargo test -p vb_runtime
Result: 1651 passed (15 suites, 1.63s)
```

---

## Verification Checklist

### Tests use real workflows ✓
- All tests use `fresh_frame()` creating real `RunFrame` objects
- Tests use `ValueStore::new()` for real value storage
- `minimal_workflow()` / `minimal_workflow_with_const()` create real `CompiledWorkflow`

### Assertions check exact states ✓
- `run.step_state(body)` checked for `Pending` after re-entry
- `run.pc()` verified to equal `body` step after `jump_to_body`
- `run.executed()` counter incremented
- Slot values verified (e.g., `item_slot` has correct `SlotValue::I64`)

### Deterministic ✓
- No randomness, no time-based behavior
- Each test sets up exact initial state and verifies exact final state
- `fresh_frame(6, 10)` always creates identical frames

### No mocks ✓
- No `mock_*` functions or types
- All state transitions use real `RunFrame` methods (`mark_running`, `mark_succeeded`, `mark_pending`)
- All jump functions use real `jump_to_body` implementation

### Contract parity with fix ✓
- `jump_to_body` calls `mark_pending(body)?` then `jump_to(run, body)`
- 6 primitive call sites confirmed via grep:
  - `for_each.rs:84` → `jump_to_body(run, body)`
  - `reduce.rs:82` → `jump_to_body(run, body)`
  - `collect.rs:397` → `jump_to_body(run, body)`
  - `collect.rs:521` → `jump_to_body(run, body)`
  - `repeat.rs:88` → `jump_to_body(run, body)`
  - `repeat.rs:115` → `jump_to_body(run, body)`

### Succeeded→Pending in VALID_TRANSITIONS ✓
- `is_valid_step_state_transition(Succeeded, Pending)` returns true
- `is_valid_step_state_transition(Succeeded, Running)` returns false (negative case)

---

## Test Quality Assessment

### Strengths
1. **Exhaustive re-entry coverage**: Tests cover 2-item, 3-item, and 4-page scenarios
2. **Boundary conditions**: Empty lists, exhausted attempts, empty iterators
3. **State verification**: Body step state explicitly checked after each re-entry
4. **Negative tests**: GWT-RE-6 confirms invalid transition is rejected
5. **Deterministic**: Noflakiness, reproducible results

### Minor Gaps (non-blocking)
1. TC-004 (Waiting→Pending via jump_to_body) is implicitly tested via error path but not explicitly named in TC series
2. TC-005 tests 3 items but doesn't verify intermediate PC values between iterations

### Risk Assessment: LOW
- Tests directly exercise the fixed code path (`jump_to_body` → `mark_pending` → `jump_to`)
- Multiple independent assertions per test
- Full suite passes with 1651 tests total

---

## Conclusion

The test suite is **APPROVED**. All 14 unit tests and 6 BDD scenarios provide
comprehensive, deterministic, mock-free verification of the `jump_to_body`
re-entry fix for for_each/repeat/reduce/collect primitives.

**RECOMMENDATION: Proceed to implementation/stabilization.**

---

## Files Under Review

| File | Lines | Purpose |
|------|-------|---------|
| `crates/vb_runtime/src/primitives/helpers.rs` | 60-66, 420-502 | `jump_to_body` impl + TC-001-005 |
| `crates/vb_runtime/src/primitives/reentry_tests.rs` | 1-1151 | All re-entry unit + BDD tests |
| `bd/vb-y4pa/test-plan.md` | 1-353 | Test plan (reference) |
