# TEST-PLAN.md — vb_runtime

## Crate: `vb_runtime`
## VERDICT: REJECTED — 6x bare assert!, 10x silent discards, 251 clippy violations

---

## Section 1 — Behavior Inventory

### resolve_contract (action.rs)
1. `resolve_contract returns Err(UnknownAction) when registry empty` — line 459
2. `resolve_contract returns Ok(contract) when index+id match` — line 471
3. `resolve_contract returns Err when index matches but id differs` — line 484
4. `resolve_contract returns first contract when id=0` — line 507 **[LETHAL: bare assert at line 510]**
5. `resolve_contract returns last contract when id=2` — line 514 **[LETHAL: bare assert at line 517]**

### execute_do (action.rs)
6. `execute_do returns Err(CapabilityDenied) when capability not granted` — line 540
7. `execute_do returns Ok(AwaitingAction) when capability is granted` — line 569 **[LETHAL: bare assert at line 590]**
8. `execute_do writes slot before capability check` — line 542 **[LETHAL: silent discard]**
9. `execute_do writes slot before action execution` — line 571 **[LETHAL: silent discard]**

### drive_deterministic_full (drive.rs)
10. `drive_deterministic_full collects evidence events` — line 822 **[LETHAL: silent discard at line 836]**
11. `drive_deterministic_full handles step budget exhaustion` — line 841
12. `drive_deterministic_full routes to body/retry path` — line 785

### execute_node_full (execute.rs)
13. `execute_node_full dispatches Do node without contract to AwaitingAction` — line 532 **[LETHAL: silent discard at line 557]**
14. `execute_node_full dispatches Do node with contract to AwaitingAction` — line 592 **[LETHAL: silent discard at line 606]**
15. `execute_node_full rejects taint-violating input on Do node` — line 712 **[LETHAL: silent discard at line 740]**
16. `execute_node_full routes RetryCheck Never-policy attempt 0 to body` — line 785
17. `execute_node_full routes RetryCheck Never-policy attempt 1 to exhausted` — line 830 **[LETHAL: silent discard at line 847]**
18. `execute_node_full routes RetryCheck DEFAULT-policy attempt 1 to body` — line 879 **[LETHAL: silent discard at line 895]**
19. `execute_node_full routes RetryCheck DEFAULT-policy attempt 3 to exhausted` — line 923 **[LETHAL: silent discard at line 939]**

---

## Section 2 — Trophy Allocation

