# TEST-PLAN.md — vb_ui crate

## Crate Summary
- **Package:** `vb_ui`
- **Location:** `/home/lewis/src/Velvet-ballistics/crates/vb_ui`
- **Current State:** REJECTED — 46 tautological `assert!(false, ...)` tests, 1 clippy error, coverage below threshold
- **Target:** ≥5x test density, handlers.rs 44%→70%, dispatch.rs 23%→50%, client.rs 48%→70%

---

## Section 1 — Behavior Inventory

### 1.1 Certificate Verification (`verify/certificates.rs` / `certificates_tests.rs`)

| Behavior | File:Line |
|----------|-----------|
| `VerificationResult::analyze` returns StructuralValidity certificate | `certificates_tests.rs:64` |
| `VerificationResult::analyze` returns 8 total certificates | `certificates_tests.rs:185` |
| `verify_workflow` returns 8 pre-flight checks | `certificates_tests.rs:847` |
| Empty nodes workflow fails structural validity | `certificates_tests.rs:257` |
| Entry out-of-bounds fails structural validity | `certificates_tests.rs:273` |
| Node ID mismatch fails structural validity | `certificates_tests.rs:291` |
| Zero max_steps fails bounded transitions check | `certificates_tests.rs:338` |
| Zero max_slots fails bounded transitions check | `certificates_tests.rs:356` |
| Node count exceeding max_steps fails boundedness | `certificates_tests.rs:374` |
| Clean workflow passes secret-to-result-leak check | `certificates_tests.rs:414` |
| Secret reaching Finish fails secret-to-result-leak | `certificates_tests.rs:430` |
| Workflow with error handler passes strict durability | `certificates_tests.rs:483` |
| Do without error handler warns strict durability | `certificates_tests.rs:542` |
| No Do nodes passes action idempotency | `certificates_tests.rs:595` |
| Ungarded actions warn action idempotency | `certificates_tests.rs:611` |
| Small slot count passes memory budget | `certificates_tests.rs:664` |
| Zero slots passes memory budget | `certificates_tests.rs:680` |
| Exceeding output limit warns memory budget | `certificates_tests.rs:698` |
| Within max_transitions passes | `certificates_tests.rs:721` |
| Exceeding max_transitions fails | `certificates_tests.rs:734` |
| Within max_action_calls passes | `certificates_tests.rs:778` |
| Exceeding max_action_calls ceiling warns | `certificates_tests.rs:791` |
| `CheckStatus::merge_worst` Fail dominates all | `certificates_tests.rs:877` |
| `CheckStatus::merge_worst` Warn dominates Pass | `certificates_tests.rs:897` |
| `CheckStatus::merge_worst` Pass+Pass = Pass | `certificates_tests.rs:909` |
| `collect_successors` for Jump node | `certificates_tests.rs:1401` |
| `collect_successors` for TogetherStart node | `certificates_tests.rs:1413` |
| `collect_successors` includes on_error target | `certificates_tests.rs:1428` |

### 1.2 Resource Bounds (`verify/resources.rs`)

| Behavior | File:Line |
|----------|-----------|
| `ResourceBoundsPanel::all_within_bounds` returns false when exceeded | `resources.rs:512` |
| `ResourceBoundsPanel` node_count at limit reports AtLimit | `resources.rs:529` |
| `ResourceBoundsPanel` exceeds limit reports ExceedsLimit | `resources.rs:558` |
| Panel metrics count equals 8 | `resources.rs:598` |
| `classify(5, 10)` returns WithinBounds | `resources.rs:616` |
| `classify(10, 10)` returns AtLimit | `resources.rs:621` |
| `classify(11, 10)` returns ExceedsLimit | `resources.rs:626` |
| `classify(0, 0)` returns AtLimit | `resources.rs:631` |
| Default contract produces passing panel | `resources.rs:638` |
| Tight contract produces failing panel | `resources.rs:662` |

### 1.3 Replay State (`replay/state.rs`)

