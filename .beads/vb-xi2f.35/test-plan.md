# Test Plan: ResourceContract Digest Coverage

## Summary

- **Bead**: `vb-xi2f.35` — P1: digest covers resource contract semantics
- **Contract**: 10 clauses (C1–C10), 9 acceptance criteria with proofs
- **Behaviors identified**: 47
- **Trophy allocation**:
  - Static analysis: 6 invariants (compile-time, lint, type-safety)
  - Unit / Calc: 19 behaviors (pure functions, encoding, digest)
  - Integration: 22 behaviors (compilation pipelines, validation, runtime)
  - E2E: 0 behaviors (no CLI/API boundary in scope; YAML parsing is P2)
- **Proptest invariants**: 8 (4 new, 4 repair/extension of existing)
- **Fuzz targets**: 1 (YAML contract parsing — P2 deferred, but target defined)
- **Kani harnesses**: 14 (existing, no new harnesses needed; execution pending CI)
- **Mutation threshold**: ≥ 90% kill rate for all new/modified functions

## 1. Behavior Inventory

### Group A: Digest Contract Binding (Clause C1)

| ID | Behavior |
|----|----------|
| A1 | `canonical_digest(source, contract)` produces a deterministic digest when given identical inputs |
| A2 | `canonical_digest(source, contract)` produces different digests when any single ResourceContract field differs |
| A3 | `canonical_digest(source, contract)` produces different digests when any combination of fields differs |
| A4 | Toggling `allows_secret_results` from true to false (or vice versa) changes the canonical digest |
| A5 | `canonical_digest` hashes all 17 contract fields in a stable, fixed order |
| A6 | `canonical_digest` domain-tags each contract field to prevent cross-field collisions |
| A7 | `canonical_digest` produces the same result regardless of which compilation path (part_05 or compile/mod) is used — given identical inputs |
| A8 | `canonical_digest(source, DEFAULT)` — digest using the DEFAULT contract is consistent (deterministic) |

### Group B: Single Canonical Type (Clause C2)

| ID | Behavior |
|----|----------|
| B1 | `workflow::ResourceContract` has exactly 17 fields — `max_transitions_per_tick` and `allows_secret_results` are present |
| B2 | `CompiledWorkflow::try_from_parts()` accepts `WorkflowParts` with a `ResourceContract` that contains `max_transitions_per_tick` and `allows_secret_results` |
| B3 | `CompiledWorkflow::resource_contract()` returns the full 17-field contract (not a truncated 16-field variant) |
| B4 | `validation/resource.rs` imports and operates on the canonical 17-field `ResourceContract` |
| B5 | No code path can construct or use the 16-field `compiled_workflow::ResourceContract` in lieu of the canonical type |

### Group C: Entry Point Contract (Clause C3)

| ID | Behavior |
|----|----------|
| C1 | `compile_source(source, contract)` accepts a `ResourceContract` parameter — it does not hardcode DEFAULT |
| C2 | `compile_source(source, DEFAULT)` produces a `CompiledWorkflow` whose `resource_contract()` equals DEFAULT |
| C3 | `compile_source(source, non_default_contract)` produces a `CompiledWorkflow` whose `resource_contract()` equals the input contract |
| C4 | Both compilation paths (`part_01::compile_source` and `compile/mod::compile_source`) accept and pass through the contract parameter |
| C5 | `compile_source_with_default(source)` exists and delegates to `compile_source(source, ResourceContract::DEFAULT)` — producing identical results |
| C6 | Changing only the contract (same source) produces `CompiledWorkflow` values with different `digest()` and `resource_contract()` |

### Group D: Taint Flag Sensitivity (Clause C4)

| ID | Behavior |
|----|----------|
| D1 | `encode_contract_bytes` produces different byte sequences for contracts differing only in `allows_secret_results` |
| D2 | `canonical_digest(source, contract_true)` ≠ `canonical_digest(source, contract_false)` when contracts differ only in `allows_secret_results` |
| D3 | The runtime enforcement `handle_ask_answer` references `allows_secret_results` from the same contract that was hashed into the digest |
| D4 | `RuntimeError::SecretResultNotAllowed` is returned when a secret-tainted answer arrives and `allows_secret_results` is false |
| D5 | No `SecretResultNotAllowed` is returned when `allows_secret_results` is true, regardless of answer taint |

### Group E: Validation Coverage (Clause C5)

| ID | Behavior |
|----|----------|
| E1 | `validate_resource_contract()` validates `max_steps` — when actual nodes exceed contract limit, returns `ResourceContractExceeded` with `resource: "max_steps"` |
| E2 | `validate_resource_contract()` validates `max_slots` — when actual slots exceed contract limit, returns `ResourceContractExceeded` with `resource: "max_slots"` |
| E3 | `validate_resource_contract()` validates `max_constants` — when actual constants exceed contract limit, returns `ResourceContractExceeded` with `resource: "max_constants"` |
| E4 | `validate_resource_contract()` validates `max_accessors` — when actual accessors exceed contract limit, returns `ResourceContractExceeded` with `resource: "max_accessors"` |
| E5 | `validate_resource_contract()` validates `max_expressions` — when actual expressions exceed contract limit, returns `ResourceContractExceeded` with `resource: "max_expressions"` |
| E6 | `validate_resource_contract()` validates `max_expr_stack` — when expression stack exceeds contract limit, returns `ResourceContractExceeded` with `resource: "max_expr_stack"` |
| E7 | `validate_contract_limit()` returns `ResourceContractTooLarge` when any contract field exceeds its hard system limit |
| E8 | `validate_contract_limit()` returns `ResourceContractExceeded` when `declared >= hard_limit` but `actual > declared` |
| E9 | `validate_budget()` validates `max_transitions_per_tick` — value of 0 returns `BudgetExceeded`; value > `HARD_MAX_TRANSITIONS_PER_TICK` returns `ResourceContractTooLarge` |
| E10 | `validate_budget()` validates `max_step_budget_per_tick` against `HARD_MAX_STEP_BUDGET_PER_TICK` |
| E11 | Validation errors carry specific `resource` identifiers (not generic messages) for precise diagnostics |

### Group F: Dual Path Consistency (Clause C6)

| ID | Behavior |
|----|----------|
| F1 | `part_05::canonical_digest(source, contract)` and `compile/mod::canonical_digest(source, contract)` produce identical digests for identical inputs |
| F2 | Both `canonical_digest` implementations call the shared `encode_contract_bytes` encoding function |
| F3 | Any future change to `canonical_digest` in one path without the matching change in the other is detectable by test |

### Group G: YAML Contract Parsing (Clause C7 — P2, test planning only)

| ID | Behavior |
|----|----------|
| G1 | Parser accepts `resource_contract` as a valid top-level YAML key |
| G2 | All 17 contract fields parse correctly from YAML |
| G3 | Unknown fields inside `resource_contract` section produce `YamlError::InvalidResourceContract` |
| G4 | Missing `resource_contract` section results in DEFAULT contract usage |
| G5 | Invalid field types (e.g., string for u16) produce `YamlError::InvalidResourceContract` with `field` and `reason` |

### Group H: Backward Compatibility (Clause C8)

| ID | Behavior |
|----|----------|
| H1 | Digest values differ from pre-fix values — this is documented as a one-time migration |
| H2 | `compute_policy_digest()` continues to operate on serialized contract bytes independently of canonical digest changes |
| H3 | Fresh compilations produce new digests that include the contract |

### Group I: Encoding Layer (supporting A1–A8, D1–D2)

| ID | Behavior |
|----|----------|
| I1 | `encode_contract_bytes(DEFAULT)` is deterministic — same input always produces same bytes |
| I2 | `encode_contract_bytes` uses all 17 field tags in a fixed order |
| I3 | `encode_contract_bytes` uses little-endian encoding for multi-byte values |
| I4 | `encode_contract_bytes` uses unique domain-tag strings for each field (no shared tags) |
| I5 | `encode_contract_bytes` produces different output for every distinct contract value |
| I6 | `encode_contract_bytes` buffer allocation does not panic for any valid contract input |

