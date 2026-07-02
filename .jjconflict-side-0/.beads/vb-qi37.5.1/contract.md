# Contract Specification: vb-qi37.5.1 - verifier idempotency contract model

## 1. Scope

Define the contract model for verifier-side idempotency checks over compiled workflow IR and registered `vb_core::action::ActionContract` values.

The verifier model must make one source of truth for static idempotency legality and must not duplicate divergent rules between `vb_validate`, `vb_compile::check_idempotency_gates`, CLI verification, and IPC certificate generation.

### In scope

- Static verifier contract over `WorkflowParts` plus workflow-specific `ActionContract` records.
- Compatibility with existing core domain terms: `ActionContract`, `Idempotency`, `SideEffect`, `RetrySafety`, `IdempotencyViolation`, `CompiledNodeKind::Do`.
- A typed validation error surface for idempotency contract failures.
- Acceptance rules for each `SideEffect`, `RetrySafety`, and `Idempotency` combination.
- Bounded, deterministic, allocation-conscious verification behavior consistent with the existing validation gate style.

### Out of scope

- Production code implementation.
- Test implementation.
- Runtime action dispatch behavior beyond documenting how static verifier proof relates to `verify_idempotency`.
- Runtime parsing of JSON, YAML, or HTTP in the core.
- New idempotency-key derivation algorithms.
- Proving `RandomInKey` or `TimeInKey` until slot metadata exists to support those checks.
- UI or IPC contract registry plumbing, except as a consumer requirement.

## 2. Context read

Relevant current behavior:

- `vb_core::action` owns `Idempotency`, `SideEffect`, `RetrySafety`, `IdempotencyViolation`, `ActionContract`, `ActionTicket`, `verify_idempotency`, and `validate_idempotency_key_ingredients`.
- Current runtime `verify_idempotency` rules: `SideEffect::None` passes; `RetrySafety::Safe` passes; `RetrySafety::KeyRequired` requires non-empty clean key slots; `RetrySafety::Unsafe` fails.
- `vb_compile::check_idempotency_gates` statically rejects side-effecting `RetrySafety::Unsafe` and side-effecting `Idempotency::AtLeastOnceExternal`.
- `vb_validate::shared::validate` skips Gate 12 because action contracts are external; `validate_with_contracts` runs Gate 12 after non-contract gates.
- Gate 12 currently proves completeness only: every Do node has a matching contract and every supplied contract is used by at least one Do node.
- CLI `verify` currently calls `compile_workflow` and `vb_validate::shared::validate(&parts)`, so it cannot prove action contracts without a registered contract source.

## 3. Domain terms

- **Workflow action use**: a `CompiledNodeKind::Do { action, input }` occurrence in `WorkflowParts.nodes`.
- **Action contract**: `ActionContract` for an action ID used by the workflow.
- **Workflow-specific registry**: the finite set of contracts supplied for a single workflow verification run.
- **Pure action**: contract with `side_effect == SideEffect::None`.
- **Side-effecting action**: contract with `side_effect != SideEffect::None`.
- **Statically idempotent external action**: side-effecting contract with `idempotency == Idempotency::IdempotentExternal` and `retry_safety != RetrySafety::Unsafe`.
- **Key-required action**: side-effecting contract with `retry_safety == RetrySafety::KeyRequired`; verifier may statically accept the contract model but runtime dispatch must supply clean key ingredients before retry.
- **At-least-once external action**: contract with `idempotency == Idempotency::AtLeastOnceExternal`; static verifier must reject when side-effecting.

## 4. Assumptions

- The canonical domain enums remain in `vb_core::action`; downstream crates must not define parallel enums.
- Verifier idempotency validation is a cold-path static gate over workflow-specific contracts.
- Gate 12 completeness remains separate but must run before or as part of idempotency verifier checks when `WorkflowParts` are involved.
- The verifier must accumulate idempotency contract violations across all supplied relevant contracts, matching `vb_compile::check_idempotency_gates` multi-error behavior, rather than stopping at the first idempotency violation.
- Existing Gate 12 may still short-circuit for missing or orphan contracts unless a future bead explicitly changes Gate 12 accumulation.
- `ActionTicket.idempotency_key == 0` remains a valid numeric key value and must not be used as the static representation of key absence.