| Behavior | File:Line |
|----------|-----------|
| `apply_event(StepStarted)` inserts Running state | `state.rs:563` |
| `apply_event(StepStarted)` does not increment completed | `state.rs:575` |
| `apply_event(StepSucceeded)` inserts Succeeded state | `state.rs:589` |
| `apply_event(StepSucceeded)` increments completed counter | `state.rs:600` |
| `apply_event(StepSucceeded)` records output slot | `state.rs:607` |
| `apply_event(ActionScheduled)` increments dispatched | `state.rs:622` |
| Multiple `ActionScheduled` accumulate | `state.rs:630` |
| `apply_event(ActionCompleted)` increments completed | `state.rs:643` |
| `apply_event(ActionFailed)` increments failed | `state.rs:654` |
| `apply_event(SlotWritten)` records slot | `state.rs:665` |
| `apply_event(SlotWritten)` does not overwrite existing | `state.rs:676` |
| `apply_event(WaitScheduled)` sets Waiting state | `state.rs:693` |
| `apply_event(AskScheduled)` sets Asking state | `state.rs:708` |
| `apply_event(AskAnswered)` transitions to Running | `state.rs:723` |
| `apply_event(RetryScheduled)` is informational | `state.rs:739` |
| `apply_event(RunCancelled)` sets terminal | `state.rs:757` |
| `apply_event(RunCompleted)` sets terminal | `state.rs:757` |
| `apply_event(RunAccepted)` does not set terminal | `state.rs:1291` |
| SlotWritten on fresh slot uses default value | `state.rs:1256` |
| Step state transitions Running→Waiting | `state.rs:1271` |

### 1.4 Ticket Panel (`replay/ticket_panel.rs`)

| Behavior | File:Line |
|----------|-----------|
| `worst_side_effect_certainty` empty list returns None | `ticket_panel.rs:687` |
| `worst_side_effect_certainty` all None returns None | `ticket_panel.rs:590` |
| `worst_side_effect_certainty` elevated by Unknown | `ticket_panel.rs:604` |
| `worst_side_effect_certainty` elevated to Certain | `ticket_panel.rs:639` |
| Empty ticket list is replay safe | `ticket_panel.rs:678` |
| Multiple actions same idempotency key are distinct tickets | `ticket_panel.rs:700` |
| First ticket is safe, later are duplicate completions | `ticket_panel.rs:726` |

### 1.5 Layout (`layout.rs`)

| Behavior | File:Line |
|----------|-----------|
| Multiple nodes with edges get finite positions | `layout.rs:805` |
| Single node no edges trivial layout | `layout.rs:828` |
| Group bounds can produce negative origin | `layout.rs:852` |

### 1.6 Incident Types (`incident/types.rs`)

| Behavior | File:Line |
|----------|-----------|
| `IncidentContext` construction with all fields | `types.rs:507` |
| `TimelineEntry` construction | `types.rs:524` |
| `FailureDetail` construction | `types.rs:540` |
| `IncidentRecord` construction | `types.rs:577` |
| `Incident` construction | `types.rs:599` |
| Incident record minimal fields | `types.rs:634` |

---

## Section 2 — Trophy Allocation

| Layer | Target % | Rationale |
|-------|----------|-----------|
| **Integration** (`/tests/`) | 60% | End-to-end verify_workflow, replay state machine, layout computation |
| **Unit** (`#[cfg(test)]`) | 30% | Pure functions: classify(), collect_successors(), worst_side_effect_certainty() |
| **E2E** | 5% | Full workflow verification + UI render smoke tests |
| **Static** (clippy/deny) | 5% | `cargo clippy -D clippy::perf` + `cargo deny` supply chain |

### Coverage Targets
- `handlers.rs`: 44% → **70%** (add integration tests for event handler branches)
- `dispatch.rs`: 23% → **50%** (add unit tests for dispatch table)
- `client.rs`: 48% → **70%** (add integration tests for client state transitions)

---

## Section 3 — Tautological Test Fixes

### Fix Pattern
Every `assert!(false, "...")` in a `let Some(x) = expr else { ... }` block is unreachable IF the preceding `assert!(x.is_some())` passes. The correct idioms are:

**Option A (preferred):** Remove redundant `assert!(is_some())` + `let-else`, use `expect()`:
```rust
// BEFORE (tautological):
assert!(result.is_some());
let Some(x) = result else {
    assert!(false, "should not happen");
    return;
};

// AFTER (correct):
let x = result.expect("result must be Some because precondition X");
```