### Full Behavior Count

| Group | Count | Category |
|-------|-------|----------|
| A — Digest Binding | 8 | Integration + Unit |
| B — Single Type | 5 | Static + Unit |
| C — Entry Point | 6 | Integration |
| D — Taint Sensitivity | 5 | Unit + Integration |
| E — Validation | 11 | Integration |
| F — Dual Path | 3 | Integration |
| G — YAML (P2) | 5 | Integration (future) |
| H — Backward Compat | 3 | Integration |
| I — Encoding | 6 | Unit |
| **Total** | **47** | |

---

## 2. Trophy Allocation

### Layer Distribution

| Layer | Count | % | Rationale |
|-------|-------|---|-----------|
| **Static Analysis** | 6 | ~13% | Type system ensures contract is Copy/Eq/17-field; clippy gates on `unsafe`, panics, unwrap; compile-time checks on type divergence. Contract clause C2 type resolution is primarily a static-analysis concern. |
| **Unit / Calc** | 19 | ~40% | Exhaustive combinatorial on pure functions: `encode_contract_bytes`, `canonical_digest`, `validate_contract_limit`, `validate_expr_stack_contract`. Proptest for field sensitivity and determinism. |
| **Integration** | 22 | ~47% | Compilation pipeline using real `compile_source` and `CompiledWorkflow::try_from_parts`. Validation covering all 17 fields. Runtime enforcement tests. Dual-path equivalence. Real dependencies only (no mocks). |
| **E2E** | 0 | 0% | No CLI/API boundary in scope. YAML parsing is P2. E2E tested through `workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs` when YAML parsing is implemented. |

**Deviation justification**: Integration ratio (47%) is below the 60% target because this bead is heavily focused on pure functions (encoding/digest) where unit+proptest provides the strongest coverage. The 13% static allocation reflects the type-safety dimension of Clause C2 (single canonical type). This is appropriate given the bead's domain: contract-sensitive hashing and validation.

### Test Type Distribution within Layers

| Test Type | Count | Example |
|-----------|-------|---------|
| Inline `#[test]` unit | 19 | `encode_contract_bytes_is_deterministic`, per-field validation tests |
| Proptest | 8 | `proptest_contract_17_fields_independent_sensitivity`, `proptest_secret_results_digest_sensitivity` |
| Integration `#[test]` | 14 | `compile_source_preserves_contract`, dual-path equivalence |
| Kani (existing) | 14 | PO-K01 through PO-K14 — not new, planned for CI execution |
| Fuzz | 1 | `fuzz_yaml_resource_contract_parsing` — P2 deferred |
| Mutation | N/A | Checkpoints defined; run `cargo mutants` gate post-implementation |

---

## 3. BDD Scenarios

### 3.1 Group A: Digest Contract Binding

---

#### Behavior A1: Deterministic digest

```
Given: a valid WorkflowSource and a valid ResourceContract
When: canonical_digest(source, contract) is called twice with identical inputs
Then: both invocations return identical WorkflowDigest values
```

**Test name**: `fn canonical_digest_produces_identical_result_when_same_inputs_called_twice()`

**Edge cases**:
- DEFAULT contract → deterministic
- Contract with all fields set to extreme values (0, MAX) → deterministic
- Contract with `allows_secret_results: true` → deterministic
- Contract with `allows_secret_results: false` → deterministic

---

#### Behavior A2: Single-field sensitivity

```
Given: two ResourceContract values that differ in exactly one field (e.g., max_steps = 100 vs max_steps = 200)
When: canonical_digest(source, contract_A) and canonical_digest(source, contract_B) are computed
Then: the two digests are different
```

**Test name per field**:
1. `fn canonical_digest_differs_when_max_steps_differs()`
2. `fn canonical_digest_differs_when_max_slots_differs()`
3. `fn canonical_digest_differs_when_max_constants_differs()`
4. `fn canonical_digest_differs_when_max_accessors_differs()`
5. `fn canonical_digest_differs_when_max_expressions_differs()`
6. `fn canonical_digest_differs_when_max_expr_stack_differs()`
7. `fn canonical_digest_differs_when_max_step_budget_per_tick_differs()`
8. `fn canonical_digest_differs_when_max_transitions_per_tick_differs()`
9. `fn canonical_digest_differs_when_max_input_bytes_differs()`
10. `fn canonical_digest_differs_when_max_output_bytes_differs()`
11. `fn canonical_digest_differs_when_max_blob_bytes_differs()`
12. `fn canonical_digest_differs_when_max_ipc_payload_bytes_differs()`
13. `fn canonical_digest_differs_when_max_retry_attempts_differs()`
14. `fn canonical_digest_differs_when_max_fanout_differs()`
15. `fn canonical_digest_differs_when_max_collect_items_differs()`
16. `fn canonical_digest_differs_when_max_queue_depth_differs()`
17. `fn canonical_digest_differs_when_max_journal_batch_bytes_differs()`

**Edge cases**:
- Field at minimum vs minimum+1 (boundary)
- Field at maximum vs maximum-1 (boundary)
- Field value wrapping (e.g., u16::MAX → 0 for wrapping_add)
- For `max_expr_stack` (u8): 0→1, 254→255 (all boundary edges)
- `allows_secret_results`: true→false, false→true (covered in D2)

**Note on coverage approach**: Rather than writing 17 near-identical test functions, implement a proptest that iterates through all 17 fields and toggles each, OR implement a single proptest with a `FieldSelector` enum. The plan specifies 17 test NAMES to ensure traceability, but the implementation may use a single parameterized proptest to achieve coverage. See §4 Proptest Invariants for the recommended approach.

---

#### Behavior A3: Multi-field sensitivity

```
Given: two ResourceContract values that differ in multiple fields simultaneously
When: canonical_digest is computed for each
Then: the two digests are different
```

**Test name**: `fn canonical_digest_differs_when_multiple_fields_differ()`

**Edge cases**:
- All 17 fields randomized — still different
- Most fields identical, 2-3 differing — still different
- Fields that visually "cancel" in naive encoding (e.g., field A's value equals field B's tag string) — domain tags prevent collision

---

#### Behavior A4: allows_secret_results digest sensitivity

```
Given: two ResourceContract values identical except allows_secret_results = true vs false
When: canonical_digest is computed for each
Then: the digests differ
```

**Test name**: `fn canonical_digest_differs_when_allows_secret_results_toggled()`

---

#### Behavior A5: Stable field ordering

```
Given: a ResourceContract and the encoding function
When: encode_contract_bytes is called twice with the same contract
Then: the byte sequences are identical (fields appear in the same order)
```

**Test name**: `fn encode_contract_bytes_preserves_field_ordering_across_calls()`

---

#### Behavior A6: Domain-tagged fields

```
Given: two different contract fields whose normalized values could collide (e.g., max_steps = 42 and max_slots = 42)
When: encode_contract_bytes is called
Then: the output for each field differs in at least the tag-prefix bytes
```

**Test name**: `fn encode_contract_bytes_domain_tags_prevent_cross_field_collision()`

**Strategy**: Assert that `encode_contract_bytes` for contract A (max_steps=42, max_slots=0) ≠ encode_contract_bytes for contract B (max_steps=0, max_slots=42). Since tags differ, the encoding differs even though the non-tag bytes might overlap.

---

#### Behavior A7: Dual-path equivalence

```
Given: a WorkflowSource and a ResourceContract
When: part_05::canonical_digest(source, contract) and compile::mod::canonical_digest(source, contract) are both called
Then: both return identical WorkflowDigest values
```

**Test name**: `fn dual_compilation_paths_produce_identical_canonical_digest()`

**NOTE (PF-BR-001)**: This is the TRUE dual-path test. The existing `proptest_dual_path_equivalence.rs` tests determinism (same function × 2), NOT dual-path equivalence. This test must call both `mod_compile_lowering::part_05::canonical_digest` and `compile::mod::canonical_digest` independently and compare their outputs.

