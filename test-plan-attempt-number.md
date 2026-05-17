# Test Plan: MAJOR-5 — `$attempt.number` Restriction Not Implemented

## Summary

- **Bead ID**: MAJOR-5
- **Feature**: Compile-time restriction enforcement for `$attempt.number` variable scope
- **Behaviors identified**: 2 core behaviors (happy path + scope violation)
- **Trophy allocation**: 2 unit / 3 integration / 1 e2e (asymmetric — restriction testing)
- **Proptest invariants**: 1 (scope context preservation)
- **Fuzz targets**: 1 (expression parser with `$attempt` variants)
- **Kani harnesses**: 1 (exhaustive scope state machine)

---

## 1. Behavior Inventory

### B1: `$attempt.number` accessible inside repeat attempt blocks

**Description**: When `$attempt.number` appears in an expression within the body of a `repeat` step, the compiler must accept it and bind it to the current attempt count (1-indexed).

**Public API**: `vb_compile::YamlCompiler::parse_ast()` → `Result<WorkflowAst, CompileErrors>`

**Trigger**: YAML workflow containing a `repeat` step whose body expressions reference `$attempt.number`

**Guarantees**:
- Compilation succeeds without `CompileError`
- The reference is retained in the AST as `AstExpression::Reference("$attempt.number")`
- The reference is NOT resolved at compile time (runtime binding only)

---

### B2: `$attempt.number` NOT accessible outside attempt blocks

**Description**: When `$attempt.number` appears in an expression outside the body of a `repeat` step, the compiler must reject it with `CompileError::InvalidVariableScope`.

**Public API**: `vb_compile::YamlCompiler::parse_ast()` → `Result<WorkflowAst, CompileErrors>`

**Trigger**: YAML workflow containing `$attempt.number` in:
- Top-level `vars` expressions
- Top-level `inputs` expressions
- Top-level `examples` values
- `finish.result` expression
- `save` field expressions
- Any step that is NOT inside a `repeat` body
- Inside `for_each`, `together`, `collect`, `reduce`, `wait`, `ask` bodies

**Guarantees**:
- Compilation fails with `CompileErrors` containing `CompileError::InvalidVariableScope`
- Error message includes the offending reference (`$attempt.number`)
- Error message indicates the reference is only valid inside `repeat` bodies

---

## 2. Trophy Allocation

| Scenario | Test Layer | Rationale |
|----------|------------|-----------|
| `$attempt.number` in repeat body compiles | Integration | Real compiler pipeline, exercise full AST → validation flow |
| `$attempt.number` rejected outside repeat | Unit | Scope restriction logic is pure function, can test in isolation |
| Multiple nesting levels of repeat | Unit | Exhaustive scope context tracking |
| Mixed expressions with `$attempt.number` | Integration | Real expression bytecode compilation |
| E2E: compile valid repeat workflow | E2E | Full black-box CLI compile with `$attempt.number` in repeat |
| `$attempt.number` in different step types | Unit | Each step type scope boundary |

**Ratios**: ~50% unit / ~33% integration / ~17% e2e
Justification: Restriction enforcement is scope-logic heavy; integration confirms pipeline wiring; e2e validates CLI boundary.

---

## 3. BDD Scenarios

### Behavior B1: `$attempt.number` accessible inside repeat attempt blocks

#### Scenario 1: `$attempt.number` in repeat body `save` field

**Given**: A workflow with a `repeat` step containing a `save` field that references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: The compilation succeeds with no errors

```
fn attempt_number_in_repeat_body_save_field_compiles()
```

```yaml
version: velvet-ballastics/v1
name: repeat_with_attempt
when:
  manual: {}
steps:
  - id: retry_step
    repeat:
      max_attempts: 3
      steps:
        - id: log_attempt
          save:
            current_attempt: $attempt.number
  - id: done
    finish:
      result: 0
```

#### Scenario 2: `$attempt.number` in repeat body `do` input expression

**Given**: A workflow with a `repeat` step containing a `do` action whose input references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: The compilation succeeds with no errors

```
fn attempt_number_in_repeat_body_do_input_compiles()
```

```yaml
version: velvet-ballastics/v1
name: repeat_do_with_attempt
when:
  manual: {}
steps:
  - id: retry_action
    repeat:
      max_attempts: 3
      steps:
        - id: call_api
          do: my_action
          input: $attempt.number
  - id: done
    finish:
      result: 0
```

#### Scenario 3: `$attempt.number` in deeply nested repeat body

**Given**: A workflow with a `repeat` step nested inside another `repeat`, where the inner repeat body references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: The compilation succeeds (inner repeat body is valid scope for `$attempt.number`)

```
fn attempt_number_in_nested_repeat_body_compiles()
```