| Layer | Tests | Rationale |
|-------|-------|-----------|
| **Static** (clippy) | 251 violations | Free compile-time checks; enforce zero warnings |
| **Unit** (#[cfg(test)]) | 19 behaviors above | Pure functions: `resolve_contract`, `execute_do`, `execute_retry_check`, `execute_error_handler`, `compute_idempotency_key` |
| **Integration** (/tests/) | 5 behaviors | Cross-component: `drive_deterministic_full`, `execute_node_full` with real `RunFrame`, `ValueStore`, `EvidenceCollector` |
| **E2E** | 0 | Runtime engine tested via integration + property tests |

Target: ~70% unit, ~25% integration, ~5% static.

---

## Section 3 — BDD Scenarios

### Behavior: resolve_contract returns first contract when id matches index
```
Given: a registry with one contract where contract.id == ActionId::new(0)
When:  resolve_contract(ActionId::new(0), &contracts) is called
Then:  result is Ok(&contract) where contract.id == ActionId::new(0)
```

**Test name:** `fn resolve_contract_returns_first_contract()`
**Current (LETHAL):**
```rust
let result = resolve_contract(ActionId::new(0), &contracts);
assert!(result.is_ok()); // LINE 510 — bare assert!
```
**Fix:** Replace with `assert_eq!(result.unwrap().id, ActionId::new(0));`

---

### Behavior: resolve_contract returns last contract when id matches index
```
Given: a registry with three contracts where contract[2].id == ActionId::new(2)
When:  resolve_contract(ActionId::new(2), &contracts) is called
Then:  result is Ok(&contract) where contract.id == ActionId::new(2)
```

**Test name:** `fn resolve_contract_returns_last_contract()`
**Current (LETHAL):**
```rust
let result = resolve_contract(ActionId::new(2), &contracts);
assert!(result.is_ok()); // LINE 517 — bare assert!
```
**Fix:** Replace with `assert_eq!(result.unwrap().id, ActionId::new(2));`

---

### Behavior: execute_do succeeds when required capability is granted
```
Given: a run frame with slot 0 containing I64(0), a contract requiring "secrets" capability, and granted capabilities containing "secrets"
When:  execute_do(...) is called
Then:  result is Ok(RuntimeSignal::AwaitingAction(ticket)) where ticket.action == ActionId::new(0)
```

**Test name:** `fn execute_do_succeeds_when_required_capability_is_granted()`
**Current (LETHAL):**
```rust
assert!(result.is_ok()); // LINE 590 — bare assert!
assert!(matches!(result.unwrap(), RuntimeSignal::AwaitingAction(_))); // LINE 591
```
**Fix:** Remove line 590 entirely; line 591 is the correct assertion.

---

### Behavior: execute_do writes slot before capability check (setup)
```
Given: a freshly created RunFrame
When:  run.write_slot(SlotIdx::new(0), SlotValue::I64(0)) is called
Then:  the slot write succeeds (Result is Ok)
```

**Test name:** `fn execute_do_returns_capability_denied_when_required_capability_not_granted()`
**Current (LETHAL):**
```rust
let _ = run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(0)); // LINE 542 — silent discard
```
**Fix:** Replace with `run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(0)).expect("write_slot must succeed in test");`

---

### Behavior: execute_do writes slot before action execution (setup)
```
Given: a freshly created RunFrame
When:  run.write_slot(SlotIdx::new(0), SlotValue::I64(0)) is called
Then:  the slot write succeeds (Result is Ok)
```

**Test name:** `fn execute_do_succeeds_when_required_capability_is_granted()`
**Current (LETHAL):**
```rust
let _ = run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(0)); // LINE 571 — silent discard
```
**Fix:** Replace with `run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(0)).expect("write_slot must succeed in test");`

---

### Behavior: drive_deterministic_full collects evidence for bonus_together
```
Given: a together workflow with 3 branches, a run frame, a budget of 10, and an empty EvidenceCollector
When:  dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty()) is called
Then:  evidence collector contains StepStarted and StepSucceeded events
```

**Test name:** `fn bonus_together()`
**Current (LETHAL):**
```rust
let _ = dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty()); // LINE 836 — silent discard
```
**Fix:** Replace with `dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty()).expect("drive_deterministic_full must succeed for bonus_together test");`

---

### Behavior: execute_node_full writes slot for Do node without contract
```
Given: a workflow with a Do node at step 0, a run frame, and an empty ValueStore
When:  execute_node_full is called with the Do node
Then:  the input slot is written with the provided value
```

**Test name:** `fn execute_do_without_contract_returns_awaiting_action()`
**Current (LETHAL):**
```rust
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(0)); // LINE 557 — silent discard
```
**Fix:** Replace with `run.write_slot(SlotIdx::new(0), SlotValue::I64(0)).expect("write_slot must succeed in test");`

---

### Behavior: execute_node_full writes slot for Do node with known contract
```
Given: a workflow with a Do node at step 0, a run frame pre-loaded with I64(10), and a contract registry
When:  execute_node_full is called with the Do node and contracts
Then:  the input slot is written with I64(10)
```

**Test name:** `fn execute_do_with_known_contract_returns_awaiting_action()`
**Current (LETHAL):**
```rust
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(10)); // LINE 606 — silent discard
```
**Fix:** Replace with `run.write_slot(SlotIdx::new(0), SlotValue::I64(10)).expect("write_slot must succeed in test");`

---

### Behavior: execute_node_full rejects taint-violating input on Do node
```
Given: a workflow with a Do node at step 0, a run frame, and an ActionContract with DeterministicPure idempotency
When:  execute_node_full is called with taint=Secret on input slot
Then:  result is Err(RuntimeEngineError::TaintViolation { step: StepIdx::ZERO })
```

**Test name:** `fn execute_do_with_tainted_input_rejects_taint_violation()`
**Current (LETHAL):**
```rust
let _ = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret); // LINE 740 — silent discard
```
**Fix:** Replace with `run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret).expect("write_slot_with_taint must succeed in test");`

---

### Behavior: execute_retry_check writes attempt value for Never policy route
```
Given: a workflow with RetryCheck node (policy_slot=0, body=0, exhausted=1), a run frame with slot 0 containing I64(1)
When:  execute_node_full is called with RetryCheck node and RetryPolicy::NEVER
Then:  PC is routed to exhausted step 1
```

**Test name:** `fn execute_retry_check_never_policy_attempt_one_routes_to_exhausted()`
**Current (LETHAL):**
```rust
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(1)); // LINE 847 — silent discard
```
**Fix:** Replace with `run.write_slot(SlotIdx::new(0), SlotValue::I64(1)).expect("write_slot must succeed in test");`

---

### Behavior: execute_retry_check writes attempt value for DEFAULT policy body route
```
Given: a workflow with RetryCheck node (policy_slot=0, body=0, exhausted=1), a run frame with slot 0 containing I64(1)
When:  execute_node_full is called with RetryCheck node and RetryPolicy::DEFAULT
Then:  PC is routed to body step 0
```

**Test name:** `fn execute_retry_check_default_policy_routes_to_body()`
**Current (LETHAL):**
```rust
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(1)); // LINE 895 — silent discard
```
**Fix:** Replace with `run.write_slot(SlotIdx::new(0), SlotValue::I64(1)).expect("write_slot must succeed in test");`

---

### Behavior: execute_retry_check writes attempt value for DEFAULT policy exhausted route
```
Given: a workflow with RetryCheck node (policy_slot=0, body=0, exhausted=1), a run frame with slot 0 containing I64(3)
When:  execute_node_full is called with RetryCheck node and RetryPolicy::DEFAULT
Then:  PC is routed to exhausted step 1
```

**Test name:** `fn execute_retry_check_default_policy_attempt_three_routes_to_exhausted()`
**Current (LETHAL):**
```rust
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(3)); // LINE 939 — silent discard
```
**Fix:** Replace with `run.write_slot(SlotIdx::new(0), SlotValue::I64(3)).expect("write_slot must succeed in test");`

---

## Section 4 — Proptest Invariants

### compute_idempotency_key (action.rs:217)
**Property:** Given any `(RunId, SeqNo, ActionId)` triple, the key is deterministic (same inputs → same output).
```rust
proptest! {
    #[test]
    fn idempotency_key_is_deterministic_for_all_inputs(
        run in any::<u16>(),
        seq in any::<u16>(),
        action in any::<u16>()
    ) {
        let key = compute_idempotency_key(RunId::new(run), SeqNo::new(seq), ActionId::new(action));
        let key2 = compute_idempotency_key(RunId::new(run), SeqNo::new(seq), ActionId::new(action));
        assert_eq!(key, key2);
    }
}
```

**Property:** Keys for different inputs must not collide (probability of collision must be negligible).
```rust
proptest! {
    #[test]
    fn idempotency_keys_differ_for_different_inputs(
        run1 in any::<u16>(),
        seq1 in any::<u16>(),
        action1 in any::<u16>(),
        run2 in any::<u16>(),
        seq2 in any::<u16>(),
        action2 in any::<u16>()
    ) {
        let key1 = compute_idempotency_key(RunId::new(run1), SeqNo::new(seq1), ActionId::new(action1));
        let key2 = compute_idempotency_key(RunId::new(run2), SeqNo::new(seq2), ActionId::new(action2));
        // Only assertneq if inputs actually differ
        if (run1, seq1, action1) != (run2, seq2, action2) {
            assert_ne!(key1, key2, "different inputs must produce different keys");
        }
    }
}
```

### resolve_contract (action.rs:230)
**Property:** When a contract exists at the action's index AND its id matches, the result is Ok.
**Input strategy:** Generate `(action_id: u16, contracts: Vec<ActionContract>)` where `contracts.len() > action_id` and `contracts[action_id].id == action_id`.
**Invalid input class:** Index is in bounds but `contracts[index].id != action` → must return Err.

### execute_retry_check (action.rs:127)
**Property:** Routes to body iff `attempt < policy.max_attempts`.
**Invariant:** `result == body if attempt < max_attempts else exhausted`.

---

## Section 5 — Fuzz Targets

### resolve_contract
**File:** N/A (pure function, not a parser)
**Reason:** No direct user-input parsing; contracts are built programmatically.

### compute_idempotency_key
**File:** N/A (deterministic arithmetic)
**Reason:** No uncontrolled input; all inputs are validated `u16` newtypes.

### execute_node_full
**File:** `fuzz/fuzz_targets/execute_node_fuzz.rs` (new)
**Input type:** `ExecuteNodeFuzzInput` — a struct containing `CompiledWorkflow`, `RunFrame`, `ValueStore`, `SlotIdx`, `SlotValue`, `RetryPolicy`
**Risk class:** HIGH — executes arbitrary workflow graphs with arbitrary slot values
**Corpus seeds:** Minimal; generate from existing test workflows in `engine/tests.rs`

---

## Section 6 — Kani Harnesses

### compute_idempotency_key (action.rs:217)
**Property to prove:** For any `(run, seq, action)` ∈ `(0..=u16::MAX)³`, `key1 == key2` where both calls use identical inputs.
**Bound:** All u16 → exhaustive across full range (65536³ = 2.8e14 → too large; use symbolic bounded model).
**Rationale:** Ensure FNV-1a-inspired hash produces zero collisions for distinct inputs.

### resolve_contract (action.rs:230)
**Property to prove:** `resolve_contract(action, contracts)` returns `Err(UnknownAction)` if `action.get() >= contracts.len()`.
**Bound:** `contracts.len()` up to 256 (realistic limit for action registry).
**Rationale:** Index-bound safety; prevent out-of-bounds access.

---

## Section 7 — Mutation Testing Checkpoints

| Behavior | Mutation Target | Kill Method |
|----------|---------------|-------------|
| `resolve_contract returns first/last contract` | Line 510/517: `assert!(result.is_ok())` | Mutate `is_ok()` → `is_err()` → assertion fails, test detects |
| `execute_do succeeds with capability granted` | Line 590: `assert!(result.is_ok())` | Remove capability grant → `is_err()` → test fails |
| `execute_do discards write_slot result` | Line 542/571: `let _ = run.write_slot(...)` | Inject slot write failure → test silently passes (MUTATION KILLS THIS) |
| `drive_deterministic_full discards dde result` | Line 836: `let _ = dde(...)` | Inject evidence collection failure → test silently passes (MUTATION KILLS THIS) |
| `execute_node_full discards write_slot results` | Lines 557,606,740,847,895,939 | Same as above — 6 mutations |

**Mutation kill rate target:** ≥90%
**Current coverage gap:** Silent discards (10 total) allow mutations to pass silently. Fixing each with `.expect()` enables mutation kill.

---

## Section 8 — Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| resolve_contract empty registry | `&[]` | `Err(UnknownAction)` | unit |
| resolve_contract index<len, id matches | `contracts[0].id == 0` | `Ok(&contracts[0])` | unit |
| resolve_contract index<len, id differs | `contracts[0].id != requested` | `Err(UnknownAction)` | unit |
| resolve_contract index>=len | `action.get() >= len` | `Err(UnknownAction)` | unit |
| execute_do capability denied | empty `CapabilitySet` | `Err(CapabilityDenied)` | unit |
| execute_do capability granted | matching `CapabilitySet` | `Ok(AwaitingAction(ticket))` | unit |
| execute_do taint violation | `Taint::Secret` on DeterministicPure | `Err(TaintViolation)` | unit |
| RetryCheck Never, attempt=0 | `I64(0) in policy_slot` | PC → body (step 0) | integration |
| RetryCheck Never, attempt=1 | `I64(1) in policy_slot` | PC → exhausted (step 1) | integration |
| RetryCheck DEFAULT, attempt=1 | `I64(1) in policy_slot` | PC → body (step 0) | integration |
| RetryCheck DEFAULT, attempt=3 | `I64(3) in policy_slot` | PC → exhausted (step 1) | integration |
| drive with zero budget | `StepBudget(0)` | `StepBudgetExhausted` | integration |
| drive with evidence collection | `EvidenceCollector::new()` | events populated | integration |

---

## Section 9 — Exact Lethal Fixes

### FIX 1: action.rs:510 — bare `assert!(result.is_ok())`
```rust
// BEFORE (LETHAL)
let result = resolve_contract(ActionId::new(0), &contracts);
assert!(result.is_ok());

// AFTER
let result = resolve_contract(ActionId::new(0), &contracts);
assert_eq!(
    result.expect("resolve_contract should succeed for id-in-range").id,
    ActionId::new(0),
    "resolved contract id must match requested action id"
);
```

### FIX 2: action.rs:517 — bare `assert!(result.is_ok())`
```rust
// BEFORE (LETHAL)
let result = resolve_contract(ActionId::new(2), &contracts);
assert!(result.is_ok());

// AFTER
let result = resolve_contract(ActionId::new(2), &contracts);
assert_eq!(
    result.expect("resolve_contract should succeed for id-in-range").id,
    ActionId::new(2),
    "resolved contract id must match requested action id"
);
```

### FIX 3: action.rs:590 — bare `assert!(result.is_ok())` + redundant assertion
```rust
// BEFORE (LETHAL)
assert!(result.is_ok());
assert!(matches!(result.unwrap(), RuntimeSignal::AwaitingAction(_)));

// AFTER
// Line 590 removed entirely; line 591 already provides stronger assertion
assert!(matches!(result.expect("execute_do should succeed with capability granted"), RuntimeSignal::AwaitingAction(_)));
```

### FIX 4: action.rs:542 — silent discard of `write_slot` result
```rust
// BEFORE (LETHAL)
let _ = run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(0));

// AFTER
run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(0))
    .expect("write_slot must succeed: slot is valid and uninitialized in test");
```

### FIX 5: action.rs:571 — silent discard of `write_slot` result
```rust
// BEFORE (LETHAL)
let _ = run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(0));

// AFTER
run.write_slot(SlotIdx::new(0), vb_core::value::SlotValue::I64(0))
    .expect("write_slot must succeed: slot is valid and uninitialized in test");
```

### FIX 6: drive.rs:836 — silent discard of `dde` result
```rust
// BEFORE (LETHAL)
let _ = dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty());

// AFTER
dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())
    .expect("drive_deterministic_full must succeed for bonus_together test");
```

### FIX 7: execute.rs:557 — silent discard of `write_slot` result
```rust
// BEFORE (LETHAL)
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(0));

// AFTER
run.write_slot(SlotIdx::new(0), SlotValue::I64(0))
    .expect("write_slot must succeed: slot is valid and uninitialized in test");
```

### FIX 8: execute.rs:606 — silent discard of `write_slot` result
```rust
// BEFORE (LETHAL)
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(10));

// AFTER
run.write_slot(SlotIdx::new(0), SlotValue::I64(10))
    .expect("write_slot must succeed: slot is valid and uninitialized in test");
```

### FIX 9: execute.rs:740 — silent discard of `write_slot_with_taint` result
```rust
// BEFORE (LETHAL)
let _ = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);

// AFTER
run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret)
    .expect("write_slot_with_taint must succeed: slot is valid and uninitialized in test");
```

### FIX 10: execute.rs:847 — silent discard of `write_slot` result
```rust
// BEFORE (LETHAL)
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(1));

// AFTER
run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
    .expect("write_slot must succeed: slot is valid and uninitialized in test");
```

### FIX 11: execute.rs:895 — silent discard of `write_slot` result
```rust
// BEFORE (LETHAL)
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(1));

// AFTER
run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
    .expect("write_slot must succeed: slot is valid and uninitialized in test");
```

### FIX 12: execute.rs:939 — silent discard of `write_slot` result
```rust
// BEFORE (LETHAL)
let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(3));

// AFTER
run.write_slot(SlotIdx::new(0), SlotValue::I64(3))
    .expect("write_slot must succeed: slot is valid and uninitialized in test");
```

---

## Section 10 — Clippy Violations (251 errors)

All 251 clippy violations must be fixed before re-submission. The following categories are expected:

1. **unused_mut** — Multiple `mut` variables on `run` in test functions (e.g., `tests.rs:665,683,709,741,1333,2165`)
2. **unused_variables** — Unused `run` variables prefixed with underscore (e.g., `_run`)
3. **result_expect** / **expect_fun_call** — Use `.expect()` instead of `.unwrap()` where appropriate
4. **clone_on_copy** — Possibly unnecessary clones
5. **redundant_clone** — Redundant clones on known owned values
6. **suspicious_clone** — Suspicious clone patterns

A full `cargo clippy -- -W clippy::all` run is needed to enumerate all 251 violations. Each must be addressed individually.

---

## Exit Criteria

- [ ] All 12 lethal fixes applied (Sections 9.1–9.12)
- [ ] All 6 bare `assert!(is_ok())` assertions replaced with specific value checks
- [ ] All 10 silent `let _ =` discards replaced with `.expect()` calls
- [ ] All 251 clippy violations addressed
- [ ] Mutation kill rate ≥90% confirmed after fixes
- [ ] `cargo test -p vb_runtime` passes with 0 failures
- [ ] `cargo clippy -p vb_runtime` passes with 0 errors, 0 warnings

---

*Generated by: test-planner agent*
*Date: 2026-05-10*