## 5. Open questions for later states

- Where will CLI `verify` obtain action contracts: embedded sample registry, explicit contract input, generated Rust registry, or deployment metadata?
- Should Gate 12 keep rejecting orphan contracts if callers provide a global registry rather than a workflow-specific subset?
- Should diagnostic rendering expose every accumulated idempotency violation in CLI and IPC certificates, or only typed structured errors?
- Will future slot metadata represent random/time provenance so `RandomInKey` and `TimeInKey` can become enforceable?

## 6. Contract invariants

### I1. Canonical type invariant

All idempotency contract checks must use `vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect}` as the canonical model.

### I2. Workflow-specific registry invariant

For a successful verifier proof, every `Do` action ID in `WorkflowParts` has exactly one matching effective contract in the supplied workflow-specific registry, and every effective contract corresponds to at least one `Do` action unless the caller explicitly uses a future non-orphan global-registry mode.

### I3. Pure action invariant

If `contract.side_effect == SideEffect::None`, the static idempotency verifier accepts the contract regardless of `idempotency` and `retry_safety`. This preserves current runtime behavior where pure actions have no observable retry side effects.

### I4. Side-effecting unsafe invariant

If `contract.side_effect != SideEffect::None` and `contract.retry_safety == RetrySafety::Unsafe`, the static verifier rejects the contract.

### I5. Side-effecting at-least-once invariant

If `contract.side_effect != SideEffect::None` and `contract.idempotency == Idempotency::AtLeastOnceExternal`, the static verifier rejects the contract.

### I6. Side-effecting accepted invariant

A side-effecting contract is statically accepted only when:

- `idempotency == Idempotency::IdempotentExternal`, and
- `retry_safety == RetrySafety::Safe` or `retry_safety == RetrySafety::KeyRequired`.

### I7. Key-required separation invariant

Static verification may accept `RetrySafety::KeyRequired` only as a deploy-time contract. Runtime dispatch/retry must still prove non-empty clean key ingredients through `validate_idempotency_key_ingredients` or an equivalent typed contract.

### I8. Secret-taint key invariant

`Taint::Secret` and `Taint::DerivedFromSecret` values must not participate in idempotency key ingredients. This is a runtime key-ingredient invariant, not a static workflow-contract invariant unless future IR metadata exposes key slots.

### I9. Bounded traversal invariant

Verifier traversal must be bounded by `parts.nodes.len()` and `contracts.len()`. Index increments must use checked arithmetic or iterator constructs that cannot overflow and must not use unchecked indexing.

### I10. Deterministic diagnostics invariant

For identical `WorkflowParts` and contract inputs, idempotency diagnostics must be deterministic in count, action order, side-effect classification, and reason text.

## 7. Preconditions

### P1. Structural validation precondition

`WorkflowParts` supplied to verifier idempotency checks have already passed non-contract structural gates 7, 8, 9, 10, 11, 13, 14, and 15, or the idempotency verifier is invoked through a pipeline that runs those gates before idempotency proof.

### P2. Contract completeness precondition

When checking workflow idempotency, Gate 12 completeness has passed for `parts` and `action_contracts`, or the idempotency verifier must return a typed completeness error before evaluating idempotency legality.

### P3. Workflow-specific contracts precondition

The `action_contracts` slice represents the intended effective contracts for this workflow. If the input is global, callers must filter it before invoking the current contract model.

### P4. No runtime parser precondition

The verifier contract model receives typed Rust values. It must not parse YAML, JSON, or HTTP in the runtime core.

### P5. Domain-value precondition

The verifier must treat all enum values as closed Rust enum variants and must not depend on serialized numeric discriminants for behavior.

## 8. Postconditions

### Q1. Success postcondition