**Option B:** If None is actually possible, use `if let Some(x) = ...`:
```rust
// BEFORE:
assert!(result.is_some());
let Some(x) = result else {
    assert!(false, "metric missing");
    return;
};
assert_eq!(x.status, ResourceStatus::AtLimit);

// AFTER:
if let Some(x) = result {
    assert_eq!(x.status, ResourceStatus::AtLimit);
} else {
    // This is a valid failure case — test it properly
    panic!("expected metric to be present");
}
```

---

### 3.1 `verify/certificates_tests.rs` — 27 fixes

| Line | Current Code | Fix Required |
|------|-------------|--------------|
| 71 | `assert!(false, "structural cert missing")` | Remove `assert!(structural.is_some()); let Some(cert) = structural else { assert!(false...); return; };` → `let cert = structural.expect("StructuralValidity certificate must exist for non-empty workflow");` |
| 91 | `assert!(false, "cert missing")` | Same pattern for structural certificate |
| 103 | `assert!(false, "cert missing")` | Same pattern for durability certificate |
| 124 | `assert!(false, "cert missing")` | Same pattern for reachability certificate |
| 174 | `assert!(false, "cert missing")` | Same pattern for reachability certificate |
| 249 | `assert!(false, "check missing")` | For preflight structural_validity check |
| 265 | `assert!(false, "check missing")` | For preflight structural_validity check |
| 283 | `assert!(false, "check missing")` | For preflight structural_validity check |
| 313 | `assert!(false, "check missing")` | For preflight structural_validity check |
| 331 | `assert!(false, "check missing")` | For bounded_transitions check |
| 348 | `assert!(false, "check missing")` | For bounded_transitions check |
| 366 | `assert!(false, "check missing")` | For bounded_transitions check |
| 404 | `assert!(false, "check missing")` | For bounded_transitions check |
| 422 | `assert!(false, "check missing")` | For secret_to_result_leak check |
| 473 | `assert!(false, "check missing")` | For secret_to_result_leak check |
| 534 | `assert!(false, "check missing")` | For strict_durability_eligibility check |
| 585 | `assert!(false, "check missing")` | For strict_durability_eligibility check |
| 603 | `assert!(false, "check missing")` | For action_idempotency check |
| 654 | `assert!(false, "check missing")` | For action_idempotency check |
| 672 | `assert!(false, "check missing")` | For worst_case_memory_budget check |
| 690 | `assert!(false, "check missing")` | For worst_case_memory_budget check |
| 711 | `assert!(false, "check missing")` | For worst_case_memory_budget check |
| 726 | `assert!(false, "check missing")` | For max_transitions check |
| 764 | `assert!(false, "check missing")` | For max_transitions check |
| 783 | `assert!(false, "check missing")` | For max_action_calls check |
| 836 | `assert!(false, "check missing")` | For max_action_calls check |

### 3.2 `verify/resources.rs` — 3 fixes

| Line | Current Code | Fix Required |
|------|-------------|--------------|
| 551 | `assert!(false, "metric missing")` | `let m = node_metric.expect("node_count / max_steps metric must exist in panel");` |
| 580 | `assert!(false, "metric missing")` | `let node_metric = node_metric.expect("node_count / max_steps metric must exist");` |
| 591 | `assert!(false, "metric missing")` | `let slot_metric = slot_metric.expect("slot_count / max_slots metric must exist");` |

### 3.3 `replay/ticket_panel.rs` — 4 fixes

| Line | Current Code | Fix Required |
|------|-------------|--------------|
| 597 | `assert!(false, "should return Some")` | `let worst = worst.expect("worst_side_effect_certainty on non-empty list must return Some");` |
| 632 | `assert!(false, "should return Some")` | Same pattern |
| 667 | `assert!(false, "should return Some")` | Same pattern |
| 728 | `assert!(false, "must have first ticket")` | `let first = tickets.first().expect("tickets vec has 3 elements");` |

### 3.4 `replay/state.rs` — 9 fixes

