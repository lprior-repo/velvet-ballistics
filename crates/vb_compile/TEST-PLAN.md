# TEST-PLAN.md — vb_compile

## 1. Behavior Inventory

### 1.1 Core Compilation Pipeline

| Subject | Action | Outcome when | Condition |
|--------|--------|--------------|-----------|
| `YamlCompiler::compile` | accepts valid YAML | `Ok(CompiledWorkflow)` | valid v1 workflow |
| `YamlCompiler::compile` | rejects non-UTF-8 | `Err(CompileErrors([CompileError::Utf8(...)]))` | invalid UTF-8 |
| `YamlCompiler::compile` | rejects empty source | `Err(CompileErrors([CompileError::EmptySource]))` | blank/whitespace-only |
| `YamlCompiler::compile` | rejects oversized source | `Err(CompileErrors([CompileError::SourceTooLarge {...}]))` | exceeds max_source_bytes |
| `YamlCompiler::compile` | rejects multi-document YAML | `Err(CompileErrors([CompileError::DocumentCount {...}]))` | >1 document |
| `YamlCompiler::compile` | rejects non-mapping root | `Err(CompileErrors([CompileError::TopLevelNotMapping]))` | root is sequence/scalar |
| `YamlCompiler::compile` | rejects duplicate mapping keys | `Err(CompileErrors([CompileError::DuplicateKey {...}]))` | duplicate key found |
| `YamlCompiler::compile` | rejects YAML aliases/anchors | `Err(CompileErrors([CompileError::AliasForbidden {...}]))` | anchor/alias present |
| `YamlCompiler::compile` | rejects YAML tags | `Err(CompileErrors([CompileError::TagForbidden {...}]))` | explicit tag present |
| `YamlCompiler::compile` | rejects float scalars | `Err(CompileErrors([CompileError::FloatForbidden]))` | floating-point value |
| `YamlCompiler::compile` | rejects exceeding depth limit | `Err(CompileErrors([CompileError::DepthLimit {...}]))` | nesting > max_depth |
| `YamlCompiler::compile` | rejects exceeding node limit | `Err(CompileErrors([CompileError::NodeLimit {...}]))` | visited > max_nodes |
| `YamlCompiler::compile` | rejects invalid version | `Err(CompileErrors([CompileError::InvalidVersion {...}]))` | version != velvet-ballastics/v1 |
| `YamlCompiler::compile` | rejects missing `name` | `Err(CompileErrors([CompileError::MissingField { field: "name" }]))` | no name key |
| `YamlCompiler::compile` | rejects empty steps | `Err(CompileErrors([CompileError::EmptySteps]))` | steps is empty |
| `YamlCompiler::compile` | rejects last step != finish | `Err(CompileErrors([CompileError::LastStepMustFinish]))` | non-finish final step |
| `YamlCompiler::compile` | rejects bad step ID | `Err(CompileErrors([CompileError::InvalidName { field: "step id", ... }]))` | invalid identifier |
| `YamlCompiler::compile` | rejects duplicate step ID | `Err(CompileErrors([CompileError::DuplicateStepId {...}]))` | duplicate step ID |
| `YamlCompiler::compile` | rejects unknown trigger kind | `Err(CompileErrors([CompileError::UnknownTriggerKind {...}]))` | unknown trigger |
| `YamlCompiler::compile` | rejects bad trigger shape | `Err(CompileErrors([CompileError::TriggerShape {...}]))` | malformed trigger |
| `YamlCompiler::compile` | rejects wrong primitive field | `Err(CompileErrors([CompileError::UnknownStepPrimitiveField {...}]))` | unknown primitive |
| `YamlCompiler::compile` | rejects missing primitive | `Err(CompileErrors([CompileError::MissingStepPrimitive {...}]))` | step has no primitive |
| `YamlCompiler::compile` | rejects multiple primitives | `Err(CompileErrors([CompileError::MultipleStepPrimitives {...}]))` | >1 primitive in step |
| `YamlCompiler::compile` | rejects unsupported control fields | `Err(CompileErrors([CompileError::UnsupportedStepControlField {...}]))` | `if`, `with`, `try_again`, `on_error`, `then` |
| `YamlCompiler::compile` | rejects backward branch | `Err(CompileErrors([CompileError::BackwardBranchTarget {...}]))` | `on_true`/`on_false` points backward |
| `YamlCompiler::compile` | rejects empty choose branches | `Err(CompileErrors([CompileError::Workflow(WorkflowError::EmptyBranchTable)]))` | choose with no branches + no otherwise |
| `YamlCompiler::compile` | rejects uninitialized slot reference | `Err(CompileErrors([CompileError::UnknownSlotType {...}]))` | read-before-write slot |
| `YamlCompiler::compile` | rejects secret taint leak | `Err(CompileErrors([CompileError::SecretTaintLeak {...}]))` | secret crosses public boundary |
| `YamlCompiler::compile` | rejects bad expression syntax | `Err(CompileErrors([CompileError::ExpressionUnexpectedChar {...}]))` | invalid char in expression |
| `YamlCompiler::compile` | rejects unterminated string | `Err(CompileErrors([CompileError::ExpressionUnterminatedString {...}]))` | missing closing quote |
| `YamlCompiler::compile` | rejects out-of-range integer | `Err(CompileErrors([CompileError::ExpressionIntegerOutOfRange {...}]))` | i64 overflow |
| `YamlCompiler::compile` | rejects bad accessor path | `Err(CompileErrors([CompileError::UnsupportedAccessorReference {...}]))` | unknown accessor root/path |
| `YamlCompiler::compile` | rejects unknown reference kind | `Err(CompileErrors([CompileError::UnknownReferenceName {...}]))` | unknown `$kind.name` |
| `YamlCompiler::compile` | rejects illegal runtime reference | `Err(CompileErrors([CompileError::IllegalReference {...}]))` | deterministic ref to runtime-only |

