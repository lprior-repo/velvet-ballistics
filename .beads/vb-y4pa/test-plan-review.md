# Test Plan: vb-y4pa for_each/repeat/reduce/collect body re-entry fix

## BEAD: vb-y4pa
## STATE: 7 test-planner
## FIX: `jump_to_body` resets body step Succeeded→Pending before re-entry

---

## 1. Bug Summary

**Root Cause**: When a loop body step completes (`Succeeded`) and the primitive calls `jump_to(run, body)` to re-enter for the next iteration, the engine scheduler rejects `Succeeded → Running` because this transition was absent from `VALID_TRANSITIONS`.

**Fix Applied**:
- `jump_to_body(run, body)` helper: calls `run.mark_pending(body)?` then `jump_to(run, body)`
- `VALID_TRANSITIONS` extended: `(StepState::Succeeded, StepState::Pending)` added
- All 6 primitives now use `jump_to_body` instead of `jump_to`

**Affected Primitives** (6 total):

| Primitive | Function | Line |
|-----------|----------|------|
| `for_each` | `for_each_next` | `for_each.rs:84` |
| `reduce` | `reduce_next` | `reduce.rs:82` |
| `collect` | `collect_page` | `collect.rs:397` |
| `collect` | `collect_next` | `collect.rs:521` |
| `repeat` | `repeat_attempt` | `repeat.rs:88` |
| `repeat` | `repeat_check` | `repeat.rs:115` |

---

## 2. Unit Tests (in `reentry_tests.rs`)

### 2.1 Existing Tests (already approved in proof-review.md attempt 2)

| Test ID | Test Name | Primitive | Coverage |
|---------|-----------|-----------|----------|
| `vb_y4pa_001` | `for_each_two_item_reentry` | for_each_next | 2-item list: body executes item1 (Succeeded), re-entry for item2 |
| `vb_y4pa_002` | `reduce_reentry` | reduce_next | 2-item list: first body run Succeeded, re-entry for second item |
| `vb_y4pa_003` | `collect_next_reentry` | collect_next | 4 items, page_size=2: page1 body Succeeded, re-entry for page2 |
| `vb_y4pa_004` | `collect_page_reentry` | collect_page | page body Succeeded, re-entry |
| `vb_y4pa_005` | `repeat_attempt_reentry` | repeat_attempt | body attempt 1 Succeeded, re-entry for attempt 2 |
| `vb_y4pa_006` | `repeat_check_reentry` | repeat_check | attempt 1 Succeeded, check loops back to body_entry |

### 2.2 Missing Unit Tests (gap analysis from proof-review.md)

#### TC-001: `jump_to_body_succeeded_to_pending`
**File**: `helpers.rs` tests
- Given: body step in `Succeeded` state
- When: `jump_to_body(run, body)` called
- Then: body transitions to `Pending`, PC set, executed incremented

#### TC-002: `jump_to_body_pending_unchanged`
**File**: `helpers.rs` tests
- Given: body step in `Pending` state
- When: `jump_to_body(run, body)` called
- Then: body stays `Pending` (no error), PC set, executed incremented

#### TC-003: `jump_to_body_waiting_unchanged`
**File**: `helpers.rs` tests
- Given: body step in `Waiting` state
- When: `jump_to_body(run, body)` called
- Then: body stays `Waiting` (no error), PC set, executed incremented

#### TC-004: `jump_to_body_asking_unchanged`
**File**: `helpers.rs` tests
- Given: body step in `Asking` state
- When: `jump_to_body(run, body)` called
- Then: body stays `Asking` (no error), PC set, executed incremented

#### TC-005: `for_each_three_item_reentry`
**File**: `reentry_tests.rs`
- Given: 3-item list `[A, B, C]`, for_each_start binds A, body runs A → Succeeded
- When: for_each_next binds B → body Succeeded; for_each_next binds C
- Then: body processes C correctly with PC at body step

#### TC-006: `for_each_empty_list_does_not_reenter`
**File**: `reentry_tests.rs`
- Given: for_each_start with empty list
- When: body step is still Pending (never executed)
- Then: for_each_next jumps directly to done (no re-entry attempt)