| Line | Current Code | Fix Required |
|------|-------------|--------------|
| 567 | `assert!(false, "step 0 must be present in step_states")` | `let s = next.step_states.get(&StepIdx::new(0)).expect("step 0 must be present after StepStarted");` |
| 593 | `assert!(false, "step 3 must be present in step_states")` | `let s = next.step_states.get(&StepIdx::new(3)).expect("step 3 must be present after StepSucceeded");` |
| 611 | `assert!(false, "output slot 7 must be recorded")` | `let v = next.slot_values.get(&SlotIdx::new(7)).expect("slot 7 must be recorded after StepSucceeded");` |
| 669 | `assert!(false, "slot 12 must be recorded")` | `let v = next.slot_values.get(&SlotIdx::new(12)).expect("slot 12 must be recorded after SlotWritten");` |
| 682 | `assert!(false, "slot 5 must be present")` | `let v = next.slot_values.get(&SlotIdx::new(5)).expect("slot 5 must be present (not overwritten)");` |
| 697 | `assert!(false, "step 2 must be present in step_states")` | `let s = next.step_states.get(&StepIdx::new(2)).expect("step 2 must be present after WaitScheduled");` |
| 712 | `assert!(false, "step 4 must be present in step_states")` | `let s = next.step_states.get(&StepIdx::new(4)).expect("step 4 must be present after AskScheduled");` |
| 728 | `assert!(false, "step 1 must be present in step_states")` | `let s = answered.step_states.get(&StepIdx::new(1)).expect("step 1 must be present after AskAnswered");` |
| 1260 | `assert!(false, "slot 3 must be present")` | `let v = next.slot_values.get(&SlotIdx::new(3)).expect("slot 3 must be present after SlotWritten");` |

### 3.5 `layout.rs` — 1 fix

| Line | Current Code | Fix Required |
|------|-------------|--------------|
| 814 | `assert!(false, "missing position for {id}")` | `let pos = result.positions.get(*id).expect("all input nodes must have a position (precondition: every node in graph has position entry)");` |

### 3.6 `incident/types.rs` — 3 fixes

| Line | Current Code | Fix Required |
|------|-------------|--------------|
| 517 | `assert!(false, "idempotency key must be Some")` | `let k = ctx.last_action_idempotency_key.expect("idempotency key was set to Some in test construction");` |
| 567 | `assert!(false, "step_id must be Some")` | `let sid = detail.step_id.expect("step_id was set to Some(5) in test construction");` |
| 627 | `assert!(false, "step_name must be Some")` | `let sn = incident.step_name.expect("step_name was set to Some in test construction");` |

---

## Section 4 — BDD Scenarios

### 4.1 Certificate Verification

```gherkin
Feature: Workflow Certificate Verification

  Scenario: Minimal valid workflow passes all certificates
    Given a workflow with a single Finish node
    When VerificationResult::analyze is called
    Then it returns 8 certificates
    And StructuralValidity status is Pass
    And Reachability status is Pass
    And StrictDurability status is Warn (no error handler)

  Scenario: Empty nodes workflow fails structural validity
    Given a workflow with zero nodes
    When VerificationResult::analyze is called
    Then StructuralValidity certificate has Fail status

  Scenario: Unreachable node fails reachability check
    Given a workflow where node 1 is not reachable from entry
    When verify_workflow is called
    Then Reachability check has Fail status

  Scenario: Secret reaching Finish fails leak check
    Given a workflow with WaitEvent feeding into Finish
    When verify_workflow is called
    Then secret_to_result_leak check has Fail status

  Scenario: Zero max_steps fails boundedness
    Given a workflow with default contract but max_steps=0
    When verify_workflow is called
    Then bounded_transitions check has Fail status
    And detail contains "max_steps"
```

### 4.2 Resource Bounds

```gherkin
Feature: Resource Bounds Computation

  Scenario: All bounds within contract limits
    Given ResourceBounds where all values < contract limits
    When ResourceBoundsPanel::new is called
    Then all_within_bounds returns true
    And every metric status is WithinBounds or AtLimit

  Scenario: Node count at limit
    Given node_count equals max_steps
    When ResourceBoundsPanel::new is called
    Then node_count / max_steps metric status is AtLimit

  Scenario: Node count exceeds limit
    Given node_count > max_steps
    When ResourceBoundsPanel::new is called
    Then node_count / max_steps metric status is ExceedsLimit
    And all_within_bounds returns false
```

### 4.3 Replay State Machine

