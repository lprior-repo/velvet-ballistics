# Test Plan: LETHAL-1 — validate_taint SecretResultLeak Finish Pass-Through

## Summary

- **Bead**: LETHAL-1
- **Problem**: `vb_validate` and `vb_compile` incorrectly reject `SecretResultLeak` for `Finish` results. Section 47 mandates taint MUST pass through Finish outputs — no rejection.
- **Behaviors identified**: 6 core, 12 error variants, 8 proptest invariants, 4 fuzz targets, 2 Kani harnesses
- **Trophy allocation**: 18 unit / 12 integration / 2 e2e / 3 static
- **Proptest invariants**: 8
- **Fuzz targets**: 4
- **Kani harnesses**: 2
- **Mutation threshold**: ≥ 90%

---

## 1. Behavior Inventory

### Core Behaviors (vb_validate)

1. **`validate_taint` accepts secret-tainted Finish result** — taint must pass through, not reject
2. **`validate_taint` accepts direct `$secrets.*` reference in Finish result**
3. **`validate_taint` accepts secret-derived data via slot chain in Finish result**
4. **`validate_taint` accepts composite containing secret in Finish result**
5. **`validate_taint` rejects secret-tainted Finish when Section 47 is NOT yet implemented** (current buggy behavior — document for regression)
6. **`validate_taint` accepts clean Finish result always**

### Core Behaviors (vb_compile)

7. **`validate_public_result` accepts secret-tainted Finish result** — taint must pass through
8. **`validate_workflow_ast` accepts `$secrets.*` in Finish result**
9. **`validate_workflow_ast` accepts secret slot relay in Finish result**
10. **`validate_workflow_ast` accepts composite with secret in Finish result**

### Error Behaviors

11. **Untrusted data in non-Finish context returns `ValidationError::UntrustedInput`**
12. **Untrusted data in non-Finish context returns `CompileError::UntrustedInput`**
13. **Clean data passes `validate_taint` unconditionally**
14. **Unknown reference resolves as clean (not secret) in Finish**

---

## 2. Trophy Allocation

| Behavior | Layer | Rationale |
|----------|-------|-----------|
| validate_taint accepts secret Finish | Unit | Pure function, exhaustively testable |
| validate_taint rejects untrusted (non-Finish) | Unit | Exact error variant required |
| Taint merge commutativity | Unit | Pure mathematical property |
| Taint merge associativity | Unit | Pure mathematical property |
| Deep slot chain taint propagation | Unit | Exhaustive path coverage |
| Composite taint merge | Unit | Combinatorial coverage |
| compile + validate pipeline | Integration | Real YAML→AST→IR pipeline |
| End-to-end taint passthrough | E2E | Full workflow from YAML to compiled IR |
| Clippy / Rustc lint | Static | Zero-tolerance policy |

**Ratio**: 18 unit / 12 integration / 2 e2e / 3 static ≈ 51% unit / 34% integration / 6% e2e / 9% static

---

## 3. BDD Scenarios

### Behavior: Taint passes through Finish output — validate_taint accepts direct secret reference

**Function**: `fn validate_taint_accepts_secret_direct_reference_in_finish()`

Given: A workflow with one Finish step whose result is a direct reference to `$secrets.api_key`
And: The secret `api_key` is declared in the workflow's secrets list
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — NO error about SecretResultLeak
And: The taint propagates through without rejection

```rust
// Test scaffold (DO NOT IMPLEMENT — test-writer executes)
fn validate_taint_accepts_secret_direct_reference_in_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    // CURRENT BUG: validate_taint returns Err(ValidationError::SecretResultLeak)
    // EXPECTED: Ok(())
    assert_eq!(validate_taint(&wf), Ok(()));
}
```

---

### Behavior: Taint passes through Finish output — validate_taint accepts secret slot relay

**Function**: `fn validate_taint_accepts_secret_slot_relay_in_finish()`

Given: A workflow with a Save step capturing `$secrets.token` into slot 0
And: A Finish step whose result references slot 0
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — taint passed through via slot chain

---

### Behavior: Taint passes through Finish output — validate_taint accepts composite with secret

**Function**: `fn validate_taint_accepts_secret_composite_in_finish()`

Given: A workflow with a Save step producing a Composite containing `$secrets.password`
And: A Finish step whose result is the slot containing that composite
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — composite taint propagates

---

### Behavior: Taint passes through Finish output — validate_taint accepts deep slot chain

**Function**: `fn validate_taint_accepts_deep_secret_slot_chain_in_finish()`

Given: A 5-hop slot chain where each slot relays the previous, originating from `$secrets.db_password`
And: The Finish step references the final slot in the chain
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — taint propagates through all 5 hops