**Edge cases**:
- DEFAULT contract → identical
- Non-default contract with random fields → identical
- Contract with boundary values → identical
- Multiple random contract pairs → identical

---

#### Behavior A8: DEFAULT digest determinism

```
Given: a WorkflowSource and ResourceContract::DEFAULT
When: canonical_digest is called multiple times
Then: all results are identical
```

**Test name**: `fn canonical_digest_is_deterministic_with_default_contract()`

---

### 3.2 Group B: Single Canonical Type

---

#### Behavior B1: Canonical type has 17 fields

```
Given: the workflow::ResourceContract type definition
When: the struct is inspected (compile-time, test-time reflection, or Kani)
Then: it has exactly 17 fields including max_transitions_per_tick and allows_secret_results
```

**Test name**: `fn resource_contract_canonical_type_has_17_fields()`

**Implementation approach**: Use a compile-time assertion or a test that constructs the struct with all 17 fields and verifies that field access compiles. Any missing field results in a compile error, which is itself the test.

```rust
// This test PASSES if it compiles:
let c = ResourceContract {
    max_steps: 1,
    max_slots: 1,
    max_constants: 1,
    max_accessors: 1,
    max_expressions: 1,
    max_expr_stack: 1,
    max_step_budget_per_tick: 1,
    max_transitions_per_tick: 1,     // If this line doesn't compile, test FAILS
    max_input_bytes: 1,
    max_output_bytes: 1,
    max_blob_bytes: 1,
    max_ipc_payload_bytes: 1,
    max_retry_attempts: 1,
    max_fanout: 1,
    max_collect_items: 1,
    max_queue_depth: 1,
    max_journal_batch_bytes: 1,
    allows_secret_results: false,    // If this line doesn't compile, test FAILS
    ..ResourceContract::DEFAULT
};
```

---

#### Behavior B2: CompiledWorkflow accepts 17-field contract

```
Given: WorkflowParts with a 17-field ResourceContract (including max_transitions_per_tick and allows_secret_results)
When: CompiledWorkflow::try_from_parts(parts) is called
Then: the operation succeeds (Ok) and the contract is preserved
```

**Test name**: `fn compiled_workflow_accepts_17_field_resource_contract()`

---

#### Behavior B3: resource_contract() returns full 17 fields

```
Given: a CompiledWorkflow constructed with a non-default ResourceContract
When: resource_contract() is called
Then: the returned value matches the input contract in all 17 fields
```

**Test name**: `fn compiled_workflow_resource_contract_returns_full_17_field_contract()`

---

#### Behavior B4: Validation imports canonical type

```
Given: the source file crates/vb_core/src/validation/resource.rs
When: the import statement is inspected
Then: it imports ResourceContract from crate::workflow, NOT from crate::compiled_workflow
```

**Test name**: N/A — this is a **static-only** check. Enforced by:
1. Removing or deprecating `compiled_workflow::ResourceContract`
2. `cargo check` fails if `validation/resource.rs` still references the old path
3. The `resource_contract_canonical_type_has_17_fields` unit test above serves as a proxy: if validation imports the 16-field type, tests that construct a full 17-field contract and pass it through validation will fail to compile

---

#### Behavior B5: 16-field duplicate is inaccessible

```
Given: the compiled_workflow::ResourceContract type is either deleted or re-exports the canonical type
When: code attempts to use compiled_workflow::ResourceContract for anything other than the canonical type
Then: compilation fails OR both types are the same (structurally identical, same module)
```

**Test name**: N/A — **static-only** check. If `compiled_workflow::ResourceContract` is deleted, any existing usage produces a compile error. If it's re-exported as a type alias to `workflow::ResourceContract`, then both are the same type and no divergence is possible.

**Resolution approach**: Delete `compiled_workflow::ResourceContract`. Update `CompiledWorkflow` and `WorkflowParts` in `compiled_workflow.rs` to use `crate::workflow::ResourceContract`. Update all imports.

---

### 3.3 Group C: Entry Point Contract

---

#### Behavior C1: compile_source accepts contract parameter

```
Given: a valid WorkflowSource and a ResourceContract
When: compile_source(&source, contract) is called
Then: it compiles successfully (this is a type-check + behavior test)
```

**Test name**: `fn compile_source_accepts_contract_parameter()`

**Implementation**: Simply verify that `compile_source(&source, contract)` compiles and runs. If the signature were `compile_source(&source)` with hardcoded DEFAULT, this would already be caught by the existing proptest suite (which now passes a contract parameter).

---

#### Behavior C2: compile_source with DEFAULT contract

```
Given: a valid WorkflowSource and ResourceContract::DEFAULT
When: compile_source(&source, DEFAULT) is called
Then: the resulting CompiledWorkflow has resource_contract() == DEFAULT
```

**Test name**: `fn compile_source_with_default_contract_preserves_default()`

---

#### Behavior C3: compile_source preserves non-default contract

```
Given: a valid WorkflowSource and a non-default ResourceContract (e.g., max_steps = 42)
When: compile_source(&source, contract) is called
Then: the resulting CompiledWorkflow has resource_contract() equal to the input contract
```

**Test name**: `fn compile_source_preserves_non_default_contract_after_compilation()`

**Edge cases**:
- All fields set to non-default values → preserved
- Only one field non-default → preserved
- `max_transitions_per_tick` non-default → preserved
- `allows_secret_results` non-default → preserved

---

#### Behavior C4: Both compilation paths accept contract

```
Given: a valid WorkflowSource and a non-default ResourceContract
When: compile_source is called via mod_compile_lowering::part_01::compile_source
  and compile_source is called via compile::mod::compile_source
Then: both paths preserve the contract identically
```

**Test name**: `fn both_compilation_paths_preserve_resource_contract()`

---

#### Behavior C5: compile_source_with_default equivalence

```
Given: a valid WorkflowSource
When: compile_source_with_default(&source) is called
  and compile_source(&source, ResourceContract::DEFAULT) is called
Then: both produce CompiledWorkflow values with identical digest() and resource_contract()
```

**Test name**: `fn compile_source_with_default_equivalent_to_explicit_default()`

**NOTE (PF-BR-002)**: `compile_source_with_default` does NOT yet exist as an API. It must be implemented before this test can be written. The existing `proptest_with_default_equivalence.rs` tests determinism, not with_default vs explicit-DEFAULT equivalence.

---

#### Behavior C6: Different contracts → different CompiledWorkflow

```
Given: a valid WorkflowSource and two different ResourceContract values
When: compile_source(&source, contract_a) and compile_source(&source, contract_b) are called
Then: the two CompiledWorkflow values have different digest() AND different resource_contract()
```

**Test name**: `fn compile_source_produces_different_digest_and_contract_when_contract_differs()`

---

### 3.4 Group D: Taint Flag Sensitivity

---

#### Behavior D1: Encoding differs for allows_secret_results toggle

```
Given: two ResourceContract values differing only in allows_secret_results (true vs false)
When: encode_contract_bytes is called on each
Then: the byte sequences differ
```

**Test name**: `fn encode_contract_bytes_differs_when_allows_secret_results_toggled()`

---

#### Behavior D2: Digest differs for allows_secret_results toggle

```
Given: a WorkflowSource and two ResourceContract values differing only in allows_secret_results
When: canonical_digest is called with each contract
Then: the digests differ
```

**Test name**: `fn canonical_digest_differs_when_allows_secret_results_toggled()`

---

#### Behavior D3: Runtime references same allows_secret_results

```
Given: a compile_source that hashes allows_secret_results into the digest
  AND a runtime that checks contract.allows_secret_results
When: both reference the same ResourceContract field (same source type)
Then: the digest reflects the runtime behavior
```

**Test name**: N/A — **integration check**. Verified by the existing proptest suite + code audit confirming `chunk_002.rs` and `contract_encoding.rs` both reference `crate::workflow::ResourceContract::allows_secret_results`.

---

#### Behavior D4: SecretResultNotAllowed enforcement