#### TC-007: `reduce_three_item_accumulator`
**File**: `reentry_tests.rs`
- Given: 3-item list `[1, 2, 3]`, initial accumulator 0
- When: reduce body runs three times (1, 2, 3 each Succeeded)
- Then: final accumulator is sum 6

#### TC-008: `reduce_body_succeeded_resets_on_reentry`
**File**: `reentry_tests.rs`
- Given: body step is Succeeded from previous iteration
- When: `mark_pending(body)` called then `jump_to_body`
- Then: body is Pending, PC at body, no invalid_state_transition error

#### TC-009: `collect_four_page_reentry`
**File**: `reentry_tests.rs`
- Given: 8 items, page_size=2 (4 pages total)
- When: body runs for pages 1, 2, 3 (each Succeeded), re-entry for page 4
- Then: page 4 processes correctly

#### TC-010: `collect_page_body_succeeded_resets`
**File**: `reentry_tests.rs`
- Given: body step Succeeded after first page
- When: collect_page re-entry via `jump_to_body`
- Then: body step is Pending, no error

#### TC-011: `repeat_max_attempts_exhausted`
**File**: `reentry_tests.rs`
- Given: repeat with max_attempts=3
- When: 3 attempts complete (body Succeeded each time), repeat_check sees current=3 ≥ max
- Then: repeat_check jumps to done (no body re-entry needed)

#### TC-012: `repeat_body_state_resets_on_each_attempt`
**File**: `reentry_tests.rs`
- Given: repeat_attempt called 3 times sequentially
- When: each call sets body Succeeded → Pending via `jump_to_body`
- Then: each body re-entry succeeds without invalid_state_transition

#### TC-013: `for_each_next_jumps_to_done_when_iterator_empty`
**File**: `reentry_tests.rs`
- Given: for_each_next called with empty iterator (but body step is Succeeded from prior run on different list)
- When: iterator is empty
- Then: jumps to done, body step not re-entered

#### TC-014: `reduce_next_jumps_to_done_when_remaining_empty`
**File**: `reentry_tests.rs`
- Given: reduce_next called with empty remaining list
- When: remaining is empty
- Then: jumps to done, body step not re-entered

---

## 3. BDD Given/When/Then Scenarios

### GWT-RE-1: for_each body re-entry after Succeeded
```
Given:
  - A for_each primitive over [Item1, Item2]
  - for_each_start ran: body_step is Pending, iterator = [Item1, Item2]
  - Body executed Item1: body_step is Succeeded

When:
  - Engine calls for_each_next(iterator_slot=[Item2], body=body_step, done, output)

Then:
  - for_each_next calls jump_to_body(run, body_step)
  - body_step transitions Succeeded → Pending (jump_to_body calls mark_pending first)
  - body_step is set to Running by engine scheduler
  - Body executes Item2
  - for_each_next returns EngineSignal::Continue
```

### GWT-RE-2: reduce body re-entry after Succeeded
```
Given:
  - A reduce primitive over [A, B, C] with initial accumulator
  - reduce_start bound A, body ran A → Succeeded
  - reduce_next bound B, body ran B → Succeeded

When:
  - reduce_next(iterator_slot=[C], body=body_step, done, output)

Then:
  - reduce_next calls jump_to_body(run, body_step)
  - body_step transitions Succeeded → Pending
  - body runs C → Succeeded
  - reduce_next returns EngineSignal::Continue
```

### GWT-RE-3: collect_page re-entry after page body Succeeded
```
Given:
  - A collect primitive with page_size=2 over [A,B,C,D]
  - collect_page ran body for page [A,B] → body_step Succeeded

When:
  - Engine calls collect_page(collector_slot, body=body_step, done)

Then:
  - collect_page calls jump_to_body(run, body_step)
  - body_step transitions Succeeded → Pending
  - body processes next page [C,D]
```

### GWT-RE-4: repeat_attempt re-entry after attempt Succeeded
```
Given:
  - A repeat primitive with max_attempts=3
  - repeat_start initialized: attempt=0, body_step Pending
  - Body attempt 1 ran → body_step Succeeded
  - repeat_check incremented attempt to 1 (still < max)

When:
  - repeat_attempt(attempt_slot, body=body_step, done)

Then:
  - repeat_attempt calls jump_to_body(run, body_step)
  - body_step transitions Succeeded → Pending
  - Body attempt 2 runs
```