```gherkin
Feature: Replay State Event Application

  Scenario: StepStarted inserts Running state
    Given initial ReplayState
    When apply_event is called with StepStarted(0, seq=2)
    Then step_states[0] equals Running
    And at_seq equals 2
    And steps_completed is 0

  Scenario: StepSucceeded transitions to Succeeded and increments counter
    Given initial ReplayState
    When apply_event is called with StepSucceeded(3, seq=10, output_slot=5)
    Then step_states[3] equals Succeeded
    And slot_values[5] equals "<written>"
    And steps_completed equals 1

  Scenario: SlotWritten does not overwrite existing value
    Given a ReplayState with slot_values[5] = "custom"
    When apply_event is called with SlotWritten(5, seq=1)
    Then slot_values[5] remains "custom"

  Scenario: AskAnswered transitions from Asking to Running
    Given a ReplayState with step 1 in Asking state
    When apply_event is called with AskAnswered(1, seq=2)
    Then step_states[1] equals Running
```

---

## Section 5 — Proptest Invariants

### 5.1 `verify/resources.rs::classify`

```rust
// Invariant: classify(value, limit) must be consistent
proptest! {
    #[test]
    fn classify_invariant(value: u32, limit: u32) {
        let result = classify(value, limit);
        match result {
            ResourceStatus::WithinBounds => {
                prop_assert!(value < limit);
            }
            ResourceStatus::AtLimit => {
                prop_assert!(value == limit);
            }
            ResourceStatus::ExceedsLimit => {
                prop_assert!(value > limit);
            }
        }
    }

    // Boundary: zero limit with zero value is AtLimit
    #[test]
    fn classify_zero_limit_zero_value_is_at_limit(value: u32) {
        // This is a separate invariant about (0, 0) → AtLimit
        assert_eq!(classify(0, 0), ResourceStatus::AtLimit);
    }
}
```

### 5.2 `verify/certificates.rs::collect_successors`

```rust
// Invariant: collect_successors never returns empty for nodes with edges
proptest! {
    #[test]
    fn successors_never_empty_for_nodes_with_exits(
        kind: CompiledNodeKind,
        next: Option<StepIdx>,
        on_error: Option<StepIdx>
    ) {
        // Skip terminal nodes
        let has_next = !matches!(kind, CompiledNodeKind::Finish { .. });
        let succs = collect_successors(&kind, next, on_error);
        if has_next || next.is_some() || on_error.is_some() {
            prop_assert!(!succs.is_empty());
        }
    }
}
```

### 5.3 `replay/state.rs::apply_event`

```rust
// Invariant: apply_event never panics for valid events
proptest! {
    #[test]
    fn apply_event_never_panics(event: WorkflowEvent, state: ReplayState) {
        // Should not panic - use std::panic::catch_unwind
        let result = std::panic::catch_unwind(|| state.apply_event(&event));
        prop_assert!(result.is_ok());
    }
}
```

---

## Section 6 — Fuzz Targets

### 6.1 `verify/certificates.rs::VerificationResult::analyze`

**Input:** Arbitrary `WorkflowParts`
**Risk:** High (workflow verification is security-critical)
**Corpus seeds:** Minimal workflow, workflow with 100 nodes, empty workflow

```rust
// fuzz/fuzz_targets/workflow_analysis.rs
#![no_main]]
use libfuzzer_sys::fuzz_target;
use vb_ui::verify::certificates::VerificationResult;
use vb_core::workflow::WorkflowParts;

fuzz_target!(|parts: WorkflowParts| {
    let result = VerificationResult::analyze(&parts);
    // Invariant: pass_count + fail_count + warn_count == total_checks
    assert_eq!(
        result.pass_count + result.fail_count + result.warn_count,
        result.total_checks
    );
    // Invariant: certificates.len() == total_checks
    assert_eq!(result.certificates.len(), result.total_checks);
});
```

### 6.2 `verify/resources.rs::ResourceBounds::compute`

**Input:** Arbitrary `WorkflowParts`
**Risk:** Medium (resource estimation)
**Corpus seeds:** Empty workflow, workflow with all node kinds

### 6.3 `replay/state.rs::apply_event`

**Input:** Arbitrary sequence of `WorkflowEvent`
**Risk:** Medium (state machine correctness)
**Corpus seeds:** Single event, 3-event sequence, 10-event sequence

---

## Section 7 — Kani Harnesses