On `Ok(())`, all action contracts relevant to workflow `Do` nodes are statically safe under invariants I3 through I7.

### Q2. Violation accumulation postcondition

On idempotency failure, all statically detectable idempotency violations in the supplied relevant contract set are returned in deterministic action traversal order.

### Q3. Completeness failure postcondition

If a `Do` node has no matching contract or a workflow-specific contract is orphaned, the verifier returns a typed completeness error and does not claim idempotency proof.

### Q4. No mutation postcondition

Verifier checks do not mutate `WorkflowParts`, `ActionContract`, `RunFrame`, registries, or global state.

### Q5. No effect postcondition

Verifier checks perform no external I/O, network calls, filesystem writes, or runtime action dispatch.

### Q6. Stable diagnostics postcondition

Each violation includes at minimum action ID, side-effect classification, idempotency classification, retry-safety classification, and a stable reason category.

## 9. Typed error taxonomy

The implementation should introduce or expose typed errors rather than string-only diagnostics. Exact names may be adapted to crate conventions, but semantics must remain exhaustive.

```rust
pub type IdempotencyContractResult<T> = Result<T, IdempotencyContractErrors>;

pub struct IdempotencyContractErrors(pub Box<[IdempotencyContractViolation]>);

pub enum IdempotencyContractError {
    ActionContractMissing { action_id: usize, node_index: usize },
    ActionContractOrphan { action_id: usize },
    IdempotencyViolations(IdempotencyContractErrors),
}

pub enum IdempotencyContractViolation {
    SideEffectingRetryUnsafe {
        action: ActionId,
        side_effect: SideEffect,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    },
    SideEffectingAtLeastOnceExternal {
        action: ActionId,
        side_effect: SideEffect,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    },
    SideEffectingDeterministicPure {
        action: ActionId,
        side_effect: SideEffect,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    },
}
```

### Error semantics

- `ActionContractMissing`: a `Do` node references an action ID absent from the workflow-specific registry.
- `ActionContractOrphan`: a supplied workflow-specific contract has no corresponding `Do` node.
- `SideEffectingRetryUnsafe`: side-effecting action declares `RetrySafety::Unsafe`.
- `SideEffectingAtLeastOnceExternal`: side-effecting action declares `Idempotency::AtLeastOnceExternal`.
- `SideEffectingDeterministicPure`: side-effecting action declares `Idempotency::DeterministicPure`; this is invalid because deterministic-pure semantics conflict with observable side effects. If backward compatibility requires accepting this in the first implementation, this contract must be explicitly amended before code is written.

### Stable diagnostic reason categories

- `IDEMPOTENCY_RETRY_UNSAFE`
- `IDEMPOTENCY_AT_LEAST_ONCE_EXTERNAL`
- `IDEMPOTENCY_SIDE_EFFECTING_DETERMINISTIC_PURE`
- `ACTION_CONTRACT_MISSING`
- `ACTION_CONTRACT_ORPHAN`

## 10. Contract signatures

These signatures are specification targets only; no production code is implemented by this artifact.

```rust
pub fn validate_workflow_idempotency_contracts(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Result<(), IdempotencyContractError>;

pub fn validate_action_idempotency_contract(
    contract: &ActionContract,
) -> Result<(), IdempotencyContractViolation>;

pub fn collect_idempotency_contract_violations(
    action_contracts: &[ActionContract],
) -> Result<(), IdempotencyContractErrors>;

pub fn is_statically_idempotent_contract(
    contract: &ActionContract,
) -> Result<(), IdempotencyContractViolation>;
```

### Signature constraints

- All fallible operations return `Result<T, Error>`.
- Error variants carry typed fields, not only strings.
- Functions borrow inputs and do not take ownership unless ownership is required for bounded diagnostic storage.
- Multi-error collection uses bounded memory proportional to number of contracts and never stores unbounded strings.

## 11. Acceptance criteria