### 1.2 `compile_workflow_with_contracts`

| Subject | Action | Outcome when | Condition |
|---------|--------|--------------|-----------|
| `compile_workflow_with_contracts` | accepts valid workflow + matching contracts | `Ok(CompiledWorkflow)` | all Do nodes have matching contracts |
| `compile_workflow_with_contracts` | rejects missing contract | `Err(CompileErrors([CompileError::UnknownSlotType {...}]))` | Do with unregistered action |
| `compile_workflow_with_contracts` | rejects orphan contract | `Err(CompileErrors([CompileError::UnknownSlotType {...}]))` | contract with no matching Do |
| `compile_workflow_with_contracts` | rejects unsafe retry + side-effect | `Err(CompileErrors([CompileError::IdempotencyViolation {...}]))` | SideEffect != None + RetrySafety::Unsafe |
| `compile_workflow_with_contracts` | rejects AtLeastOnceExternal | `Err(CompileErrors([CompileError::IdempotencyViolation {...}]))` | side-effect + AtLeastOnceExternal |

### 1.3 `lower_steps_to_ir`

| Subject | Action | Outcome when | Condition |
|---------|--------|--------------|-----------|
| `lower_steps_to_ir` | accepts valid parts | `Ok(CompiledWorkflow)` | valid WorkflowParts |
| `lower_steps_to_ir` | rejects empty nodes | `Err(CompileErrors([CompileError::Workflow(WorkflowError::EmptyNodes)]))` | nodes is empty |
| `lower_steps_to_ir` | rejects node ID mismatch | `Err(CompileErrors([CompileError::Workflow(WorkflowError::NodeIdMismatch {...})]))` | node.id != expected |
| `lower_steps_to_ir` | runs shared validation first | `Err(CompileErrors([...ValidationError::SlotReferenceOutOfRange...]))` | slot out of range (shared gate catches first) |

### 1.4 `validate_ir`

| Subject | Action | Outcome when | Condition |
|---------|--------|--------------|-----------|
| `validate_ir` | accepts valid parts | `Ok(CompiledWorkflow)` | valid WorkflowParts |
| `validate_ir` | returns shared validation errors | `Err(CompileErrors([...ValidationError::SlotReferenceOutOfRange...]))` | slot reference out of range |
| `validate_ir` | returns core structural errors | `Err(CompileErrors([...WorkflowError::EmptyNodes...]))` | empty nodes after shared passes |

### 1.5 Primitive Lowering Functions