---

### Behavior: Taint passes through Finish output — validate_taint accepts clean Finish

**Function**: `fn validate_taint_accepts_clean_finish_always()`

Given: A workflow with a Finish step emitting a clean literal value
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())`

---

### Behavior: Untrusted data in non-Finish step returns UntrustedInput error

**Function**: `fn validate_taint_returns_untrusted_input_for_untrusted_data_in_save()`

Given: A workflow with a Save step containing untrusted input data
When: `validate_taint` is called on the workflow
Then: The result is `Err(ValidationError::UntrustedInput)` — exact variant required

**NOTE**: `ValidationError::UntrustedInput` does not currently exist in the enum. This test documents the REQUIRED error variant that must be added to handle untrusted (non-secret, non-clean) data. The current code only has `SecretResultLeak` for secrets.

---

### Behavior: validate_compile accepts secret-tainted Finish result

**Function**: `fn compile_accepts_secret_finish_result()`

Given: A YAML workflow with `$secrets.token` directly in the Finish result
When: `YamlCompiler::default().compile(source)` is called
Then: The compilation succeeds with `Ok(CompiledWorkflow)`
And: No `CompileError::SecretTaintLeak` is returned

---

### Behavior: compile pipeline preserves taint through AST→IR lowering

**Function**: `fn compile_preserves_secret_taint_through_lowering()`

Given: A YAML workflow with a secret-tainted Finish result
When: The workflow is compiled through `YamlCompiler::compile()`
Then: The resulting `CompiledWorkflow` preserves the secret taint in the Finish IR node
And: No `SecretTaintLeak` error is emitted

---

### Error Variant: validate_taint rejects secret input in non-Finish (current correct behavior — document regression target)

**Function**: `fn validate_taint_rejects_secret_input_in_save_slot_for_regression()`

Given: A workflow with a Save step referencing a secret input `$input.password`
And: The input `password` is declared as `is_secret: true`
And: The Finish step references the saved slot
When: `validate_taint` is called on the workflow
Then: The result is `Err(ValidationError::SecretResultLeak)`
And: The error is specifically `SecretResultLeak` (not a new `UntrustedInput` variant)

**Purpose**: This test documents the CURRENT correct behavior (secret in Save IS rejected). After the Section 47 fix, only Finish is affected — Save with secrets must STILL be rejected.

---

### Error Variant: validate_taint rejects secret var in non-Finish

**Function**: `fn validate_taint_rejects_secret_var_in_save_slot()`

Given: A workflow with a Save step referencing `$vars.super_secret`
And: The variable `super_secret` has secret taint
When: `validate_taint` is called on the workflow
Then: The result is `Err(ValidationError::SecretResultLeak)`

---

### Error Variant: compile rejects non-boolean choose condition

**Function**: `fn compile_rejects_non_boolean_choose_condition()`

Given: A YAML workflow with a Choose step whose condition is not boolean
When: `YamlCompiler::default().compile(source)` is called
Then: The result is `Err(CompileErrors(...))` containing `CompileError::TypeMismatch { field: "choose.condition", .. }`

---

### Error Variant: compile rejects uninitialized slot in Finish

**Function**: `fn compile_rejects_uninitialized_slot_in_finish()`

Given: A YAML workflow with a Finish step referencing an uninitialized slot
When: `YamlCompiler::default().compile(source)` is called
Then: The result is `Err(CompileErrors(...))` containing `CompileError::UnknownSlotType { field: "finish.result", slot: N }`

---

### Error Variant: validate_taint handles empty composite in Finish

**Function**: `fn validate_taint_accepts_empty_composite_in_finish()`

Given: A workflow with a Save step producing an empty Composite
And: A Finish step referencing that slot
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — empty composite is clean

---

### Error Variant: validate_taint handles uninitialized slot reference in Finish

**Function**: `fn validate_taint_accepts_uninitialized_slot_in_finish()`

Given: A workflow with a Finish step referencing a slot index beyond any written slot
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — uninitialized slot resolves as clean

---

### Error Variant: validate_taint unknown reference root resolves clean

**Function**: `fn validate_taint_unknown_reference_resolves_clean_in_finish()`

Given: A workflow with a Finish step referencing `$unknown_root.field`
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — unknown reference roots resolve as clean

---

### Error Variant: validate_taint non-$ reference resolves clean

**Function**: `fn validate_taint_non_dollar_reference_resolves_clean_in_finish()`

Given: A workflow with a Finish step referencing a plain string `not_a_reference`
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — non-$ references are treated as clean literals

---

### Error Variant: validate_taint reference without dot resolves clean

**Function**: `fn validate_taint_reference_without_dot_resolves_clean_in_finish()`

Given: A workflow with a Finish step referencing `$input` (no dot)
When: `validate_taint` is called on the workflow
Then: The result is `Ok(())` — incomplete references resolve as clean

---

## 4. Proptest Invariants

### Proptest: Taint.merge is commutative

**Function**: `fn taint_merge_is_commutative()`

Invariant: For all `a: Taint`, `b: Taint`, `a.merge(b) == b.merge(a)`
Strategy: `prop_compose!` over all `(Taint::Clean, Taint::Secret)` combinations
Anti-invariant: N/A — this is a mathematical property

```rust
proptest! {
    #[test]
    fn taint_merge_commutative(a: Taint, b: Taint) {
        prop_assert_eq!(a.merge(b), b.merge(a));
    }
}
```

---

### Proptest: Taint.merge is associative

Invariant: For all `a: Taint`, `b: Taint`, `c: Taint`, `a.merge(b).merge(c) == a.merge(b.merge(c))`
Strategy: Exhaustively test all 8 combinations of 3 Taint values
Anti-invariant: N/A

---

### Proptest: Taint.merge has Clean as identity

Invariant: For all `a: Taint`, `a.merge(Taint::Clean) == a` and `Taint::Clean.merge(a) == a`
Strategy: `prop_assert!` for both merge orientations
Anti-invariant: N/A

---

### Proptest: Secret contaminates everything

Invariant: For all `a: Taint`, `a.merge(Taint::Secret) == Taint::Secret` and `Taint::Secret.merge(a) == Taint::Secret`
Strategy: Test all Taint variants
Anti-invariant: N/A

---

### Proptest: validate_taint accepts any secret-tainted Finish workflow

Invariant: For all valid `WorkflowTypes` with a secret-tainted Finish result, `validate_taint(&wf) == Ok(())`
Strategy: `WorkflowTypesGenerator` producing workflows with `$secrets.*` in Finish
Anti-invariant: Workflow with secret in Save (not Finish) — must still return `Err(SecretResultLeak)`

```rust
proptest! {
    #[test]
    fn validate_taint_accepts_secret_finish_proptest(wf in workflow_with_secret_finish_strategy()) {
        prop_assert_eq!(validate_taint(&wf), Ok(()));
    }
}
```

---

### Proptest: validate_taint is deterministic

Invariant: For all `wf: WorkflowTypes`, `validate_taint(&wf)` returns the same result on repeated calls
Strategy: Repeat each workflow 100 times
Anti-invariant: N/A

---

### Proptest: Clean composite in Finish passes

Invariant: For all `values: Vec<TypedValue>` where all values are clean, `validate_taint` passes with those values in a Finish composite
Strategy: Generate clean-only composites
Anti-invariant: Composite containing any secret-tainted value

---

### Proptest: Deep slot chain propagates taint correctly

Invariant: For a chain of N slots (0 → 1 → ... → N-1 → Finish) where slot 0 contains a secret, the Finish result is always accepted
Strategy: `prop_compose!` over chain lengths 1..10
Anti-invariant: Chain where an intermediate slot breaks the chain

---

## 5. Fuzz Targets

### Fuzz Target: YAML workflow with secret-tainted Finish

**Function**: `yaml_secret_finish_fuzz`
**Input type**: `&[u8]` (raw YAML bytes)
**Risk**: Panic, logic error, wrong taint propagation
**Corpus seeds**:
- `finish: { result: $secrets.token }`
- `finish: { result: [ $secrets.a, $secrets.b ] }`
- `finish: { result: { key: $secrets.value } }`
- `finish: { result: 0 }` where slot 0 contains a secret

**Harness**:
```rust
#[cargo_mutants::fuzz]
fn yaml_secret_finish_fuzz(source: &[u8]) {
    let compiler = YamlCompiler::default();
    if let Ok(ast) = compiler.parse_ast(source) {
        if let Err(e) = validate_workflow_ast(&ast) {
            // Secret-tainted Finish must NOT produce SecretTaintLeak
            if matches!(e.0.first(), Some(CompileError::SecretTaintLeak { .. })) {
                panic!("SecretTaintLeak must not be raised for Finish results (Section 47)");
            }
        }
    }
}
```

---

### Fuzz Target: validate_taint with malformed WorkflowTypes

**Function**: `validate_taint malformed_workflow_fuzz`
**Input type**: Arbitrary `WorkflowTypes` struct via proptest
**Risk**: Panic on out-of-bounds slot access, infinite loop
**Corpus seeds**: Workflows with empty slots, max slot indices, deeply nested composites

---

### Fuzz Target: Taint merge associativity via random workflows

**Function**: `taint_merge_associativity_fuzz`
**Input type**: Three `Taint` values via `any::<(Taint, Taint, Taint)>()`
**Risk**: Associativity violation causing incorrect taint propagation
**Corpus seeds**: All 8 combinations of `(Clean, Secret)` triples

---

### Fuzz Target: Slot chain taint propagation depth

**Function**: `slot_chain_depth_fuzz`
**Input type**: `usize` chain length (1..100) + secret origin flag
**Risk**: Stack overflow, incorrect taint at depth boundaries
**Corpus seeds**: Chain lengths 0, 1, 2, 5, 10, 50, 100, u8::MAX, usize::MAX

---

## 6. Kani Harnesses

### Kani Harness: validate_step_taint never panics on valid WorkflowTypes

**Property**: For all valid `WorkflowTypes` and `Facts`, `validate_step_taint` returns `ValidationResult<()>`
**Bound**: Workflows with ≤ 1000 steps, ≤ 100 slots, ≤ 10 nesting depth
**Rationale**: `validate_step_taint` uses indexed slot access. Kani proves no out-of-bounds access regardless of workflow structure.

```rust
#[kani::proof]
fn validate_step_taint_no_panic_on_valid_input() {
    // Arbitrary but valid WorkflowTypes — use kani::any() for structural generation
    let workflow = kani::any::<WorkflowTypes>();
    let facts = Facts::build(&workflow);
    let mut slots = vec![None::<ValueFact>; workflow.steps.len()];
    // This must not panic
    let _ = validate_step_taint(&workflow, &facts, &mut slots);
}
```

---

### Kani Harness: Taint merge produces deterministic results

**Property**: For all `a: Taint`, `b: Taint`, `c: Taint`:
1. `a.merge(b)` is deterministically either `Clean` or `Secret`
2. If either argument is `Secret`, result is `Secret`
3. If both are `Clean`, result is `Clean`
**Bound**: Exhaustively check all 9 combinations of `(Clean, Secret, DerivedFromSecret)` (if applicable)
**Rationale**: Taint merge is the core propagation function — any nondeterminism is a critical bug.

```rust
#[kani::proof]
fn taint_merge_deterministic_and_proper() {
    let cases = [
        (Taint::Clean, Taint::Clean, Taint::Clean),
        (Taint::Clean, Taint::Secret, Taint::Secret),
        (Taint::Secret, Taint::Clean, Taint::Secret),
        (Taint::Secret, Taint::Secret, Taint::Secret),
    ];
    for (a, b, expected) in cases {
        prop_assert_eq!(a.merge(b), expected);
    }
}
```

---

## 7. Mutation Checkpoints

Critical mutations that MUST be caught:

| Mutation | Target Function | Catch Test |
|---------|----------------|-----------|
| Change `SecretResultLeak` rejection to `Ok(())` in `validate_step_taint` | `validate_step_taint` Finish arm | `validate_taint_accepts_secret_direct_reference_in_finish` |
| Remove taint merge in `resolve_composite` | `resolve_composite` | `validate_taint_rejects_secret_composite_in_finish` |
| Remove secret check in `validate_public_result` | `validate_public_result` | `compile_accepts_secret_finish_result` |
| Change `Taint::Secret` to `Taint::Clean` in `save_fact` | `save_fact` | `compile_rejects_secret_input_in_save` |
| Remove slot read in `expression_fact` | `expression_fact` Slot arm | `compile_rejects_uninitialized_slot_in_finish` |
| Change Clean+Secret merge result to Clean | `Taint::merge` | `taint_merge_propagates_secret` |

**Threshold**: ≥ 90% mutation kill rate on `validate_step_taint`, `validate_public_result`, `resolve_value`, `resolve_composite`, `Taint::merge`

---

## 8. Combinatorial Coverage Matrix

### Unit: validate_taint Finish outcomes

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|----------------|------------|
| Secret reference in Finish | `$secrets.*` | `Ok(())` | unit |
| Secret slot relay in Finish | Slot containing secret | `Ok(())` | unit |
| Secret composite in Finish | Composite with secret | `Ok(())` | unit |
| Deep secret chain in Finish | 5+ hop chain | `Ok(())` | unit |
| Clean literal in Finish | `TypedValue::Literal` | `Ok(())` | unit |
| Clean reference in Finish | `$input.user` (clean) | `Ok(())` | unit |
| Unknown reference in Finish | `$unknown.*` | `Ok(())` | unit |
| Non-$ reference in Finish | `not_a_ref` | `Ok(())` | unit |

### Unit: validate_taint non-Finish outcomes

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|----------------|------------|
| Secret in Save slot | `$secrets.*` → Slot | `Err(SecretResultLeak)` | unit |
| Secret input in Save | `$input.*` (secret) → Slot | `Err(SecretResultLeak)` | unit |
| Clean in Save | `$input.user` (clean) → Slot | `Ok(())` | unit |
| Secret in Choose condition | Secret in condition | `Ok(())` (taint doesn't propagate from Choose) | unit |

### Integration: compile pipeline

| Scenario | Input | Expected Output | Test Layer |
|----------|-------|----------------|------------|
| YAML with secret Finish | YAML `$secrets.token` in finish | `Ok(CompiledWorkflow)` | integration |
| YAML with clean Finish | YAML `$input.user` in finish | `Ok(CompiledWorkflow)` | integration |
| YAML with secret Save | YAML `$secrets.*` in save | `Err(CompileError::SecretTaintLeak)` | integration |
| YAML with malformed finish slot | Uninitialized slot | `Err(UnknownSlotType)` | integration |

### Error variants: CompileError

| Variant | Trigger | Exact assertion |
|---------|---------|----------------|
| `CompileError::SecretTaintLeak { field: "finish.result" }` | Secret in non-Finish | `matches!(err, SecretTaintLeak { field } if field == "finish.result")` |
| `CompileError::TypeMismatch { field: "choose.condition", expected: "boolean", found }` | Non-boolean condition | Exact `expected` and `found` strings |
| `CompileError::UnknownSlotType { field: "finish.result", slot: N }` | Uninitialized slot | Exact slot index |
| `CompileError::UnknownReferenceName { kind, name }` | Unknown reference | Exact `kind` and `name` |

---

## 9. Open Questions

1. **`ValidationError::UntrustedInput` does not exist** — the current enum has no `UntrustedInput` variant. The task description requires this exact variant for untrusted (non-secret, non-clean) data. Does this variant need to be added to the enum, or is `SecretResultLeak` the intended variant for all tainted data (including untrusted but non-secret data)?

2. **Section 47 taint levels**: The master spec mentions a three-level lattice `Clean < DerivedFromSecret < Secret`. The current `Taint` enum only has `Clean` and `Secret`. Should a `DerivedFromSecret` level be added for proper Section 47 compliance?

3. **vb_compile vs vb_validate parity**: `vb_validate` uses `ValidationError::SecretResultLeak` while `vb_compile` uses `CompileError::SecretTaintLeak`. Should these be unified into a single error concept with a shared `TaintError` type?

4. **End-to-end test scope**: Should the E2E tests cover the full pipeline (YAML → vb_compile → vb_validate → IR execution) or only the validation boundary?

---

## 10. Regression Target (Documenting Current Bug)

The following test documents the CURRENT (incorrect) behavior. After the Section 47 fix, these tests should FAIL, proving the bug existed:

```rust
// REGRESSION TEST — documents current bug (should FAIL after fix)
#[test]
fn regression_validate_taint_rejects_secret_finish_incorrectly() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    // This currently returns Err(SecretResultLeak) — BUG per Section 47
    // After fix, this should return Ok(())
    let result = validate_taint(&wf);
    // Document the bug: currently it's an error
    assert!(
        matches!(result, Err(ValidationError::SecretResultLeak)),
        "BUG: currently rejects secret Finish (Section 47 violation)"
    );
}
```

---

## 11. Test File Locations

| Crate | File | Tests to add |
|-------|------|-------------|
| `vb_validate` | `crates/vb_validate/src/type_taint_tests.rs` | All `validate_taint` BDD scenarios |
| `vb_compile` | `crates/vb_compile/src/type_taint/tests.rs` | All compile BDD scenarios |
| `workspace_tests` | `crates/workspace_tests/src/taint_passthrough.rs` | Integration + E2E tests |
| `fuzz` | `fuzz/src/taint_passthrough.rs` | All 4 fuzz targets |

---

## 12. References

- Section 47 of `velvet-ballistics-MASTER.md` — taint propagation rules
- `velvet-ballistics-MASTER.md:609` — `EngineSignal::Finished(SlotValue, Taint)` carries taint
- `velvet-ballistics-MASTER.md:658` — Finish taint contract
- `BIG-ASS-TESTING-TO-FIX.md` — existing bug documentation
- `crates/vb_validate/src/type_taint.rs` — current (buggy) implementation
- `crates/vb_compile/src/type_taint.rs` — current (buggy) implementation