```
Given: a workflow with allows_secret_results = false in its ResourceContract
  AND a runtime answer with Taint::Secret
When: the runtime evaluates the answer
Then: RuntimeError::SecretResultNotAllowed is returned
```

**Test name**: `fn runtime_returns_secret_result_not_allowed_when_flag_false_and_answer_is_secret()`

**Layer**: Integration (vb_runtime). Requires constructing or simulating a runtime scenario. For unit-level coverage, test the guard condition in isolation.

**Fallback**: If full runtime mockup is not feasible, the Kani harness `prove_secret_result_not_allowed_enforcement` (PO-K09) provides bounded formal coverage.

---

#### Behavior D5: No SecretResultNotAllowed when flag is true

```
Given: a workflow with allows_secret_results = true in its ResourceContract
  AND a runtime answer with Taint::Secret
When: the runtime evaluates the answer
Then: no SecretResultNotAllowed error is returned (answer is accepted)
```

**Test name**: `fn runtime_accepts_secret_answer_when_allows_secret_results_true()`

---

### 3.5 Group E: Validation Coverage

---

#### Behavior E1–E6: Per-field contract exceeded

Each test follows the same pattern. Exemplar for E1:

```
Given: WorkflowParts with 2 nodes and ResourceContract with max_steps = 1
When: CompiledWorkflow::try_from_parts(parts) is called
Then: Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" }) is returned
```

**Test names**:
1. `fn validation_rejects_nodes_exceeding_max_steps_contract()`
2. `fn validation_rejects_slots_exceeding_max_slots_contract()`
3. `fn validation_rejects_constants_exceeding_max_constants_contract()`
4. `fn validation_rejects_accessors_exceeding_max_accessors_contract()`
5. `fn validation_rejects_expressions_exceeding_max_expressions_contract()`
6. `fn validation_rejects_expr_stack_exceeding_max_expr_stack_contract()`

**Edge cases**:
- At exact limit → OK (no error)
- At exact limit + 1 → ResourceContractExceeded
- At exact limit + large delta → ResourceContractExceeded
- actual = 0, declared = 0 → OK (vacuous truth)

---

#### Behavior E7: Hard limit exceeded

```
Given: WorkflowParts with a ResourceContract where max_steps > MAX_STEPS_PER_WORKFLOW
When: CompiledWorkflow::try_from_parts(parts) is called
Then: Err(WorkflowError::ResourceContractTooLarge { resource: "max_steps" }) is returned
```

**Test names** (one per validated dimension):
1. `fn validation_rejects_contract_when_max_steps_exceeds_hard_limit()`
2. `fn validation_rejects_contract_when_max_slots_exceeds_hard_limit()`
3. `fn validation_rejects_contract_when_max_constants_exceeds_hard_limit()`
4. `fn validation_rejects_contract_when_max_accessors_exceeds_hard_limit()`
5. `fn validation_rejects_contract_when_max_expressions_exceeds_hard_limit()`
6. `fn validation_rejects_contract_when_max_expr_stack_exceeds_hard_limit()`
7. `fn validation_rejects_contract_when_max_transitions_per_tick_exceeds_hard_limit()`
8. `fn validation_rejects_contract_when_max_step_budget_per_tick_exceeds_hard_limit()`

---

#### Behavior E8: declared ≥ hard_limit but actual > declared

```
Given: WorkflowParts with a ResourceContract where max_steps = MAX_STEPS_PER_WORKFLOW (at limit)
  AND actual nodes exceed max_steps
When: CompiledWorkflow::try_from_parts(parts) is called
Then: Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" }) is returned
  (NOT ResourceContractTooLarge — the declared value is within hard limits)
```

**Test name**: `fn validation_rejects_exceeded_contract_even_when_declared_at_hard_limit()`

---

#### Behavior E9: max_transitions_per_tick validation

```
Given: WorkflowParts with max_transitions_per_tick = 0
When: validate_budget is called
Then: returns Err(BudgetExceeded) because zero transitions is invalid

Given: WorkflowParts with max_transitions_per_tick > HARD_MAX_TRANSITIONS_PER_TICK
When: validate_budget is called
Then: returns Err(ResourceContractTooLarge)
```

**Test names**:
1. `fn validate_budget_rejects_zero_transitions_per_tick()`
2. `fn validate_budget_rejects_transitions_per_tick_exceeding_hard_limit()`

---

#### Behavior E10: max_step_budget_per_tick validation

```
Given: WorkflowParts with max_step_budget_per_tick > HARD_MAX_STEP_BUDGET_PER_TICK
When: validate_budget is called
Then: returns Err(ResourceContractTooLarge)
```

**Test name**: `fn validate_budget_rejects_step_budget_exceeding_hard_limit()`

---

#### Behavior E11: Error variant specificity

```
Given: any validation error scenario
When: an error is returned
Then: the error variant includes a specific resource identifier string (not a generic message)
  AND the error variant is an exact match (ResourceContractExceeded vs ResourceContractTooLarge as appropriate)
```

**Test name**: `fn validation_errors_carry_specific_resource_identifiers()`

**Implementation**: Meta-test that iterates through known error-producing scenarios and asserts `resource` field values match expected constants. Alternatively, ensure each E1–E10 test already asserts the exact error variant and resource identifier.

---

### 3.6 Group F: Dual Path Consistency

---

#### Behavior F1: Cross-path digest equality

```
Given: a WorkflowSource and a ResourceContract
When: canonical_digest is computed via part_05.rs AND compile/mod.rs independently
Then: both results are identical WorkflowDigest values
```

**Test name**: `fn dual_path_canonical_digest_equivalence()`

**NOTE (PF-BR-001)**: This is the true dual-path test. The existing proptest does NOT test dual paths — it calls `compile_source` twice (determinism). This test must:

```rust
// DO: Call both paths independently
let digest_part05 = vb_compile::mod_compile_lowering::part_05::canonical_digest(&source, contract);
let digest_compile_mod = vb_compile::compile::mod::canonical_digest(&source, contract);
assert_eq!(digest_part05, digest_compile_mod);
```

**Edge cases**:
- DEFAULT contract → identical
- Contract with all fields randomized → identical
- Multiple random (source, contract) pairs → identical across all

**Access note**: If `canonical_digest` is `pub(crate)` in both paths, this integration test must be placed within `crates/vb_compile/tests/` (which has access to `pub(crate)` items in the crate under test).

**Proptest version**: Extend `proptest_dual_path_equivalence.rs` to call both paths independently.

---

#### Behavior F2: Both paths use shared encoding

```
Given: the source of both canonical_digest implementations
When: the implementations are inspected
Then: both call vb_core::contract_encoding::encode_contract_bytes (not independent copy-paste)
```

**Test name**: N/A — **static audit check**. Verified via code review and grep. A mutation that removes the `encode_contract_bytes` call from one path would be caught by F1.

---

#### Behavior F3: Drift detection

```
Given: the dual-path equivalence test (F1)
When: a developer modifies canonical_digest in only one compilation path
Then: the F1 test fails, alerting to the drift
```

**Test name**: Same as F1 — `dual_path_canonical_digest_equivalence` IS the drift detection.

**Defense in depth**: Additionally, a build script or xtask could grep both implementations and assert structural equivalence of the hash-pipeline calls.

---

### 3.7 Group I: Encoding Layer

---

#### Behavior I1: Encoding determinism

```
Given: a ResourceContract
When: encode_contract_bytes is called twice
Then: both calls return identical Vec<u8>
```

**Test name**: `fn encode_contract_bytes_is_deterministic()`

---

#### Behavior I2: All 17 field tags present

```
Given: encode_contract_bytes output for any ResourceContract
When: the output bytes are scanned
Then: all 17 field tag strings appear in order: "max_steps", "max_slots", ..., "allows_secret_results"
```

**Test name**: `fn encode_contract_bytes_contains_all_17_field_tags_in_order()`

---

#### Behavior I3: Little-endian encoding