| Subject | Action | Outcome when | Condition |
|---------|--------|--------------|-----------|
| `lower_set` | produces SetConst node | returns `CompiledNodeKind::SetConst` | valid inputs |
| `lower_do` | produces Do node | returns `CompiledNodeKind::Do` | valid action + slots |
| `lower_choose` | rejects empty branches | `Err(CompileError::Workflow(WorkflowError::EmptyBranchTable))` | no branches + no otherwise |
| `lower_choose` | produces ChooseSlot node | `Ok(CompiledNodeKind::ChooseSlot)` | valid branches |
| `lower_for_each` | produces ForEachStart + ForEachNext | `Ok([CompiledNode, CompiledNode])` | valid slots |
| `lower_together` | rejects >u16::MAX branches | `Err(CompileError::PrimitiveLoweringLimitExceeded {...})` | branches.len() > 65535 |
| `lower_together` | produces TogetherStart + TogetherJoin | `Ok([CompiledNode, CompiledNode])` | valid branch count |
| `lower_collect` | produces CollectStart + CollectPage + CollectFinish | `Ok([CompiledNode, CompiledNode, CompiledNode])` | valid source + slots |
| `lower_reduce` | produces ReduceStart + ReduceNext + ReduceFinish | `Ok([CompiledNode, CompiledNode, CompiledNode])` | valid input + accumulator |
| `lower_repeat` | produces RepeatStart + RepeatAttempt + RepeatFinish | `Ok([CompiledNode, CompiledNode, CompiledNode])` | valid max_attempts |
| `lower_wait` (Until) | produces WaitUntil | returns `CompiledNodeKind::WaitUntil` | WaitKind::Until |
| `lower_wait` (Event) | produces WaitEvent | returns `CompiledNodeKind::WaitEvent` | WaitKind::Event |
| `lower_ask` | rejects step index overflow | `Err(CompileError::PrimitiveLoweringLimitExceeded {...})` | id + 1 > u16::MAX |
| `lower_ask` | produces Ask + AskResume | `Ok([CompiledNode, CompiledNode])` | valid prompt + answer slots |
| `lower_finish` | produces Finish node | returns `CompiledNodeKind::Finish` | valid result slot |
| `check_idempotency_gates` | accepts safe contracts | `Ok(())` | no violations |
| `check_idempotency_gates` | rejects unsafe combination | `Err(CompileErrors([CompileError::IdempotencyViolation {...}]))` | side-effect + unsafe retry |

### 1.6 SlotCompiler

| Subject | Action | Outcome when | Condition |
|---------|--------|--------------|-----------|
| `SlotCompiler::push_constant` | rejects pool overflow | `Err(CompileError::Workflow(WorkflowError::ConstOutOfBounds {...}))` | constants.len() >= 65536 |
| `SlotCompiler::push_expression` | rejects table overflow | `Err(CompileError::ExpressionLoweringUnsupported {...})` | expressions.len() >= 65536 |
| `SlotCompiler::push_accessor` | rejects table overflow | `Err(CompileError::ExpressionLoweringUnsupported {...})` | accessors.len() >= 65536 |
| `SlotCompiler::slot_count` | rejects slot count overflow | `Err(CompileError::SlotIndexOutOfRange {...})` | max_slot + 1 > u16::MAX |
| `SlotCompiler::build_parts` | rejects slot count overflow | `Err(CompileError::SlotIndexOutOfRange {...})` | computed slot_count > u16::MAX |

---

## 2. Trophy Allocation

### 2.1 Layer Distribution Target

| Layer | Target | Rationale |
|-------|--------|-----------|
| **Static** (clippy, types) | 0 errors, 0 warnings | Free; catches 46 clippy errors currently |
| **Unit** (`#[cfg(test)]` in lib.rs + inline modules) | ~30% of behaviors | Calc-layer pure functions, proptest invariants |
| **Integration** (`/tests/` directory) | ~60% of behaviors | Real deps; full compile pipeline; YAML round-trips |
| **E2E** | ~5% | Postcard serialize/deserialize round-trip; codegen emit |
| **Fuzz** | Parser/deserializer inputs | `strict_yaml`, expression lexer, YAML parser boundaries |

