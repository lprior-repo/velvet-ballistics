# Contract: vb-qi37.7.3 - ir: Validate symbol, action, and resource references

## 1. Scope

This bead specifies cold-path validation for compiled numeric IR reference integrity before an artifact is admitted or executed.

Validation applies to:

- Symbol references inside `WorkflowParts`:
  - `AccessorProgram.path` entries of `PathSegment::Field(SymbolId)`.
  - `ConstValue::Symbol(SymbolId)` constant-pool entries.
  - `CompiledNodeKind::BuildObject { fields }` field-key `SymbolId` values.
- Action references inside `CompiledNodeKind::Do { action, input }` nodes, validated against supplied `ActionContract` data.
- Resource references/contracts declared by `WorkflowParts.resource_contract`, including node count, slot count, constants, accessors, expressions, and expression stack depth.

Primary implementation surfaces expected by this contract:

- Core admission: `vb_core::workflow::CompiledWorkflow::try_from_parts(parts)` for symbol and resource validation.
- Cold verifier: `vb_validate::shared::validate(parts)` for non-action gates, and `vb_validate::shared::validate_with_contracts(parts, action_contracts)` when action contracts are available.
- Gate 12 remains the explicit action-contract boundary because `WorkflowParts` does not own an action-contract table.

## 2. Domain Terms

- `WorkflowParts`: untrusted compiled workflow artifact emitted across a compiler boundary.
- `SymbolId`: interned numeric symbol key. A symbol reference is valid only when `symbol.get() < parts.symbols_count`.
- `ActionId`: numeric action dispatch key used by `CompiledNodeKind::Do`.
- `ActionContract`: static contract for an action, including `id`, input/output slot counts, byte bounds, timeout, idempotency, side-effect, retry safety, and required capabilities.
- `ResourceContract`: declared upper bounds carried with the compiled artifact.
- Hard protocol limit: maximum bound accepted by the core independent of artifact contents.
- Orphan action contract: supplied contract whose `id` is not referenced by any `Do` node.

## 3. Preconditions

### P1. Structural input ownership

- Validators receive borrowed `&WorkflowParts`; action validation receives borrowed `&[ActionContract]`.
- Validation must not mutate `WorkflowParts`, action contracts, registries, or global state.

### P2. Core admission input

- `CompiledWorkflow::try_from_parts(parts)` may receive untrusted `WorkflowParts`.
- All fallible admission behavior must return `Result<CompiledWorkflow, WorkflowError>`.

### P3. Action validation input

- `validate_with_contracts(parts, action_contracts)` must be used by callers that require action-reference completeness.
- `validate(parts)` is allowed to skip Gate 12 because action contracts are external to `WorkflowParts`.
- Action contract lookup is in-memory only; no runtime JSON, YAML, HTTP, filesystem, or network lookup may be introduced into the runtime core.

### P4. Resource validation input

- Declared `ResourceContract` values are treated as untrusted.
- Actual resource usage is derived from the in-memory IR arrays and expression metadata only.

## 4. Postconditions

### S1. Valid artifact admission

If core admission returns `Ok(CompiledWorkflow)`, then:

- Every symbol reference in accessors, constants, and build-object field keys is within `symbols_count`.
- `symbols_count == 0` implies no symbol-bearing IR location exists.
- Declared resource contract values do not exceed hard protocol limits.
- Actual resource usage does not exceed declared resource contract values.
- Expression `max_stack` requirements do not exceed `resource_contract.max_expr_stack`.
- Existing node, slot, constant, expression, accessor, reachability, and forward-edge invariants remain preserved.

### S2. Valid verifier result without contracts

If `vb_validate::shared::validate(parts)` returns `Ok(())`, then all non-action gates selected by the default pipeline have passed. This result does not prove that `Do.action` values have contracts.

### S3. Valid verifier result with contracts

If `vb_validate::shared::validate_with_contracts(parts, action_contracts)` returns `Ok(())`, then:

- Every `CompiledNodeKind::Do.action` has at least one supplied `ActionContract` with exactly the same `ActionId`.
- Every supplied `ActionContract.id` is referenced by at least one `Do` node.
- No missing action contract or orphan action contract remains.

### S4. Failure atomicity

On any validation error:

- No partially accepted `CompiledWorkflow` is returned.
- No mutation or side effect is performed.
- The first failing gate/admission check returns a typed error variant, not a string-only failure.

## 5. Invariants

### I1. Symbol bounds invariant

For every symbol-bearing IR location `s`, `s.get() < parts.symbols_count`.

### I2. Zero-symbol invariant