```
Given: a ResourceContract with a known value (e.g., max_steps = 0x0102)
When: encode_contract_bytes is called
Then: the value bytes appear in little-endian order (0x02, 0x01 for u16)
```

**Test name**: `fn encode_contract_bytes_uses_little_endian_for_multi_byte_values()`

---

#### Behavior I4: Unique domain tags

```
Given: the tag strings used in encode_contract_bytes
When: all 17 tag strings are collected
Then: each tag string is unique (no duplicates)
```

**Test name**: `fn encode_contract_bytes_field_tags_are_unique()`

---

#### Behavior I5: Encoding injectivity

```
Given: two distinct ResourceContract values (contract_a ≠ contract_b)
When: encode_contract_bytes is called on each
Then: the outputs differ: encode_a ≠ encode_b
```

**Test name**: `fn encode_contract_bytes_is_injective_for_distinct_contracts()`

**Coverage approach**: This is primarily a proptest invariant (§4, PI-02). Unit test covers deterministic cases; proptest covers random pairs.

---

#### Behavior I6: No panic on valid input

```
Given: any valid ResourceContract (including extreme values)
When: encode_contract_bytes is called
Then: it returns a Vec<u8> without panicking
```

**Test name**: `fn encode_contract_bytes_does_not_panic_for_extreme_contract_values()`

**Edge cases**:
- All fields set to 0
- All fields set to their type's MAX
- Mixed extreme values
- DEFAULT contract
- All bool values

---

## 4. Proptest Invariants

### PI-01: Per-field digest sensitivity (all 17 fields)

**Invariant**: For every field `f` in ResourceContract (all 17), changing only `f` changes `canonical_digest(source, contract)`.

**Strategy**: Generate a random `ResourceContract` base, clone it, modify one field by a random non-zero delta (or toggle for bool), assert digests differ. Do this for each of the 17 fields independently. Use a `FieldSelector` enum so one proptest covers all fields.

**Anti-invariant**: Two contracts differing only in a non-hashed (hypothetical) field → digest identical. Since no such field exists, there is no anti-invariant.

**Coverage target**: ≥ 1000 cases per field (≥ 17,000 total cases).

**Relationship to existing tests**: Existing `proptest_contract_field_sensitivity.rs` covers only 2 fields (`max_steps`, `max_slots`) + `allows_secret_results`. This invariant extends to all 17 fields (PF-BR-003 remediation).

**Edge cases**:
- `max_expr_stack` (u8): test at 0→1, 127→128, 254→255
- `allows_secret_results` (bool): toggle true↔false
- Wrapping boundaries: u16::MAX → 0, u64::MAX → 0

---

### PI-02: Encoding injectivity

**Invariant**: For all contract_a ≠ contract_b, `encode_contract_bytes(contract_a) ≠ encode_contract_bytes(contract_b)`.

**Strategy**: Generate two random ResourceContract values independently. If they are unequal, assert their encodings differ. Use the full 17-field randomization.

**Anti-invariant**: Two contracts that are equal should produce equal encodings (trivially true; cover as sanity check).

**Coverage target**: ≥ 5000 random pairs.

**Relationship to existing tests**: Covered by `proptest_contract_field_sensitivity.rs` `proptest_all_fields_randomized_digest_differs` (2-field) and `proptest_multi_field_differs` (8-field). Extend to full 17-field randomization.

---

### PI-03: Digest determinism at scale

**Invariant**: `canonical_digest(source, contract)` is a pure function — same inputs always produce same output. Proptest verifies this across thousands of random (source, contract) pairs.

**Strategy**: Generate random source YAML strings and random contracts. Call `canonical_digest` twice. Assert equality.

**Anti-invariant**: Two calls with intentionally different inputs should produce different digests (tested separately).

**Coverage target**: ≥ 5000 cases.

**Note**: This replaces the THREE overlapping determinism proptests (PO-P04, PO-P05, PO-P06) per PF-BR-005. The existing determinism tests are consolidated into a single proptest.

---

### PI-04: Dual-path digest equivalence

**Invariant**: `part_05::canonical_digest(source, contract)` == `compile::mod::canonical_digest(source, contract)` for all inputs. **(PF-BR-001 remediation)**

**Strategy**: Generate random (source, contract) pairs. Call both `canonical_digest` implementations independently. Assert equality.

**Anti-invariant**: The two paths truly differ only if they compute different logic — this invariant says they never should.

**Coverage target**: ≥ 2000 random pairs.

**Access constraint**: `canonical_digest` is `pub(crate)` in both modules. The integration test must live in `crates/vb_compile/tests/` which has crate-internal access.

---

### PI-05: compile_source_with_default equivalence

**Invariant**: `compile_source_with_default(source)` produces the same digest and contract as `compile_source(source, ResourceContract::DEFAULT)`. **(PF-BR-002 remediation)**

**Strategy**: Generate random source YAML strings. Call both APIs (once `compile_source_with_default` is implemented). Assert identical `digest()` and `resource_contract()`.

**Anti-invariant**: N/A — both should always produce identical results.

**Coverage target**: ≥ 1000 cases.

**Precondition**: `compile_source_with_default` must be implemented first (see Closure Obligation 4 in proof-to-rust-map.md).

---

### PI-06: Entry point contract preservation

**Invariant**: `compile_source(source, contract)` → `workflow.resource_contract() == contract` for all valid (source, contract) pairs.

**Strategy**: Generate random source YAML and random contracts. Compile, assert contract roundtrips.

**Coverage target**: ≥ 2000 cases.

**Already exists**: `proptest_entry_point_contract.rs` provides this. Extend coverage to more fields.

---

### PI-07: Validation rejects all 17 exceeded dimensions

**Invariant**: For every contract field that has a validator, setting the contract's declared value lower than the actual resource count produces `WorkflowError::ResourceContractExceeded` with the correct `resource` identifier.

**Strategy**: For each of the validated fields, construct `WorkflowParts` with actual count = declared + 1 or declared = 0. Assert exact error variant and resource name. This is better as a unit test matrix than a proptest, but the invariant is stated here for completeness.

**Coverage target**: ≥ 1 case per validated dimension = ≥ 8 dimensions.

---

### PI-08: Validation rejects all 8 hard-limit exceeded dimensions

**Invariant**: For every field with a hard system limit, setting the contract's declared value > hard limit produces `WorkflowError::ResourceContractTooLarge` with the correct `resource` identifier.

**Strategy**: Construct contracts where each dimension exceeds its hard limit. Assert exact error variant.

**Coverage target**: ≥ 1 case per hard-limited dimension = ≥ 8 dimensions.

---

### Proptest Coverage Summary

| Invariant | New/Existing | Cases | Test File |
|-----------|:---:|---:|-----------|
| PI-01: 17-field sensitivity | **NEW** (extends existing) | 17,000+ | `proptest_contract_field_sensitivity.rs` (extend) |
| PI-02: Encoding injectivity | **NEW** (extends existing) | 5,000+ | `proptest_contract_field_sensitivity.rs` (extend) |
| PI-03: Determinism | Consolidate existing | 5,000+ | Consolidate PO-P04/P05/P06 into one file |
| PI-04: Dual-path equivalence | **NEW** (PF-BR-001) | 2,000+ | `proptest_dual_path_equivalence.rs` (rewrite) |
| PI-05: with_default equivalence | **NEW** (PF-BR-002) | 1,000+ | `proptest_with_default_equivalence.rs` (rewrite) |
| PI-06: Entry point preservation | Existing, extend | 2,000+ | `proptest_entry_point_contract.rs` (extend) |
| PI-07: Validation exceed | Test matrix, not proptest | 8 dimensions | New inline `#[test]` or existing `workflow/tests.rs` |
| PI-08: Validation hard limit | Test matrix, not proptest | 8 dimensions | New inline `#[test]` or existing `workflow/tests.rs` |

---

## 5. Fuzz Targets

### FZ-01: YAML resource contract parsing

**Target**: `vb_yaml::parse_workflow_source` (when extended with `resource_contract` section)