```yaml
version: velvet-ballastics/v1
name: nested_repeat
when:
  manual: {}
steps:
  - id: outer_retry
    repeat:
      max_attempts: 2
      steps:
        - id: inner_retry
          repeat:
            max_attempts: 3
            steps:
              - id: log
                save:
                  value: $attempt.number
  - id: done
    finish:
      result: 0
```

#### Scenario 4: `$attempt.number` in `choose` condition inside repeat body

**Given**: A workflow with a `repeat` step containing a `choose` whose condition references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: The compilation succeeds

```
fn attempt_number_in_choose_condition_inside_repeat_compiles()
```

```yaml
version: velvet-ballastics/v1
name: repeat_choose_attempt
when:
  manual: {}
steps:
  - id: conditional_retry
    repeat:
      max_attempts: 3
      steps:
        - id: check
          choose:
            condition: $attempt.number > 1
            on_true: 1
            on_false: 1
  - id: done
    finish:
      result: 0
```

---

### Behavior B2: `$attempt.number` NOT accessible outside attempt blocks

#### Scenario 5: `$attempt.number` in top-level `vars` rejected

**Given**: A workflow with a `vars` declaration that references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: Compilation fails with `CompileError::InvalidVariableScope { reference: "$attempt.number", context: "vars" }`

```
fn attempt_number_in_vars_rejected_with_invalid_variable_scope()
```

```yaml
version: velvet-ballastics/v1
name: vars_attempt_error
when:
  manual: {}
vars:
  current: $attempt.number
steps:
  - id: done
    finish:
      result: 0
```

#### Scenario 6: `$attempt.number` in `finish.result` rejected

**Given**: A workflow with a `finish.result` expression that references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: Compilation fails with `CompileError::InvalidVariableScope { reference: "$attempt.number", context: "finish.result" }`

```
fn attempt_number_in_finish_result_rejected_with_invalid_variable_scope()
```

```yaml
version: velvet-ballastics/v1
name: finish_attempt_error
when:
  manual: {}
steps:
  - id: done
    finish:
      result: $attempt.number
```

#### Scenario 7: `$attempt.number` in `save` field outside repeat rejected

**Given**: A workflow with a `save` step (not inside a `repeat`) that references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: Compilation fails with `CompileError::InvalidVariableScope { reference: "$attempt.number", context: "save" }`

```
fn attempt_number_in_save_outside_repeat_rejected_with_invalid_variable_scope()
```

```yaml
version: velvet-ballastics/v1
name: save_attempt_error
when:
  manual: {}
steps:
  - id: log_attempt
    save:
      value: $attempt.number
  - id: done
    finish:
      result: 0
```

#### Scenario 8: `$attempt.number` in `for_each` body rejected

**Given**: A workflow with a `for_each` step whose body references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: Compilation fails with `CompileError::InvalidVariableScope { reference: "$attempt.number", context: "for_each.body" }`

```
fn attempt_number_in_for_each_body_rejected_with_invalid_variable_scope()
```

```yaml
version: velvet-ballastics/v1
name: foreach_attempt_error
when:
  manual: {}
vars:
  items: [1, 2, 3]
steps:
  - id: iterate
    for_each:
      items: $vars.items
      do: iterate_item
  - id: done
    finish:
      result: 0
```

(Note: The actual test would need a body step that uses `$attempt.number` inside the `for_each`)

#### Scenario 9: `$attempt.number` in `examples` value rejected

**Given**: A workflow with an `examples` entry that references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: Compilation fails with `CompileError::InvalidVariableScope { reference: "$attempt.number", context: "examples" }`

```
fn attempt_number_in_examples_rejected_with_invalid_variable_scope()
```

```yaml
version: velvet-ballastics/v1
name: examples_attempt_error
when:
  manual: {}
examples:
  - name: test_case
    attempt_val: $attempt.number
steps:
  - id: done
    finish:
      result: 0
```

#### Scenario 10: `$attempt.number` in `choose` condition outside repeat rejected

**Given**: A workflow with a `choose` step (not inside a `repeat`) whose condition references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: Compilation fails with `CompileError::InvalidVariableScope { reference: "$attempt.number", context: "choose.condition" }`

```
fn attempt_number_in_choose_outside_repeat_rejected_with_invalid_variable_scope()
```

```yaml
version: velvet-ballastics/v1
name: choose_attempt_error
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: $attempt.number > 1
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: 0
```

#### Scenario 11: `$attempt.number` in `reduce` body rejected

**Given**: A workflow with a `reduce` step whose body references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: Compilation fails with `CompileError::InvalidVariableScope { reference: "$attempt.number", context: "reduce.body" }`

```
fn attempt_number_in_reduce_body_rejected_with_invalid_variable_scope()
```

```yaml
version: velvet-ballastics/v1
name: reduce_attempt_error
when:
  manual: {}
vars:
  data: [1, 2, 3]
steps:
  - id: sum_values
    reduce:
      data: $vars.data
      initial: 0
      body:
        - id: accumulate
          save:
            sum: $attempt.number  # Invalid in reduce body
  - id: done
    finish:
      result: 0
```