### 2.2 Current State vs. Target

| Metric | Current | Target | Gap |
|--------|---------|--------|-----|
| Line coverage | 67.52% | ≥90% | 22.48pp |
| Branch coverage | 76.55% | ≥90% | 13.45pp |
| Clippy errors | 37 | 0 | 37 |
| Clippy warnings | 11 | 0 | 11 |

---

## 3. BDD Scenarios

### 3.1 Compilation Happy Path

#### `compile_workflow accepts minimal valid workflow`

```
Given: a minimal YAML source with name, manual trigger, and a single finish step
When:  YamlCompiler::default().compile(source) is called
Then:  returns Ok(CompiledWorkflow) where workflow.name == "test"
       and workflow.node_count() == 2 (SetConst + Finish)
```

#### `compile_workflow accepts full v1 workflow with all primitives`

```
Given: a YAML source declaring inputs, vars, secrets, and steps using all primitives (set, do, choose, for_each, together, collect, reduce, repeat, wait, ask, finish)
When:  YamlCompiler::default().compile(source) is called
Then:  returns Ok(CompiledWorkflow) with correct node count for each primitive
```

### 3.2 Error Rejection Scenarios

#### `compile_workflow rejects invalid UTF-8`

```
Given: a byte sequence that is not valid UTF-8
When:  YamlCompiler::default().compile(source) is called
Then:  returns Err(CompileErrors) where the inner error is CompileError::Utf8(_)
```

#### `compile_workflow rejects duplicate step IDs`

```
Given: a YAML workflow with two steps sharing the same id "dup"
When:  YamlCompiler::default().compile(source) is called
Then:  returns Err(CompileErrors) where the error is CompileError::DuplicateStepId { id: "dup" }
```

#### `compile_workflow rejects uninitialized slot reference`

```
Given: a YAML workflow with a do step referencing input slot 99 which was never written
When:  YamlCompiler::default().compile(source) is called
Then:  returns Err(CompileErrors) where the error is CompileError::UnknownSlotType { field: "do.input", slot: 99 }
```

#### `compile_workflow rejects secret taint leak across public boundary`

```
Given: a YAML workflow where a secret-tainted slot is used in a public result field
When:  YamlCompiler::default().compile(source) is called
Then:  returns Err(CompileErrors) where the error is CompileError::SecretTaintLeak { field: "finish.result" }
```

### 3.3 Idempotency Gate Scenarios

#### `check_idempotency_gates accepts pure deterministic action`

```
Given: an ActionContract with side_effect = SideEffect::None and retry_safety = RetrySafety::Safe
When:  check_idempotency_gates(&[contract]) is called
Then:  returns Ok(())
```

#### `check_idempotency_gates rejects side-effect + unsafe retry`

```
Given: an ActionContract with side_effect = SideEffect::FileSystem and retry_safety = RetrySafety::Unsafe
When:  check_idempotency_gates(&[contract]) is called
Then:  returns Err(CompileErrors) where the error is CompileError::IdempotencyViolation { reason: contains("RetrySafety::Unsafe") }
```

### 3.4 Lowering Scenarios

#### `lower_together rejects excessive branch count`

```
Given: a together primitive with 70000 branches
When:  lower_together(id, branches, join, builder) is called
Then:  returns Err(CompileError::PrimitiveLoweringLimitExceeded { primitive: "together", field: "branches", value: 70000, limit: 65536 })
```

#### `lower_choose rejects empty branch table`

```
Given: a choose primitive with empty branches and no otherwise target
When:  lower_choose(id, branches, otherwise, builder) is called
Then:  returns Err(CompileError::Workflow(WorkflowError::EmptyBranchTable))
```

### 3.5 Round-trip Scenarios

#### `emit_compiled_artifact and load round-trip preserves workflow`

```
Given: a valid CompiledWorkflow from compile_workflow
When:  emit_compiled_artifact(workflow) is called then the bytes are deserialized
Then:  returns a CompiledWorkflow with identical node_count, slot_count, and constant pool
```

---

## 4. LETHAL Findings — Exact Fixes

### 4.1 `test_21.rs:350` — Silent Result Discard