**Input type**: `&[u8]` — arbitrary bytes interpreted as YAML

**Risk classes**:
- **Panic**: Malformed YAML with deep nesting, huge numbers, invalid UTF-8 in field values
- **Logic error**: Parser silently accepting unknown fields inside `resource_contract`
- **OOM**: Extremely large contract values (u64::MAX for blob_bytes repeated)
- **Type confusion**: Integer overflow when parsing u16/u32/u64 from YAML numbers

**Corpus seeds**:
1. `resource_contract: {}` (empty — use DEFAULT)
2. `resource_contract: { max_steps: 100 }` (single field override)
3. `resource_contract: { max_steps: 100, allows_secret_results: true }` (multiple fields)
4. `resource_contract: { max_steps: -1 }` (negative integer — should reject)
5. `resource_contract: { max_steps: 99999999999999999999 }` (overflow)
6. `resource_contract: { max_steps: "one hundred" }` (wrong type)
7. `resource_contract: { unknown_field: 42 }` (unknown field)
8. All 17 fields specified (maximal valid contract)

**Status**: Deferred to P2 per waiver WC-001. Target definition is included here for completeness. Implement in `fuzz/fuzz_targets/yaml_resource_contract.rs`.

---

## 6. Kani Harnesses

All 14 Kani harnesses are **already written** (see `crates/vb_compile/src/kani_resource_contract_*.rs` and `crates/vb_core/src/kani_resource_contract_*.rs`). The test plan does not add new Kani harnesses. Execution is pending Kani toolchain availability on CI.

| Obligation | Harness | Crate | Status |
|-----------|---------|-------|--------|
| PO-K01 | `prove_digest_determinism` | vb_compile | ⚠️ CI pending (blake3) |
| PO-K01e | `prove_contract_encoding_determinism` | vb_compile | ✅ Approved |
| PO-K02 | `prove_single_field_changes_digest` | vb_compile | ⚠️ CI pending (blake3) |
| PO-K02e | `prove_single_field_changes_encoding` | vb_compile | ✅ Approved |
| PO-K03u32 | `prove_no_cross_field_collision_u32` | vb_compile | ✅ Approved |
| PO-K03u64 | `prove_no_cross_field_collision_u64` | vb_compile | ✅ Approved |
| PO-K03b3 | `prove_no_cross_field_collision` | vb_compile | ⚠️ CI pending (blake3) |
| PO-K04e | `prove_contract_encoding_is_stable` | vb_compile | ✅ Approved |
| PO-K04b3 | `prove_migration_digest_relationship` | vb_compile | ⚠️ CI pending (blake3) |
| PO-K05 | `prove_canonical_contract_has_17_fields` | vb_core | ⚠️ CI pending |
| PO-K06 | `prove_type_identity_across_paths` | vb_core | ⚠️ CI pending |
| PO-K07e | `prove_non_default_contract_encoding_differs` | vb_compile | ✅ Approved |
| PO-K07b3 | `prove_contract_survives_compilation` | vb_compile | ⚠️ CI pending |
| PO-K08 | `prove_secret_results_changes_digest` | vb_compile | ⚠️ CI pending (blake3) |
| PO-K09 | `prove_secret_result_not_allowed_enforcement` | vb_runtime | ⚠️ CI pending |
| PO-K10 | `prove_dual_path_digest_equivalence` | vb_compile | ⚠️ CI pending |
| PO-K11 | `prove_validation_covers_all_17_fields` | vb_core | ⚠️ CI pending |
| PO-K12 | `prove_encoding_no_collision` | vb_core | ⚠️ CI pending |
| PO-K13 | `prove_with_default_equivalence` | vb_compile | ⚠️ CI pending |
| PO-K14 | `prove_canonical_policy_digest_agree_on_identity` | vb_compile | ⚠️ CI pending |

**Evidence commands** (from proof-to-rust-map.md):
```bash
# Encoding-level proofs (already pass):
cargo kani -p vb_compile --harness prove_no_cross_field_collision_u32 --unwind 3 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_no_cross_field_collision_u64 --unwind 3 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_contract_encoding_determinism --unwind 1
cargo kani -p vb_compile --harness prove_contract_encoding_is_stable --unwind 1
cargo kani -p vb_compile --harness prove_single_field_changes_encoding --unwind 2
cargo kani -p vb_compile --harness prove_non_default_contract_encoding_differs --unwind 2

# Blake3-level proofs (pending CI):
cargo kani -p vb_compile --harness prove_digest_determinism --unwind 3 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_single_field_changes_digest --unwind 3 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_no_cross_field_collision --unwind 3 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_migration_digest_relationship --unwind 2 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_contract_survives_compilation --unwind 4 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_secret_results_changes_digest --unwind 3 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_dual_path_digest_equivalence --unwind 4 --no-unwinding-checks
cargo kani -p vb_compile --harness prove_canonical_policy_digest_agree_on_identity --unwind 2 --no-unwinding-checks

# Other-crate proofs (pending CI):
cargo kani -p vb_core --harness prove_canonical_contract_has_17_fields --unwind 1
cargo kani -p vb_core --harness prove_type_identity_across_paths --unwind 1
cargo kani -p vb_core --harness prove_validation_covers_all_17_fields --unwind 3
cargo kani -p vb_core --harness prove_encoding_no_collision --unwind 2
cargo kani -p vb_runtime --harness prove_secret_result_not_allowed_enforcement --unwind 3
```

---

## 7. Mutation Checkpoints

Threshold: **≥ 90% kill rate** for all functions modified or added by this bead.

### Critical mutations that must be caught

| Function | Mutation | Must be caught by |
|----------|----------|-------------------|
| `encode_contract_bytes` | Delete a field tag string | PI-02 (encoding injectivity proptest) |
| `encode_contract_bytes` | Change endianness of one field | PI-01 (field sensitivity) |
| `encode_contract_bytes` | Remove `allows_secret_results` encoding | D1 unit test + PI-01 |
| `encode_contract_bytes` | Swap order of two field tags | A5 unit test (field ordering) |
| `encode_contract_bytes` | Use same tag for two different fields | I4 unit test (unique tags) |
| `canonical_digest` (part_05) | Remove `encode_contract_bytes` call | A7 dual-path + PI-04 |
| `canonical_digest` (compile/mod) | Remove `encode_contract_bytes` call | A7 dual-path + PI-04 |
| `canonical_digest` (either) | Skip one contract field | PI-01 (per-field sensitivity) |
| `validate_contract_limit` | Flip `>` to `>=` | E7 hard limit tests |
| `validate_contract_limit` | Remove hard-limit check | E7 hard limit tests |
| `validate_budget` | Remove `max_transitions_per_tick == 0` check | E9 (zero transitions test) |
| `validate_budget` | Remove `> HARD_MAX` check | E9 (hard limit test) |
| `ResourceContract` struct | Delete `allows_secret_results` field | B1 unit test (compile error) |
| `ResourceContract` struct | Delete `max_transitions_per_tick` field | B1 unit test (compile error) |
| `compile_source` (either path) | Hardcode DEFAULT instead of using parameter | C3 (contract preservation test) |
| `CompiledWorkflow::try_from_parts` | Skip `validate_budget` call | E9 (budget validation tests) |
| `CompiledWorkflow::try_from_parts` | Skip `validate_resource_contract` call | E1–E6 (contract exceeded tests) |
| `handle_ask_answer` | Remove `!contract.allows_secret_results` guard | D4 (SecretResultNotAllowed test) |
| `handle_ask_answer` | Invert the boolean condition | D5 (no-rejection-when-true test) |

### Mutation survivor analysis

- **Survivor risk — blake3 internals**: Mutations inside `blake3::Hasher` are not testable by our test suite (external dependency). These are acceptable survivors. The encoding-layer mutations are the primary defense.
- **Survivor risk — `DEFAULT` constant values**: Changing `ResourceContract::DEFAULT.max_steps` from 10_000 to 9_999 would change all DEFAULT-based digests but would NOT be caught unless a specific expected-digest test exists. Add a "known answer test" (KAT) for DEFAULT digest to prevent this.
- **Survivor risk — dead code**: If the 16-field `compiled_workflow::ResourceContract` is deleted, any remaining dead-code imports would be caught by `cargo check` (static analysis). No mutation test needed.