### 7.1 `verify/resources.rs::classify` bounds proof

```rust
// kani/proofs/classify_bounds.rs
#![prove(standard)]

fn classify_invariant(value: u32, limit: u32) {
    let result = classify(value, limit);
    match result {
        ResourceStatus::WithinBounds => {
            assert!(value <= limit);  // Note: within means <= for our impl
        }
        ResourceStatus::AtLimit => {
            assert!(value == limit);
        }
        ResourceStatus::ExceedsLimit => {
            assert!(value > limit);
        }
    }
}
```

### 7.2 `replay/state.rs::step_state_transitions` state machine

Prove that invalid transitions are impossible (e.g., Succeeded → Running is not a valid transition).

---

## Section 8 — Mutation Testing Checkpoints

**Target kill rate:** ≥90%

| Module | Mutation Operator | Test that Catches It |
|--------|-----------------|---------------------|
| `verify/resources.rs` | Change `classify(5, 10)` to return `AtLimit` | `test_classify_within` fails |
| `verify/resources.rs` | Change `<` to `<=` in WithinBounds check | `test_classify_exceeds` fails |
| `replay/state.rs` | Change `Running` to `Waiting` in StepStarted | `apply_step_started_inserts_running_state` fails |
| `replay/state.rs` | Skip `steps_completed += 1` in StepSucceeded | `apply_step_succeeded_increments_completed_counter` fails |
| `replay/ticket_panel.rs` | Change `None` certainty comparison | `worst_certainty_elevated_by_unknown` fails |
| `layout.rs` | Return empty positions map | `blackhat_group_bounds_can_produce_negative_origin` fails |

---

## Section 9 — Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| verify_workflow empty | empty nodes | Fail StructuralValidity | unit |
| verify_workflow valid | single Finish | All checks Pass/Warn | integration |
| verify_workflow leak | WaitEvent→Finish | Fail secret_to_result_leak | unit |
| verify_workflow durability | Do+error_handler | Pass strict_durability | unit |
| classify within | value < limit | WithinBounds | unit |
| classify at | value == limit | AtLimit | unit |
| classify exceeds | value > limit | ExceedsLimit | unit |
| apply_event StepStarted | fresh state | Running + seq update | unit |
| apply_event StepSucceeded | Running state | Succeeded + slot + counter | unit |
| apply_event SlotWritten | fresh slot | slot = "<written>" | unit |
| apply_event SlotWritten | existing slot | no overwrite | unit |
| apply_event AskAnswered | Asking state | Running | unit |
| worst_certainty empty | [] | None | unit |
| worst_certainty all None | [None, None] | None | unit |
| worst_certainty with Unknown | [None, Unknown] | Unknown | unit |
| worst_certainty with Certain | [None, Certain] | Certain | unit |
| layout 4 nodes | graph with edges | 4 finite positions | integration |
| layout single node | single node | MARGIN_LEFT, MARGIN_TOP | unit |

---

## Section 10 — Clippy Fixes

### 10.1 `verify/action_policy.rs:115`

```rust
// BEFORE:
if !reports.contains_key(&action) {
    let report = Self::for_action(action, contract);
    reports.insert(action, report);
}

// AFTER:
reports.entry(action).or_insert_with(|| Self::for_action(action, contract));
```

---

## Section 11 — Test Density Calculation

**Current:** 2770 tests passing, density 1.81x
**Target:** ≥5x density

**To achieve 5x:**
- Current LOC: ~X (need to measure)
- Current tests: 2770
- Target tests: X × 5

**Required additions:**
1. Fix 46 tautological tests (restore proper assertions)
2. Add 20+ new property-based tests (proptest invariants)
3. Add 10+ new integration tests for handlers/dispatch/client
4. Add 5+ fuzz targets for parsers and state machine

---

## Section 12 — Exit Criteria

Before this crate is approved:

- [ ] All 46 `assert!(false, ...)` replaced with proper `expect()` or `if-let` handling
- [ ] `cargo clippy -D clippy::perf` passes (0 errors)
- [ ] Coverage: handlers.rs ≥70%, dispatch.rs ≥50%, client.rs ≥70%
- [ ] Test density ≥5x
- [ ] All 2770 existing tests still pass
- [ ] Mutation kill rate ≥90%