- `contract.md` exists and is non-empty for this bead.
- The contract model identifies `vb_core::action` as the canonical domain source.
- Static acceptance/rejection rules are specified for all `SideEffect`, `RetrySafety`, and `Idempotency` combinations.
- Side-effecting `RetrySafety::Unsafe` is rejected.
- Side-effecting `Idempotency::AtLeastOnceExternal` is rejected.
- Side-effecting `Idempotency::DeterministicPure` is specified as rejected due to contradictory semantics.
- Pure actions are accepted regardless of retry/idempotency fields.
- `RetrySafety::KeyRequired` is statically accepted only for side-effecting `IdempotentExternal` actions and remains subject to runtime clean-key proof.
- `ActionTicket.idempotency_key == 0` is not treated as missing key.
- `Secret` and `DerivedFromSecret` remain forbidden key ingredients for runtime key validation.
- The verifier model accumulates all idempotency contract violations in deterministic order.
- Contract-aware verification is specified as separate from parser/runtime core concerns.
- No production code or tests are implemented by this artifact.

## 12. Martin Fowler test plan

These are executable-specification scenarios for later test-writing states. They must be implemented as tests only after the contract is accepted.

### Happy path tests

- `accepts_workflow_with_no_do_nodes_and_no_contracts`
- `accepts_pure_action_regardless_of_retry_safety`
- `accepts_side_effecting_idempotent_external_when_retry_safe`
- `accepts_side_effecting_idempotent_external_when_key_required`
- `accepts_multiple_do_nodes_when_each_contract_is_safe_and_complete`

### Error path tests

- `rejects_do_node_when_action_contract_is_missing`
- `rejects_workflow_specific_orphan_contract`
- `rejects_side_effecting_action_when_retry_safety_is_unsafe`
- `rejects_side_effecting_action_when_idempotency_is_at_least_once_external`
- `rejects_side_effecting_action_when_idempotency_is_deterministic_pure`
- `accumulates_multiple_idempotency_violations_in_action_order`

### Edge case tests

- `accepts_empty_contract_slice_when_workflow_has_no_do_nodes`
- `does_not_treat_zero_idempotency_key_as_missing_static_key`
- `keeps_key_required_static_acceptance_separate_from_runtime_key_validation`
- `returns_deterministic_diagnostics_for_reordered_unrelated_non_do_nodes`
- `handles_maximum_reasonable_contract_count_without_unbounded_growth`

### Contract verification tests

- `verifies_invariant_pure_action_always_passes_static_gate`
- `verifies_invariant_side_effecting_unsafe_always_fails_static_gate`
- `verifies_invariant_side_effecting_at_least_once_always_fails_static_gate`
- `verifies_invariant_side_effecting_idempotent_external_key_required_passes_static_gate`
- `verifies_invariant_secret_taint_rejected_by_runtime_key_ingredient_validation`
- `verifies_postcondition_success_proves_all_relevant_contracts_safe`
- `verifies_postcondition_failure_reports_typed_action_side_effect_and_reason`

## 13. Given/When/Then scenarios

### Scenario 1: Pure workflow has no action contracts

Given a compiled workflow with no `Do` nodes
And an empty workflow-specific action contract registry
When verifier idempotency contract validation runs
Then validation succeeds
And no idempotency diagnostics are emitted

### Scenario 2: Side-effecting idempotent action is retry safe

Given a workflow with one `Do` node for action `A`
And contract `A` has `side_effect = Writes`
And contract `A` has `idempotency = IdempotentExternal`
And contract `A` has `retry_safety = Safe`
When verifier idempotency contract validation runs
Then validation succeeds
And the verifier records that action `A` is statically retry safe

### Scenario 3: Side-effecting action requires a runtime key

Given a workflow with one `Do` node for action `A`
And contract `A` has `side_effect = Sends`
And contract `A` has `idempotency = IdempotentExternal`
And contract `A` has `retry_safety = KeyRequired`
When verifier idempotency contract validation runs
Then validation succeeds statically
And the proof states that runtime dispatch must supply non-empty clean key ingredients before retry

### Scenario 4: Unsafe side-effecting action is rejected