When `parts.symbols_count == 0`, any `PathSegment::Field(_)`, `ConstValue::Symbol(_)`, or `BuildObject` field key must be rejected.

### I3. Action-contract bijection invariant

For action-validation mode, the set of unique `Do.action` IDs must equal the set of unique supplied `ActionContract.id` values.

### I4. Resource hard-limit invariant

No declared `ResourceContract` member may exceed its protocol hard limit.

### I5. Resource coverage invariant

Actual IR usage must be less than or equal to the corresponding declared `ResourceContract` member.

### I6. Deterministic bounded-resource posture

Validation must use bounded, deterministic scans over existing arrays/slices. It must not add unbounded recursion, network access, runtime schema parsing, dynamic plugin loading, or data-dependent allocation beyond small bounded collections already used by validation gates.

### I7. Engineering invariant

New source must preserve repo constraints: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked casts, or unchecked arithmetic.

## 6. Contract Signatures

The final code may keep existing function names, but behavior must be expressible by these contracts:

```rust
pub fn validate_symbol_references(parts: &WorkflowParts) -> Result<(), WorkflowError>;

pub fn validate_resource_references(parts: &WorkflowParts) -> Result<(), WorkflowError>;

pub fn validate_action_references(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Result<(), ValidationError>;

pub fn validate(parts: &WorkflowParts) -> Result<(), ValidationError>;

pub fn validate_with_contracts(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Result<(), ValidationError>;

impl CompiledWorkflow {
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError>;
}
```

All fallible operations must remain railway-oriented through `Result<T, Error>`.

## 7. Typed Error Taxonomy

### Core admission errors

- `WorkflowError::SymbolOutOfBounds { symbol }`
  - Returned when any symbol-bearing IR location has `symbol.get() >= parts.symbols_count`.
- `WorkflowError::ResourceContractTooLarge { resource }`
  - Returned when a declared resource contract value exceeds a protocol hard limit.
- `WorkflowError::ResourceContractExceeded { resource }`
  - Returned when actual IR usage exceeds the declared resource contract.
- Existing structural errors remain valid and must not be collapsed into generic errors.

### Cold verifier errors

- `ValidationError::ActionContractMissing { action_id, node_index }`
  - Returned when a `Do` node references an action without a supplied matching contract.
- `ValidationError::ActionContractOrphan { action_id }`
  - Returned when a supplied contract has no matching `Do` node.
- A symbol-reference verifier error must be added or an existing precise verifier error must be extended if `vb_validate` is required to match core symbol coverage. The error must include at least the offending `symbol` and enough context to identify whether the source is accessor, constant, or build-object field.
- Resource verifier errors must either map to existing precise gate errors or introduce exact variants; they must not use string-only diagnostics.

### Diagnostic-code obligations

- Existing E05xx codes for action contract errors must remain stable.
- Any new verifier symbol/resource error variant must receive a stable diagnostic code and diagnostic rendering coverage.

## 8. Acceptance Criteria

1. Core admission rejects out-of-range symbols in all three symbol-bearing locations: accessor field segments, `ConstValue::Symbol`, and build-object field keys.
2. Core admission rejects every symbol-bearing IR location when `symbols_count == 0`.
3. Cold verifier achieves parity for symbol reference validation, or the implementation documents and tests that symbol validation is intentionally core-only.
4. `validate_with_contracts` rejects every `Do.action` without a matching `ActionContract.id` using `ValidationError::ActionContractMissing` with the offending node index.
5. `validate_with_contracts` rejects every orphan supplied `ActionContract.id` using `ValidationError::ActionContractOrphan`.
6. `validate` continues to skip Gate 12 and must not falsely claim action-contract completeness.
7. Resource contracts reject both declared-over-hard-limit and actual-usage-over-declared-limit cases with exact typed errors.
8. Validation remains pure, deterministic, and bounded over in-memory IR; no runtime JSON/YAML/HTTP is introduced into the runtime core.
9. New tests assert exact enum variants and salient fields, not only error strings.
10. `moon ci` is the canonical acceptance gate after implementation.

## 9. Martin Fowler Given/When/Then Scenarios

### Scenario 1: accepts valid symbol references across all carriers

Given a `WorkflowParts` with `symbols_count = 3` and symbol IDs `0`, `1`, and `2` used in an accessor field, a symbol constant, and a build-object field
When `CompiledWorkflow::try_from_parts(parts)` runs
Then admission succeeds and the resulting workflow preserves the supplied IR.

### Scenario 2: rejects accessor field symbol outside declared symbol table