### GWT-RE-5: repeat_check loops back to body after attempt Succeeded
```
Given:
  - A repeat primitive with max_attempts=3
  - Body attempt 2 ran → body_step Succeeded
  - repeat_check sees current_attempt=2, max=3

When:
  - repeat_check(attempt_slot, done, next=body_entry, step)

Then:
  - repeat_check calls jump_to_body(run, body_entry)
  - body_entry transitions Succeeded → Pending
  - repeat_check returns EngineSignal::Continue
  - PC is body_entry
```

### GWT-RE-6: Succeeded→Running transition rejected by state machine (negative)
```
Given:
  - A body step in Succeeded state
  - Plain jump_to (not jump_to_body) is called

When:
  - Engine scheduler attempts Succeeded → Running transition

Then:
  - validate_transition returns Err("invalid_state_transition")
  - This is why jump_to_body must call mark_pending first
```

---

## 4. Kani Proof Harnesses (already written in `reentry_proofs.rs`)

### Harness Coverage Matrix

| Harness | PO | Arbitrary State | Cover Succeeded | Strong Assert |
|---------|----|----------------|-----------------|---------------|
| `for_each_next_reentry` | PO-004 | `kani::any::<StepState>()` | `kani::cover!(body_state == StepState::Succeeded)` | `kani::assert(state.is_ok())` |
| `reduce_next_reentry` | PO-005 | `kani::any::<StepState>()` | `kani::cover!(body_state == StepState::Succeeded)` | `kani::assert(state.is_ok())` |
| `collect_next_reentry` | PO-006 | `kani::any::<StepState>()` | `kani::cover!(body_state == StepState::Succeeded)` | `kani::assert(state.is_ok())` |
| `collect_page_reentry` | PO-007 | `kani::any::<StepState>()` | `kani::cover!(body_state == StepState::Succeeded)` | `kani::assert(state.is_ok())` |
| `repeat_attempt_reentry` | PO-008 | `kani::any::<StepState>()` | `kani::cover!(body_state == StepState::Succeeded)` | `kani::assert(state.is_ok())` |
| `repeat_check_reentry` | PO-009 | `kani::any::<StepState>()` | `kani::cover!(body_state == StepState::Succeeded)` | `kani::assert(state.is_ok())` |

**Note**: Proof-review.md (attempt 2) APPROVED all 6 harnesses. All use `kani::any::<StepState>()`, have `kani::cover` statements, and assert `state.is_ok()`.

---

## 5. Proptest / Fuzz Targets

### PROP-1: `jump_to_body_state_transitions`
- **Strategy**: Arbitrary StepState → verify Succeeded→Pending succeeds, others unchanged
- **Property**: `jump_to_body` never returns error; body step always Pending after call
- **Invariants tested**: `mark_pending` succeeds for Succeeded; `jump_to` always succeeds

### PROP-2: `for_each_n_items_all_reentry`
- **Strategy**: `vec[SlotValue]` of length 1-20, random item types
- **Property**: All items processed without panic; PC ends at done
- **Invariant**: Body step transitions Succeeded→Pending on each re-entry

### PROP-3: `reduce_accumulation_reentry`
- **Strategy**: List of 1-50 integers; arbitrary initial accumulator
- **Property**: Final accumulator equals fold of list with initial value
- **Invariant**: Each body re-entry succeeds (Succeeded→Pending)

### PROP-4: `collect_pagination_reentry`
- **Strategy**: List of 1-100 items, page_size 1-10
- **Property**: All items collected in correct order
- **Invariant**: Each page body re-entry via `jump_to_body` succeeds

### PROP-5: `repeat_attempt_reentry`
- **Strategy**: max_attempts 1-10, verify all attempts run
- **Property**: repeat runs exactly max_attempts times
- **Invariant**: Body step Succeeded→Pending on each `repeat_attempt` call

---

## 6. Regression Tests