**File:** `crates/vb_compile/src/tests/test_21.rs:350`

**Current:**
```rust
// The result is deterministic -- either compile or validation error
let _ = YamlCompiler::default().compile(source);
```

**Problem:** `let _ =` silently discards the `Result`, providing zero evidence the test ran.

**Fix:**
```rust
// The result is deterministic -- either compile or validation error
// Assert that we get a specific variant to prove the code path was exercised
let result = YamlCompiler::default().compile(source);
// Either the single-branch together compiles successfully, or it fails
// with a specific validation error about branch routing.
// Both outcomes are valid; we just need to prove the match ran.
match result {
    Ok(workflow) => {
        // Single branch with join after finish is valid
        assert_eq!(workflow.node_count(), 3, "single-branch together should produce 3 nodes");
    }
    Err(errors) => {
        // If it fails validation, the error should mention branch/routing
        let err_text = errors.first().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            err_text.contains("branch") || err_text.contains("target") || err_text.contains("slot"),
            "expected branch/target/slot error, got: {}",
            err_text
        );
    }
}
```

### 4.2 16 Unreachable Patterns in `lib.rs`

**File:** `crates/vb_compile/src/lib.rs`

**Pattern:** In test `match` arms, `Err(...)` and `Ok(_)` are matched, then `other => panic!(...)` is added as a catch-all. Since `Result` is `Ok(T) | Err(E)`, the `other` case is truly unreachable.

**Locations:**
- `lib.rs:3826` — inside `#[test] fn lower_steps_to_ir_validates_before_core()`
- `lib.rs:3863` — inside `#[test] fn validate_ir_orders_shared_validation_before_core()`
- `lib.rs:3942` — inside `#[test] fn lower_steps_to_ir_returns_workflow_error_for_empty_nodes()`
- `lib.rs:3977` — inside `#[test] fn lower_steps_to_ir_returns_workflow_error_for_node_id_mismatch()`
- `lib.rs:3998` — inside `#[test] fn validate_ir_returns_workflow_error_when_core_fails_after_shared_passes()`
- `lib.rs:4040` — inside `#[test] fn compile_workflow_with_contracts_rejects_missing_action_contract()`
- `lib.rs:4091` — inside `#[test] fn compile_workflow_with_contracts_rejects_orphan_action_contract()`
- `lib.rs:4188` — inside `#[test] fn validate_ir_returns_validation_error_when_shared_fails_first()`

**Fix:** Remove the unreachable `other => panic!(...)` arm. Replace with `debug_assert` if the invariant must be documented in production builds, but in test code simply delete the arm:

**Before:**
```rust
match result {
    Err(CompileErrors(ref errors)) => { /* assertions */ }
    Ok(_) => panic!("Expected error, got Ok"),
    other => panic!("Expected CompileErrors, got: {:?}", other),
}
```

**After:**
```rust
match result {
    Err(CompileErrors(ref errors)) => { /* assertions */ }
    Ok(_) => panic!("Expected error, got Ok"),
    // Note: Err/Ok are exhaustive for Result; no 'other' case exists
}
```

For the 8 double-nested `other => panic!` arms inside inner `match err` blocks (e.g., `lib.rs:3816`, `lib.rs:3856`, etc.), the same applies: `CompileError` is an enum, and once `Validation(...)` and `Workflow(...)` variants are matched, no other variant exists. Remove these arms.

### 4.3 27 `panic!` in Test Code

All 27 `panic!` calls in the test module (`#[cfg(test)]` block at end of `lib.rs`) violate `clippy::panic`. These are test assertions, not production code — but `#![forbid(unsafe_code)]` does not disable lint in test mode.

**Fix approach:** Replace `panic!("message")` with `#[allow(clippy::panic)]` on the specific arms or functions, OR use `#[track_caller]` with proper assertion helpers.

Example for `lib.rs:3822`:
```rust
// Before:
Ok(_) => panic!("Expected error but lower_steps_to_ir succeeded..."),

// After:
Ok(_) => {
    #[allow(clippy::panic)]
    {
        panic!("Expected error but lower_steps_to_ir succeeded. This FAILS before fix because lower_steps_to_ir bypasses Gate 9.");
    }
}
```