Given a `WorkflowParts` with `symbols_count = 1` and an accessor path containing `PathSegment::Field(SymbolId::new(1))`
When core admission runs
Then it returns `Err(WorkflowError::SymbolOutOfBounds { symbol })` for `SymbolId::new(1)`.

### Scenario 3: rejects symbol constant outside declared symbol table

Given a `WorkflowParts` with `symbols_count = 0` and `constants` containing `ConstValue::Symbol(SymbolId::new(0))`
When core admission runs
Then it returns `Err(WorkflowError::SymbolOutOfBounds { symbol })`.

### Scenario 4: rejects build-object field symbol outside declared symbol table

Given a `WorkflowParts` with `symbols_count = 2` and a `BuildObject` field key `SymbolId::new(2)`
When core admission runs
Then it returns `Err(WorkflowError::SymbolOutOfBounds { symbol })`.

### Scenario 5: validates action contract completeness

Given a `WorkflowParts` containing `Do { action: ActionId::new(7), input }` and a supplied `ActionContract { id: ActionId::new(7), ... }`
When `validate_with_contracts(parts, contracts)` runs
Then validation succeeds for Gate 12.

### Scenario 6: rejects missing action contract

Given a `WorkflowParts` containing a `Do` node with `ActionId::new(7)` and an empty contract slice
When `validate_with_contracts(parts, contracts)` runs
Then it returns `Err(ValidationError::ActionContractMissing { action_id: 7, node_index })`.

### Scenario 7: rejects orphan action contract

Given a `WorkflowParts` with no `Do` node for `ActionId::new(9)` and a supplied `ActionContract { id: ActionId::new(9), ... }`
When `validate_with_contracts(parts, contracts)` runs
Then it returns `Err(ValidationError::ActionContractOrphan { action_id: 9 })`.

### Scenario 8: validate without contracts does not prove action completeness

Given a `WorkflowParts` containing a `Do` node and no action contracts are supplied
When `validate(parts)` runs
Then Gate 12 is skipped and the result must not be interpreted as action-contract completeness.

### Scenario 9: rejects resource contract above hard limit

Given a `WorkflowParts` whose `resource_contract.max_expr_stack` exceeds `MAX_EXPRESSION_STACK`
When core admission runs
Then it returns `Err(WorkflowError::ResourceContractTooLarge { resource: "max_expr_stack" })`.

### Scenario 10: rejects actual resource usage above declared contract

Given a `WorkflowParts` with two nodes and `resource_contract.max_steps = 1`
When core admission runs
Then it returns `Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" })`.

### Scenario 11: rejects expression stack usage above declared resource contract

Given an expression whose `max_stack` is greater than `resource_contract.max_expr_stack`
When core admission or the corresponding verifier gate runs
Then validation returns the exact typed resource/stack error for expression stack overuse.

## 10. Proof Obligations

- Symbol coverage proof: tests must cover accessor fields, constants, and build-object fields independently.
- Zero-bound proof: at least one test must prove `symbols_count == 0` rejects `SymbolId::new(0)`.
- Action bijection proof: tests must cover success, missing contract, orphan contract, and duplicate `Do` references to the same contract if supported.
- Pipeline-boundary proof: tests must prove `validate` skips Gate 12 and `validate_with_contracts` includes Gate 12.
- Resource proof: tests must cover `ResourceContractTooLarge` and `ResourceContractExceeded` separately.
- Diagnostic proof: any new `ValidationError` variant must have diagnostic conversion, diagnostic rendering, and stable code coverage.
- Boundedness proof: implementation must use linear scans or bounded collections only; no unbounded recursion or runtime I/O.

## 11. Out of Scope

- Implementing production code or tests in this state.
- Adding action contracts into `WorkflowParts` unless a later architecture decision explicitly changes the IR boundary.
- Runtime action dispatch semantics beyond compile-time/action-reference contract lookup.
- YAML/string-level reference validation in `vb_validate::references` or `ref_validate`; this bead is numeric IR validation.
- Performance claims or benchmarks.
- Generated Rust dispatch changes.

## 12. Risk Notes

- `vb_core` already validates symbols/resources, while `vb_validate` appears to have weaker symbol coverage; parity must be deliberate to avoid split-brain validation.
- Forcing action validation into `CompiledWorkflow::try_from_parts` would create an API-boundary problem because action contracts are external data.
- Gate 12 currently treats orphan contracts as errors; changing this would alter the existing action-contract bijection invariant.
- Adding symbol verifier errors requires stable diagnostic-code allocation and renderer updates.
- Compiler paths that emit `symbols_count: 0` must not emit symbol-bearing IR without updating symbol-table production.