### REG-1: `jump_to_body_must_be_used_not_jump_to`
Verify all 6 primitives call `jump_to_body` (grep-based structural test):
- `for_each_next` → `jump_to_body` at for_each.rs:84
- `reduce_next` → `jump_to_body` at reduce.rs:82
- `collect_page` → `jump_to_body` at collect.rs:397
- `collect_next` → `jump_to_body` at collect.rs:521
- `repeat_attempt` → `jump_to_body` at repeat.rs:88
- `repeat_check` → `jump_to_body` at repeat.rs:115

### REG-2: `Succeeded_Pending_in_valid_transitions`
Verify `step_state.rs` contains `(StepState::Succeeded, StepState::Pending)` in `VALID_TRANSITIONS`.

### REG-3: `Succeeded_Running_not_in_valid_transitions`
Regression: verify `(Succeeded, Running)` is NOT in `VALID_TRANSITIONS` (the bug path must stay blocked).

---

## 7. Test Execution Order

```
Phase A: helpers unit tests (TC-001 to TC-004)
  → run: cargo test -p vb_runtime -- helpers::tests

Phase B: reentry unit tests (existing vb_y4pa_001 to vb_y4pa_006 + new TC-005 to TC-014)
  → run: cargo test -p vb_runtime -- reentry_tests

Phase C: BDD integration scenarios (end-to-end workflow tests)
  → run: cargo test -p workspace_tests -- vb_y4pa

Phase D: Kani harnesses (formal verification)
  → run: cargo kani -p vb_runtime -- --function for_each_next_reentry
  → run: cargo kani -p vb_runtime -- --function reduce_next_reentry
  → run: cargo kani -p vb_runtime -- --function collect_next_reentry
  → run: cargo kani -p vb_runtime -- --function collect_page_reentry
  → run: cargo kani -p vb_runtime -- --function repeat_attempt_reentry
  → run: cargo kani -p vb_runtime -- --function repeat_check_reentry

Phase E: Proptest
  → run: cargo test -p vb_runtime --jump_to_body_state_transitions
  → run: cargo test -p vb_runtime --for_each_n_items_all_reentry
  → run: cargo test -p vb_runtime --reduce_accumulation_reentry
  → run: cargo test -p vb_runtime --collect_pagination_reentry
  → run: cargo test -p vb_runtime --repeat_attempt_reentry
```

---

## 8. Files Under Test

| File | Lines | Purpose |
|------|-------|---------|
| `crates/vb_runtime/src/primitives/helpers.rs` | 60-66 | `jump_to_body` implementation |
| `crates/vb_runtime/src/primitives/for_each.rs` | 59-85 | `for_each_next` using `jump_to_body` |
| `crates/vb_runtime/src/primitives/reduce.rs` | 56-83 | `reduce_next` using `jump_to_body` |
| `crates/vb_runtime/src/primitives/collect.rs` | 388-398, 496-522 | `collect_page` + `collect_next` using `jump_to_body` |
| `crates/vb_runtime/src/primitives/repeat.rs` | 78-89, 94-117 | `repeat_attempt` + `repeat_check` using `jump_to_body` |
| `crates/vb_proof_kernels/src/step_state.rs` | 48 | `(Succeeded, Pending)` in `VALID_TRANSITIONS` |
| `crates/vb_runtime/src/primitives/reentry_tests.rs` | 1-317 | Existing 6 unit tests |
| `crates/vb_runtime/src/primitives/reentry_proofs.rs` | 1-451 | 6 Kani harnesses |

---

## 9. Pass Criteria

- [ ] TC-001 to TC-014 all pass
- [ ] All `vb_y4pa_00X_*` tests pass
- [ ] BDD scenarios GWT-RE-1 to GWT-RE-6 implemented and passing
- [ ] All 6 Kani harnesses: `cargo kani` reports no failures
- [ ] Proptest PROP-1 to PROP-5: 1000 iterations each, no failures
- [ ] REG-1: structural grep finds `jump_to_body` at all 6 call sites
- [ ] REG-2: `(Succeeded, Pending)` in `VALID_TRANSITIONS`
- [ ] REG-3: `(Succeeded, Running)` NOT in `VALID_TRANSITIONS`