OR, better — use the existing `compile_test_fail!` macro pattern from helpers:
```rust
Ok(_) => compile_test_fail!(
    "Expected error but lower_steps_to_ir succeeded. This FAILS before fix because lower_steps_to_ir bypasses Gate 9."
),
```

### 4.4 7 `expect()` on Option in Test Code

**Pattern:** `errors.first().expect("should have first error")` after already matching `Err(CompileErrors(ref errors))` where `errors.len() == 1`.

**Locations:** `lib.rs:3801`, `lib.rs:3841`, `lib.rs:3935`, `lib.rs:3967`, `lib.rs:4030`, `lib.rs:4075`, `lib.rs:4182`

**Fix:** Use `unwrap()` since the length assertion precedes it, or restructure:

```rust
// Before:
assert_eq!(errors.len(), 1, "Expected exactly 1 error, got {}", errors.len());
let err = errors.first().expect("should have first error");

// After:
let err = match errors.as_slice() {
    [err] => err,
    _ => panic!("Expected exactly 1 error, got {}", errors.len()),
};
```

### 4.5 2 `unwrap()` on Result in Test Code

**Locations:** `lib.rs:3886` (`let workflow = result.unwrap();`), `lib.rs:3905`

**Fix:** These occur after `assert!(result.is_ok())` checks. The `unwrap()` is technically safe due to the assertion, but the linter complains. Replace:

```rust
// Before:
assert!(result.is_ok(), "lower_steps_to_ir should succeed for valid parts");
let workflow = result.unwrap();

// After:
let workflow = match result {
    Ok(w) => w,
    Err(e) => panic!("lower_steps_to_ir should succeed for valid parts, got: {e}"),
};
```

### 4.6 Redundant Guard in `ast/tests.rs:395`

**File:** `crates/vb_compile/src/ast/tests.rs:395`

**Current:**
```rust
CompileError::PrimitiveLoweringLimitExceeded { value, .. } if value == 70000 => Ok(()),
```

**Fix:**
```rust
CompileError::PrimitiveLoweringLimitExceeded { value: 70000, .. } => Ok(()),
```

### 4.7 Length Comparison to One in `references/tests.rs:925`

**File:** `crates/vb_compile/src/references/tests.rs:925`

**Fix:** Change `x.len() == 1` to `x.len() == 1` (already correct) or use `matches!(x.len(), 1)` if clippy is complaining about the comparison pattern. Alternatively, add `#[allow(clippy::cmp_owned)]`.

---

## 5. Proptest Invariants

### 5.1 `is_public_name` — Valid Name Grammar

**Function:** `is_public_name` (private, but pure — test via public API effects)

**Invariant:**
```rust
prop_forall!(name in "\\a[a-z][a-z0-9_]{0,62}".prop_filter("reserved names excluded"))
assert!(is_public_name(name) == true);

prop_forall!(name in "\\A[A-Z].*\\z" | "\\A.*\\s.*\\z" | "\\A_{1,}\\z")
assert!(is_public_name(name) == false);
```

**Input Strategy:**
- ASCII lowercase first char, then ASCII alphanumerics + underscore
- Length 1-64
- Exclude reserved names

### 5.2 `validate_public_name` — Round-trip

**Invariant:** For any `name` where `is_public_name(name)` is true, `validate_public_name("step id", name)` returns `Ok(())`.

### 5.3 `lower_together` — Branch Count Bounds

**Function:** `lower_together`

**Invariant:** For any `branches: Vec<StepIdx>` with `branches.len() <= 65535`, `lower_together(id, branches, join, builder)` returns `Ok(...)`. For `branches.len() > 65535`, returns `Err(CompileError::PrimitiveLoweringLimitExceeded { primitive: "together", ...})`.

### 5.4 `SlotCompiler::slot_count` — Monotonicity

**Invariant:** After calling `record_slot(slot)` N times with slots `[0..N-1]`, `slot_count()` returns `Ok(N)`.

### 5.5 `CompileError::code()` — Deterministic

**Invariant:** For any `CompileError` variant, `code()` returns a non-empty static string that is stable across calls.

### 5.6 `CompileErrors::len` — Accurate Length