Given a workflow with one `Do` node for action `A`
And contract `A` has `side_effect = Destroys`
And contract `A` has `retry_safety = Unsafe`
When verifier idempotency contract validation runs
Then validation fails
And the failure includes `SideEffectingRetryUnsafe`
And the failure includes action `A` and side effect `Destroys`

### Scenario 5: At-least-once side-effecting action is rejected

Given a workflow with one `Do` node for action `A`
And contract `A` has `side_effect = Creates`
And contract `A` has `idempotency = AtLeastOnceExternal`
When verifier idempotency contract validation runs
Then validation fails
And the failure includes `SideEffectingAtLeastOnceExternal`

### Scenario 6: Contradictory deterministic-pure side effect is rejected

Given a workflow with one `Do` node for action `A`
And contract `A` has `side_effect = Writes`
And contract `A` has `idempotency = DeterministicPure`
When verifier idempotency contract validation runs
Then validation fails
And the failure includes `SideEffectingDeterministicPure`

### Scenario 7: Missing contract blocks proof

Given a workflow with one `Do` node for action `A`
And no matching contract for action `A`
When contract-aware verifier validation runs
Then validation fails with `ActionContractMissing`
And no successful idempotency proof is claimed

### Scenario 8: Multiple idempotency violations are accumulated

Given a workflow with `Do` nodes for actions `A` and `B`
And contract `A` is side-effecting and retry-unsafe
And contract `B` is side-effecting and at-least-once external
When verifier idempotency contract validation runs
Then validation fails
And diagnostics include both action `A` and action `B`
And diagnostics are ordered deterministically by contract traversal order

### Scenario 9: Runtime key taint remains forbidden

Given a statically accepted key-required action
And runtime key slots contain a `Secret` or `DerivedFromSecret` taint
When runtime key ingredient validation runs
Then validation fails with `IdempotencyViolation::SecretInKey`
And the static verifier contract remains unchanged because key taint is runtime frame data

## 14. Proof obligations

- Prove no side-effecting action can pass static verification when declared retry-unsafe.
- Prove no side-effecting action can pass static verification when declared at-least-once external.
- Prove no side-effecting action can pass static verification when declared deterministic-pure.
- Prove pure actions preserve existing acceptance behavior.
- Prove key-required actions are accepted statically only under `IdempotentExternal` semantics.
- Prove missing and orphan contracts prevent a successful workflow idempotency proof.
- Prove all fallible contract APIs expose typed `Result` errors.
- Prove diagnostic order is deterministic for identical inputs.
- Prove verifier functions perform no external effects and do not mutate inputs.
- Prove implementation respects repository safety constraints: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked casts, or unchecked arithmetic in production.

## 15. Risk notes

- CLI `verify` lacks contract input today; adding strict idempotency proof to default CLI verification without a contract source would create false failures or skipped proof ambiguity.
- IPC certificate code currently passes empty contracts to Gate 12; stricter verifier checks require registry plumbing before certificates can honestly claim action idempotency proof.
- Existing compile and validate surfaces have overlapping idempotency logic; implementation must centralize rules or delegate to one shared function to prevent drift.
- Current runtime `verify_idempotency` accepts side-effecting `DeterministicPure` with `RetrySafety::Safe`; this contract intentionally rejects that combination statically because the declaration is contradictory. This may require migration tests or compatibility notes.
- `RandomInKey` and `TimeInKey` variants exist but cannot be fully enforced until slot provenance metadata exists.
- Gate 12 orphan rejection can conflict with global registries; current contract assumes workflow-specific registries.

## 16. Moon/CI and repository constraints for implementers

- `moon ci` remains the canonical quality gate.
- No runtime JSON/YAML/HTTP in the core idempotency model.
- No unsafe code.
- No panics, unwraps, expects, todos, unimplemented markers, dbg macros, unchecked indexing, unchecked slicing, unchecked casts, or unchecked arithmetic in production code.
- Keep cold-path verifier bounded and deterministic.
- Do not make performance claims without baseline/result benchmark evidence.