### Known Answer Tests (KAT)

To harden against silent DEFAULT value changes:

```
Given: the canonical DEFAULT WorkflowSource and ResourceContract::DEFAULT
When: canonical_digest is computed
Then: the digest is exactly <expected 32-byte hex value>
```

**Test name**: `fn canonical_digest_known_answer_for_default_contract()`

This test provides a "golden hash" that would change if either `encode_contract_bytes` or `ResourceContract::DEFAULT` changes. Any change to DEFAULT would fail this test, forcing explicit acknowledgement.

---

## 8. Combinatorial Coverage Matrix

### 8.1 encode_contract_bytes

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| DEFAULT contract | `ResourceContract::DEFAULT` | Deterministic Vec<u8> | Unit |
| All fields = 0 | Zeroed contract | Vec<u8> containing all tags + zero bytes | Unit |
| All fields = MAX | Maxed contract | Vec<u8> containing all tags + max LE bytes | Unit |
| allows_secret_results = true | Toggle bool | Bytes differ from false encoding | Unit |
| allows_secret_results = false | Toggle bool | Bytes differ from true encoding | Unit |
| Any field differs from DEFAULT | Single field change | Different from DEFAULT encoding | Unit |
| Two fields differ | Multi-field change | Different from single-field-change encoding | Unit |
| Deterministic | Same input × 2 | Identical bytes | Unit |
| All 17 tags present | Any contract | Tags in fixed order | Unit |
| Tags are unique | Any contract | 17 distinct tag strings | Unit |
| Little-endian u16 | Known LE value | Correct byte order | Unit |
| Little-endian u32 | Known LE value | Correct byte order | Unit |
| Little-endian u64 | Known LE value | Correct byte order | Unit |
| u8 field (max_expr_stack) | Single byte | Not zero-extended | Unit |
| No panic on extreme values | Any valid contract | Returns Vec<u8> | Unit |
| Injectivity | contract_a ≠ contract_b | enc_a ≠ enc_b | Proptest |

### 8.2 canonical_digest

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| DEFAULT contract | source + DEFAULT | Deterministic digest | Integration |
| Non-DEFAULT contract | source + non-default | Different digest from DEFAULT | Integration |
| Single field changes digest | source + contract_a vs contract_b (1 field diff) | digests differ | Proptest |
| Multi-field changes digest | source + contract_a vs contract_b (multi-field diff) | digests differ | Proptest |
| allows_secret_results changes digest | source + contract_true vs contract_false | digests differ | Integration |
| Deterministic | Same (source, contract) × 2 | Same digest | Proptest |
| Dual-path equivalence | Same input to both paths | Same digest | Proptest |
| Known answer (DEFAULT) | Canonical source + DEFAULT | Expected golden hash | Unit (KAT) |
| Contract order in hash | source + contract_a + contract_b | Field order stable | Unit |

### 8.3 compile_source

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| DEFAULT contract | source + DEFAULT | CompiledWorkflow with DEFAULT | Integration |
| Non-DEFAULT contract | source + non-default | CompiledWorkflow with that contract | Integration |
| Contract preserved | source + contract | resource_contract() == contract | Proptest |
| Digest includes contract | source + contract_a vs contract_b | Different digest() | Integration |
| Both paths preserve contract | Both compilation paths | Identical contract in result | Integration |
| compile_source_with_default | source (no contract param) | Same as explicit DEFAULT | Integration |
| Empty workflow | Empty steps YAML | CompileErrors | Integration |
| Invalid source | Malformed YAML | CompileErrors | Integration |

### 8.4 validate_resource_contract (and validate_budget)

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| nodes > max_steps | actual=2, declared=1 | `ResourceContractExceeded { resource: "max_steps" }` | Integration |
| slots > max_slots | actual=2, declared=1 | `ResourceContractExceeded { resource: "max_slots" }` | Integration |
| constants > max_constants | actual=2, declared=1 | `ResourceContractExceeded { resource: "max_constants" }` | Integration |
| accessors > max_accessors | actual=2, declared=1 | `ResourceContractExceeded { resource: "max_accessors" }` | Integration |
| expressions > max_expressions | actual=2, declared=1 | `ResourceContractExceeded { resource: "max_expressions" }` | Integration |
| expr_stack > max_expr_stack | actual=3, declared=2 | `ResourceContractExceeded { resource: "max_expr_stack" }` | Integration |
| max_steps > MAX_STEPS_PER_WORKFLOW | declared > hard limit | `ResourceContractTooLarge { resource: "max_steps" }` | Integration |
| max_slots > MAX_SLOTS_PER_WORKFLOW | declared > hard limit | `ResourceContractTooLarge { resource: "max_slots" }` | Integration |
| max_constants > MAX_CONSTANTS | declared > hard limit | `ResourceContractTooLarge { resource: "max_constants" }` | Integration |
| max_accessors > MAX_ACCESSORS | declared > hard limit | `ResourceContractTooLarge { resource: "max_accessors" }` | Integration |
| max_expressions > MAX_EXPRESSIONS | declared > hard limit | `ResourceContractTooLarge { resource: "max_expressions" }` | Integration |
| max_expr_stack > MAX_EXPRESSION_STACK | declared > hard limit | `ResourceContractTooLarge { resource: "max_expr_stack" }` | Integration |
| max_transitions_per_tick == 0 | zero transitions | `BudgetExceeded` | Integration |
| max_transitions_per_tick > HARD_MAX | declared > HARD_MAX_TRANSITIONS_PER_TICK | `ResourceContractTooLarge` | Integration |
| max_step_budget_per_tick > HARD_MAX | declared > HARD_MAX_STEP_BUDGET_PER_TICK | `ResourceContractTooLarge` | Integration |
| All limits at exact actual | exact match | Ok (no error) | Integration |
| All limits above actual | declared > actual | Ok (no error) | Integration |
| Actual = 0, declared = 0 | degenerate | Ok (vacuous truth) | Integration |
| Declared at hard limit, actual within | declared = hard, actual < hard | Ok (no error) | Integration |
| Declared at hard limit, actual exceeds | declared = hard, actual > hard | `ResourceContractExceeded` (not TooLarge) | Integration |

### 8.5 Runtime Contract Enforcement

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| Secret answer + allows=false | Taint::Secret, flag=false | `Err(SecretResultNotAllowed)` | Integration |
| Secret answer + allows=true | Taint::Secret, flag=true | Ok (answer accepted) | Integration |
| Clean answer + allows=false | Taint::Clean, flag=false | Ok (answer accepted) | Integration |
| Clean answer + allows=true | Taint::Clean, flag=true | Ok (answer accepted) | Integration |
| Derived answer + allows=false | Taint::DerivedFromSecret, flag=false | Behavior depends on runtime policy | Integration |

### 8.6 Type Safety (Static)

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| Contract has 17 fields | Struct literal with all 17 fields named | Compiles | Static |
| Contract has `allows_secret_results` | Field access `.allows_secret_results` | Compiles | Static |
| Contract has `max_transitions_per_tick` | Field access `.max_transitions_per_tick` | Compiles | Static |
| validation/resource.rs imports canonical | `use crate::workflow::ResourceContract` | Compiles; old import fails | Static |
| 16-field duplicate inaccessible | `use crate::compiled_workflow::ResourceContract` | Compile error (if deleted) OR type identity (if alias) | Static |
| No `unsafe` in encoding/validation | `#![forbid(unsafe_code)]` | Compiles | Static |

---

## 9. Implementation Guidance for test-writer

### Files to create