**Invariant:** After collecting N errors, `len()` returns N and `is_empty()` returns `false`.

---

## 6. Fuzz Targets

### 6.1 `strict_yaml` Profile Rejection

**File:** `crates/vb_compile/src/strict_yaml.rs`

**Target:** `reject_unsupported_profile_events(text: &str)`

**Risk:** YAML parser edge cases; malformed UTF-8; unicode bombing

**Corpus Seeds:**
- Valid v1 YAML workflows (positive)
- YAML with anchors/aliases (should reject)
- YAML with tags (should reject)
- Deeply nested YAML (should reject at depth limit)
- Multi-document YAML (should reject)

### 6.2 Expression Lexer

**File:** `crates/vb_compile/src/expression.rs`

**Target:** `Lexer::new(source).collect::<Result<Vec<Token>, _>>()`

**Risk:** Panic on invalid UTF-8; catastrophic backtracking on long inputs

**Corpus Seeds:**
- Valid expressions: `$input.name`, `$vars.count + 1`, `$secrets.api_key`
- Invalid: unterminated strings, overflow integers, bad chars

### 6.3 `checked_utf8` Boundary

**Function:** `checked_utf8(source: &[u8], limits: YamlLimits) -> Result<&str, CompileError>`

**Risk:** Non-UTF-8 bytes; source exactly at limit; empty source

### 6.4 `reject_duplicate_mapping_keys`

**Risk:** Hash collision DoS (though HashSet uses secure hasher in prod); deeply nested duplicates

### 6.5 `lower_choose` — Empty Branch Table

**Function:** `lower_choose(id, branches, otherwise, builder)`

**Risk:** `branches.is_empty() && otherwise.is_none()` — boundary case

---

## 7. Kani Harnesses

### 7.1 `lower_together` — Branch Count Limit

```rust
// kani::proof
fn lower_together_branch_count_bounded() {
    // Given: branches.len() is u16::MAX + 1 = 65536
    // When: lower_together is called
    // Then: returns Err(PrimitiveLoweringLimitExceeded)
    // Bound: branches.len() == 65536
}
```

### 7.2 `SlotCompiler::slot_count` — No Overflow

```rust
// kani::proof  
fn slot_count_never_panics() {
    // Given: SlotCompiler with any sequence of record_slot calls
    // When: slot_count() is called
    // Then: returns Ok(u16) or Err(SlotIndexOutOfRange), never panics
    // Bound: max_slot value space
}
```

### 7.3 `validate_depth` — Depth Limit Check

```rust
// kani::proof
fn depth_limit_never_overflows() {
    // Given: depth <= u16::MAX
    // When: validate_depth(depth, limits) is called
    // Then: returns Err if depth > limits.max_depth, Ok otherwise
    // Bound: depth as u16
}
```

---

## 8. Mutation Testing Checkpoints

### 8.1 `CompileError::code()` Completeness

| Mutation | Kill Test |
|----------|-----------|
| Remove `SourceTooLarge` case | `test_error_code_source_too_large` — checks exact string |
| Remove `Utf8` case | `test_error_code_utf8` — checks exact string |
| Swap two error codes | `test_error_codes_are_distinct` — checks pairwise inequality |

### 8.2 Idempotency Gate Mutations

| Mutation | Kill Test |
|----------|-----------|
| Change `SideEffect::None` branch to reject | `check_idempotency_gates_accepts_none` |
| Remove `RetrySafety::Unsafe` rejection | `check_idempotency_gates_rejects_unsafe_retry` |
| Remove `AtLeastOnceExternal` rejection | `check_idempotency_gates_rejects_at_least_once_external` |
| Change condition `contract.side_effect == SideEffect::None` to `!=` | `check_idempotency_gates_rejects_side_effect_with_unsafe` |

### 8.3 Lowering Function Mutations

| Mutation | Kill Test |
|----------|-----------|
| Change `lower_together` branch limit from `u16::MAX` to `u16::MAX - 1` | `lower_together_accepts_max_branches`, `lower_together_rejects_over_max_branches` |
| Remove `validate_branch_route` check | `lower_choose_rejects_empty_branches` |
| Change `lower_for_each` to use wrong slot | `lower_for_each_uses_correct_item_slot` |