#### Scenario 12: `$attempt.number` in `wait` body rejected

**Given**: A workflow with a `wait` step whose body references `$attempt.number`
**When**: `YamlCompiler::default().parse_ast(source)` is called
**Then**: Compilation fails with `CompileError::InvalidVariableScope { reference: "$attempt.number", context: "wait" }`

(Note: `wait` doesn't have a body per se, but the error should occur if `$attempt.number` is used in related expressions)

---

## 4. Proptest Invariants

### Invariant 1: Scope context preservation through AST traversal

**Function**: `validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors>`

**Invariant**: For any valid `WorkflowAst`, if a reference `$attempt.number` appears in any expression, it must be located within the `body` field of at least one `StepKindAst::Repeat` ancestor step in the AST.

**Strategy**:
- Generate any valid `WorkflowAst` (respecting `WorkflowAst` invariants)
- Collect all `$attempt.number` references via AST traversal
- For each reference, verify it has a `Repeat` ancestor

**Anti-invariant**: Any AST where `$attempt.number` appears without a `Repeat` ancestor is invalid.

```rust
/// Proptest: attempt_number_references_must_have_repeat_ancestor
/// Invariant: All $attempt.number references are inside repeat bodies
/// Strategy: Any valid WorkflowAst
/// Anti-invariant: $attempt.number outside repeat body → should not compile
```

---

## 5. Fuzz Targets

### Fuzz Target: Expression parser with `$attempt` variants

**Target function**: `crate::expression::parse_expression()` (or equivalent)

**Input type**: Arbitrary `&str` expression strings

**Risk**:
- Panic on malformed `$attempt.number` expressions
- Panic if `$attempt` is parsed as a valid root without scope checking
- Logic error: `$attempt.number` accepted outside intended scope

**Corpus seeds**:
- `"$attempt.number"` — valid inside repeat
- `"$attempt"` — bare, should be rejected
- `"$attempt.number.extra"` — nested accessor, should be rejected
- `"$attempt.number > 1"` — binary expression with attempt
- `"$attempt.number + $vars.count"` — binary with mixed refs
- `"$attempt . number"` — spaces, should still parse
- `""` — empty
- `"$slot.0"` — unrelated reference

---

## 6. Kani Harnesses

### Kani Harness: Exhaustive scope state machine

**Property**: Given a workflow AST with N steps and M `repeat` blocks, any reference path to a `$attempt.number` must traverse through at least one `repeat` block's body scope.

**Bound**: N ≤ 10 steps, M ≤ 3 nested repeat blocks, expression depth ≤ 5

**Rationale**: Scope tracking is a finite state machine. Kani can exhaustively verify that the scope context is correctly tracked through all possible AST traversal paths. This is critical because scope violations are security/correctness issues — property testing cannot exhaustively prove absence of scope violations.

```rust
/// Kani proof: attempt_number_scope_invariant
/// Property: $attempt.number is only accessible when scope_context == InRepeatBody
/// Bound: WorkflowAst with up to 10 steps, 3 nested repeats
```

---

## 7. Mutation Checkpoints

**Critical mutations to survive:**

| Function / Branch | Must be caught by test |
|-------------------|------------------------|
| `validate_compile_reference` — skip `$attempt` check | `attempt_number_in_vars_rejected_*` |
| `validate_compile_reference` — accept bare `$attempt` | `bare_attempt_reference_rejected` |
| Scope tracking — never set `InRepeatBody` | `attempt_number_in_repeat_body_*` |
| Scope tracking — never reset `InRepeatBody` on exit | `nested_repeat_attempt_number_rejected_at_wrong_scope` |
| `is_valid_attempt_reference` — always return `true` | All B2 scenarios |
| `is_valid_attempt_reference` — always return `false` | All B1 scenarios |

**Threshold**: 90% mutation kill rate minimum.

---

## 8. Combinatorial Coverage Matrix

### Unit Tests: Scope Context Tracking

| Scenario | Input | Expected Output | Layer |
|----------|-------|-----------------|-------|
| `$attempt.number` in `vars` | Valid YAML with `$attempt.number` in vars | `Err(InvalidVariableScope)` | unit |
| `$attempt.number` in `examples` | Valid YAML with `$attempt.number` in examples | `Err(InvalidVariableScope)` | unit |
| `$attempt.number` in `finish.result` | Valid YAML with `$attempt.number` in finish | `Err(InvalidVariableScope)` | unit |
| `$attempt.number` in `save` (not repeat) | Valid YAML with `$attempt.number` in save | `Err(InvalidVariableScope)` | unit |
| `$attempt.number` in `choose.condition` (not repeat) | Valid YAML | `Err(InvalidVariableScope)` | unit |
| `$attempt` bare reference | Valid YAML with bare `$attempt` | `Err(UnknownReferenceRoot)` | unit |
| `$attempt.number.extra` accessor | Valid YAML with `$attempt.number.foo` | `Err(UnknownReferenceRoot)` | unit |

### Integration Tests: Full Pipeline

| Scenario | Input | Expected Output | Layer |
|----------|-------|-----------------|-------|
| `$attempt.number` in `repeat` body `save` | Valid YAML | `Ok(WorkflowAst)` | integration |
| `$attempt.number` in `repeat` body `do` input | Valid YAML | `Ok(WorkflowAst)` | integration |
| `$attempt.number` in nested `repeat` body | Valid YAML with nested repeats | `Ok(WorkflowAst)` | integration |
| `$attempt.number` in `repeat` body `choose.condition` | Valid YAML | `Ok(WorkflowAst)` | integration |
| `$attempt.number` in `repeat` body expression | Valid YAML with expression | `Ok(WorkflowAst)` | integration |
| `$attempt.number` in `for_each` body | Valid YAML | `Err(InvalidVariableScope)` | integration |
| `$attempt.number` in `reduce` body | Valid YAML | `Err(InvalidVariableScope)` | integration |
| `$attempt.number` in `together` body | Valid YAML | `Err(InvalidVariableScope)` | integration |

### E2E Tests: CLI Boundary

| Scenario | Input | Expected Output | Layer |
|----------|-------|-----------------|-------|
| Compile workflow with `$attempt.number` in repeat body | CLI with valid YAML | Exit code 0 | e2e |
| Compile workflow with `$attempt.number` outside repeat | CLI with invalid YAML | Exit code non-zero + error message | e2e |

---

## 9. Error Contract

### New Error Variant Required

```rust
/// CompileError enum in vb_compile/src/lib.rs requires new variant:
InvalidVariableScope {
    /// The reference that violated scope rules.
    reference: Box<str>,
    /// The context where the reference appeared.
    context: &'static str,
    /// The valid contexts where this reference is allowed.
    valid_context: &'static str,
}
```

### Error Display Contract

```
$attempt.number is not valid in this context (vars).
It is only valid inside repeat body steps.
```

### Error Code Contract

`code()` returns `"INVALID_VARIABLE_SCOPE"`

---

## 10. Open Questions

1. **Q: Is `$attempt.number` the only restricted variable, or are there others?**
   A: For now, only `$attempt.number` is specified. Future restricted variables may follow the same pattern.

2. **Q: Should `$attempt` bare (without `.number`) be an error?**
   A: Yes — `$attempt` alone should be `UnknownReferenceRoot`, same as other invalid roots.

3. **Q: Should `$attempt.number` with additional accessor path (`$attempt.number.field`) be an error?**
   A: Yes — `$attempt.number` is a terminal value (u16), not an object. Additional accessors should be rejected.

4. **Q: Is there a maximum nesting depth for repeat blocks where `$attempt.number` is valid?**
   A: No explicit limit — any level of nesting where a `repeat` appears is valid.

5. **Q: Does `$attempt.number` work in `collect`/`for_each`/`reduce` body?**
   A: No — only in `repeat` bodies. These other constructs do not create attempt contexts.

---

## 11. Implementation Hints

### Files to Modify

1. **`crates/vb_compile/src/lib.rs`**: Add `CompileError::InvalidVariableScope` variant and update `code()` method

2. **`crates/vb_compile/src/references.rs`**: Add `validate_attempt_reference()` function and integrate into `validate_compile_reference()`. Requires passing scope context (boolean flag for "inside_repeat_body") through the traversal.

3. **`crates/vb_compile/src/type_taint.rs`**: May need to track `attempt` as a valid type source within repeat scope.

### Scope Tracking Design

The reference validation currently traverses AST without context. Scope tracking requires:

```rust
fn collect_references_from_steps_with_context(
    steps: &[StepAst],
    tables: &RefTables,
    inside_repeat_body: bool,  // NEW: scope context
    errors: &mut Vec<CompileError>,
) {
    for step in steps {
        let new_context = match &step.kind {
            StepKindAst::Repeat { .. } => true,  // bodies of repeat are valid
            _ => inside_repeat_body,
        };
        collect_references_from_step_kind_with_context(
            &step.kind,
            tables,
            new_context,
            errors,
        );
    }
}
```

### Validation Logic

```rust
fn validate_attempt_reference(reference: &str, inside_repeat_body: bool) -> Result<(), CompileError> {
    if reference == "$attempt.number" && !inside_repeat_body {
        return Err(CompileError::InvalidVariableScope {
            reference: Box::from(reference),
            context: "...",  // derived from current AST location
            valid_context: "repeat body steps",
        });
    }
    Ok(())
}
```