| File | Purpose | Layer |
|------|---------|-------|
| `crates/vb_core/src/contract_encoding/tests.rs` OR inline `#[cfg(test)] mod tests` in `contract_encoding.rs` | I1–I6 unit tests for encoding | Unit |
| `crates/vb_core/tests/resource_contract_validation_17_fields.rs` | E1–E11 validation integration tests | Integration |
| `crates/vb_core/tests/resource_contract_type_integrity.rs` | B1–B5 type integrity tests | Integration |
| `crates/vb_compile/tests/contract_digest_binding.rs` | A1–A8 digest binding integration tests | Integration |
| `crates/vb_compile/tests/entry_point_contract.rs` | C1–C6 entry point integration tests | Integration |

### Files to extend/repair

| File | Change | Bridge Finding |
|------|--------|:---:|
| `crates/vb_compile/tests/proptest_contract_field_sensitivity.rs` | Extend to all 17 fields (PI-01) | PF-BR-003 |
| `crates/vb_compile/tests/proptest_dual_path_equivalence.rs` | **Rewrite**: call both paths independently (PI-04) | PF-BR-001 |
| `crates/vb_compile/tests/proptest_with_default_equivalence.rs` | **Rewrite**: test with_default vs explicit DEFAULT (PI-05) — requires API | PF-BR-002 |
| `crates/vb_compile/tests/proptest_entry_point_contract.rs` | Extend to more randomized fields (PI-06) | — |
| `crates/vb_compile/tests/proptest_digest_determinism.rs` | Consolidate determinism tests into single file (PI-03) | PF-BR-005 |
| `crates/vb_compile/tests/proptest_secret_results_digest_sensitivity.rs` | Keep as-is (already correct) | — |
| `crates/vb_core/src/workflow/tests.rs` | Add validation test matrix (PI-07, PI-08, E7–E11) | — |

### Files that may be removed (consolidation)

| File | Reason |
|------|--------|
| `proptest_digest_determinism.rs` | Consolidate into PI-03 proptest covering all determinism |
| `proptest_dual_path_equivalence.rs` | Rewrite (not delete) for true dual-path test |

### Test naming convention

All test function names must follow the pattern:
```
fn [subject]_[outcome]_when_[condition]()
```

Example:
```rust
fn canonical_digest_differs_when_allows_secret_results_toggled()
fn validation_rejects_nodes_exceeding_max_steps_contract()
fn encode_contract_bytes_is_deterministic()
```

No test may be named `test_foo()`. No test may assert only `is_ok()` or `is_err()` without asserting the exact value or error variant.

### DAMP > DRY enforcement

Each test body must be self-contained. Shared helper functions for constructing `WorkflowParts`, `ResourceContract`, and `WorkflowSource` fixtures are acceptable. Shared assertion functions only if they assert a single conceptual fact (e.g., `assert_resource_exceeded(parts, "max_steps")`).

No shared mutable state between tests. Each test is hermetic.

---

## 10. Open Questions

1. **compile_source_with_default API**: Does `compile_source_with_default` need to be a separate public function, or should it be `compile_source(source, ResourceContract::DEFAULT)` with the DEFAULT as a convenience? The existing bridge repair (PF-BR-002) requires the API to exist for the proptest to be valid. **Resolution needed before PI-05 can be written.**

2. **Dual-path access**: Both `canonical_digest` implementations are `pub(crate)`. Can the dual-path proptest (PI-04) call both from `crates/vb_compile/tests/`? If yes, proceed. If no, the test must go in `crates/vb_compile/src/` as a `#[cfg(test)]` inline test. **Verified: integration tests in `tests/` have access to `pub(crate)` items in the same crate. Proceed with `tests/` location.**

3. **Runtime enforcement tests**: D4 and D5 require a runtime harness. The existing Kani harness PO-K09 provides bounded formal coverage. For behavior test coverage: can we construct a minimal `handle_ask_answer` scenario without spinning up a full runtime? Or should these remain Kani-only until the runtime test infrastructure matures? **Recommendation**: Write a unit test for the `handle_ask_answer` function directly, if it's accessible. If not, note that PO-K09 provides Kani coverage and defer full runtime integration test to a follow-up bead.**

4. **Validation import fix prerequisite**: The import change in `validation/resource.rs` (from `compiled_workflow::ResourceContract` to `workflow::ResourceContract`) is a prerequisite for B4, B5, E1–E11, and PI-07/PI-08. Is this import fix in scope for vb-xi2f.35, or tracked separately? **Confirmed: In scope. See proof-to-rust-map.md Source File Impact Matrix — `validation/resource.rs` import fix is rated HIGH priority and blocks PO-K11 verification.**

5. **Mutation threshold measurement**: At what point should `cargo mutants` be run — after all tests pass? Should it gate CI? **Recommendation**: Run after all tests pass, manually; integrate into CI as a nightly/optional check, not a merge gate. The ≥90% threshold is aspirational; actual threshold depends on blake3 external dependency mutations (which are acceptable survivors).**

6. **Coverage overlap consolidation**: PF-BR-005 notes three determinism tests overlap. Should PO-P04, PO-P05, PO-P06 be consolidated into a single determinism proptest? **Recommendation: Yes — consolidate into PI-03. This frees PO-P04 and PO-P06 slots for their intended purposes (dual-path and with-default equivalence respectively).**

---

## 11. Exit Criteria Verification

| Criterion | Status |
|-----------|:---:|
| Every public API behavior has ≥1 BDD scenario | ✅ 47 behaviors, all with Given-When-Then |
| Every pure function with multiple inputs has ≥1 proptest invariant | ✅ PI-01 through PI-08 |
| Every parsing/deserialization boundary has a fuzz target | ✅ FZ-01 defined (P2 deferred) |
| Every error variant has an explicit test scenario | ✅ E1–E11 cover all contract/budget error variants; D4/D5 cover RuntimeError |
| No test asserts only `is_ok()` or `is_err()` | ✅ All assertions specify exact values or error variants |
| Mutation threshold target stated | ✅ ≥ 90% stated in §7 |
| Bridge gap findings addressed | ✅ PF-BR-001 (PI-04), PF-BR-002 (PI-05), PF-BR-003 (PI-01), PF-BR-004 (B4), PF-BR-005 (consolidation) |
| Trophy allocation justified | ✅ §2 with deviation rationale |

---

## Appendix A: Contract Clause Traceability

| Clause | Behaviors | Tests |
|--------|-----------|-------|
| C1: Digest-Contract Binding | A1–A8 | PI-01, PI-02, PI-03, A1–A8 unit + proptest |
| C2: Single Canonical Type | B1–B5 | B1–B5 static + unit |
| C3: Entry Point Contract | C1–C6 | PI-06, C1–C6 integration, PI-05 |
| C4: Taint Digest Sensitivity | D1–D5 | D1–D5 unit + integration, PO-K09 |
| C5: Full Validation | E1–E11 | E1–E11 integration, PI-07, PI-08 |
| C6: Dual Path Consistency | F1–F3 | PI-04, F1 integration |
| C7: YAML Parsing (P2) | G1–G5 | FZ-01 fuzz target (defined, deferred) |
| C8: Backward Compatibility | H1–H3 | H1 doc check, H2–H3 integration |
| C9: Proof Obligation | — | 14 Kani harnesses (existing), 8 proptest invariants |
| C10: Non-Requirements | — | N/A |

## Appendix B: Hazard Coverage

| Hazard | Covered By |
|--------|-----------|
| H-001: Digest orphan | PI-01 (all 17 fields), A2–A8 (BDD) |
| H-002: Duplicate types | B1–B5 (type integrity) |
| H-003: Hardcoded DEFAULT | C1–C6, PI-05, PI-06 |
| H-004: Taint silent match | D1–D5, PI-01 (allows_secret_results field) |
| H-005: Dual path drift | PI-04, F1 |
| H-006: Missing YAML parsing | FZ-01 (fuzz target defined, P2) |
| H-007: Validation gap | E9, E10, PI-08 |
| H-008: No test coverage | Entire test plan addresses this |
| H-009: Digest split | H2, PO-K14 Kani |
| H-010: Field name stability | I4 (unique tags), A5 (field ordering), KAT |