**Target:** ≥90% mutation kill rate using `mutate` or `cargo-mutants`.

---

## 9. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| compile_workflow happy path | valid v1 YAML with all primitives | `Ok(CompiledWorkflow)` with correct node count | integration |
| compile_workflow rejects bad version | version != velvet-ballastics/v1 | `Err(CompileError::InvalidVersion)` | unit |
| compile_workflow rejects bad trigger | unknown trigger kind | `Err(CompileError::UnknownTriggerKind)` | unit |
| compile_workflow rejects empty steps | steps: [] | `Err(CompileError::EmptySteps)` | unit |
| compile_workflow rejects last non-finish | last step is `do:` not `finish:` | `Err(CompileError::LastStepMustFinish)` | unit |
| compile_workflow rejects duplicate keys | duplicate YAML mapping key | `Err(CompileError::DuplicateKey)` | unit |
| compile_workflow rejects uninitialized slot | do.input: 99 (never written) | `Err(CompileError::UnknownSlotType { slot: 99 })` | integration |
| compile_workflow rejects secret leak | secret used in finish.result | `Err(CompileError::SecretTaintLeak)` | integration |
| compile_workflow rejects bad expression | expression with invalid char | `Err(CompileError::ExpressionUnexpectedChar)` | unit |
| lower_together accepts max = 65535 | branches.len() == 65535 | `Ok([TogetherStart, TogetherJoin])` | unit |
| lower_together rejects over max | branches.len() == 65536 | `Err(CompileError::PrimitiveLoweringLimitExceeded)` | unit |
| lower_choose accepts valid branches | branches: [SlotBranch {...}] | `Ok(CompiledNodeKind::ChooseSlot)` | unit |
| lower_choose rejects empty | branches: [], otherwise: None | `Err(CompileError::Workflow(EmptyBranchTable))` | unit |
| lower_for_each produces 2 nodes | valid for_each YAML | `Ok([ForEachStart, ForEachNext])` | unit |
| lower_do produces Do node | valid action + slots | `CompiledNodeKind::Do { action, input }` | unit |
| check_idempotency_gates accepts pure | side_effect: None | `Ok(())` | unit |
| check_idempotency_gates rejects unsafe | side_effect: FileSystem + RetrySafety::Unsafe | `Err(CompileErrors([IdempotencyViolation {...}]))` | unit |
| emit_compiled_artifact round-trip | valid CompiledWorkflow | `Ok(bytes)` then deserialize equals original | integration |
| compile_to_generated_rust emits | valid CompiledWorkflow | `Ok(String)` containing Rust source | integration |

---

## 10. Coverage Gap Analysis

**Current:** 67.52% line, 76.55% branch  
**Target:** ≥90% line, ≥90% branch

### 10.1 Uncovered Production Code (High Priority)

| Module | Uncovered Lines | Likely Reason |
|--------|-----------------|---------------|
| `strict_yaml` | profile rejection paths | no integration test with anchors/tags |
| `expression_bytecode` | helper lowering | limited bytecode tests |
| `compile_builder` | error constructor paths | not called from integration tests |
| `references` | reference validation branches | narrow input coverage |

### 10.2 Coverage Improvement Actions

1. **Add integration test with YAML anchors/tags** — exercises `AliasForbidden`, `AnchorForbidden`, `TagForbidden`
2. **Add integration test with deep nesting** — exercises `DepthLimit`
3. **Add unit tests for each `PrimitiveLoweringLimitExceeded` boundary** — especially `lower_together` at exactly 65535/65536
4. **Add round-trip test for every error code** — each `CompileError` variant should be constructible and displayable from integration test
5. **Expand expression lexer test corpus** — unterminated strings, integer overflow, accessor paths

---

## 11. Verification

After implementing fixes:

```bash
# 1. Clippy must be clean
cargo clippy -p vb_compile --all-targets 2>&1 | grep -c "^error:"  # must be 0

# 2. Tests must pass
cargo test -p vb_compile 2>&1 | tail -5  # must show "246 passed"

# 3. Coverage must meet threshold
cargo llvm-cov -p vb_compile 2>&1 | grep "^TOTAL"  # line >= 90%, branch >= 90%
```
