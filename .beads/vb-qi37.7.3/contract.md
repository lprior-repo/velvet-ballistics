# Contract Specification: vb-qi37.7.3

## Context
- Feature: IR admission validates symbol, slot, action, resource, handler, and constant references before an artifact is trusted.
- Scope: numeric compiled IR in `WorkflowParts`, core admission via `CompiledWorkflow::try_from_parts`, and cold verifier admission via `vb_validate::shared::{validate, validate_with_contracts}`.
- Domain terms: `SymbolId`, `ActionId`, `ActionContract`, `ResourceContract`, `CompiledNodeKind::Do`, `CompiledNodeKind::BuildObject`, `AccessorProgram`, `PathSegment::Field`, `ConstValue::Symbol`, slot reference, handler reference, constant reference, admitted artifact.
- Assumptions: action contracts remain external to `WorkflowParts`; `validate(parts)` intentionally skips action-contract completeness; `validate_with_contracts(parts, contracts)` is the action-complete verifier surface.
- Open questions: exact verifier enum names for added symbol/handler/constant diagnostics may be chosen by implementation, but they must be typed and stable-code backed.

## Preconditions
- PRE-001: Validators accept untrusted `WorkflowParts`; no caller may assume reference integrity before admission.
- PRE-002: Validation receives borrowed data and must not mutate `WorkflowParts`, action contracts, registries, globals, or external state.
- PRE-003: `CompiledWorkflow::try_from_parts(parts)` owns the submitted artifact and returns `Result<CompiledWorkflow, WorkflowError>`.
- PRE-004: Action-complete validation requires supplied `&[ActionContract]`; callers using only `validate(parts)` must treat action completeness as unproven.
- PRE-005: Declared `ResourceContract` values and all reference IDs are untrusted numeric inputs.
- PRE-006: Runtime core validation must not perform YAML, JSON, HTTP, filesystem, network, plugin, or dynamic schema lookup.

## Postconditions
- POST-001: `Ok(CompiledWorkflow)` implies every `SymbolId` in accessor fields, symbol constants, build-object field keys, and any admitted symbol-bearing location is `< parts.symbols_count`.
- POST-002: `Ok(CompiledWorkflow)` implies `symbols_count == 0` has no symbol-bearing IR location.
- POST-003: `Ok(())` from `validate_with_contracts` implies every `Do.action` has a matching supplied `ActionContract.id` and every supplied `ActionContract.id` is referenced by at least one `Do` node.
- POST-004: `Ok(())` from `validate(parts)` proves only default non-action gates; it must not be documented or surfaced as action-contract completeness.
- POST-005: Valid admission implies slot, constant, expression, accessor, handler, action-input, and resource references are in range for their owning `WorkflowParts` arrays/counts.
- POST-006: Valid admission implies reference kind correctness: symbol refs target symbols, constant refs target constants, action refs target action contracts, handler refs target declared handlers, and slot refs target slots.
- POST-007: Valid admission implies every checked reference is owned by the admitted artifact or by the supplied action-contract set for that validation call; no cross-artifact reference is accepted.
- POST-008: Valid admission implies declared resource limits do not exceed protocol hard limits and actual IR usage does not exceed declared `ResourceContract` limits.
- POST-009: Any failure returns a precise typed error with salient offending ID and location when available; no partially accepted artifact is returned.

## Invariants
- INV-001: Symbol bounds: for every symbol-bearing location `s`, `s.get() < parts.symbols_count`.
- INV-002: Zero-symbol rejection: `parts.symbols_count == 0` rejects any symbol-bearing IR location.
- INV-003: Action-contract bijection: in action-complete mode, unique `Do.action` IDs equal unique supplied `ActionContract.id` values.
- INV-004: Slot bounds: every slot reference is `< parts.slot_count` and refers to a slot owned by the same artifact.
- INV-005: Constant bounds/kind: every constant reference is in range and any symbol-valued constant also satisfies INV-001.
- INV-006: Handler bounds/kind: every handler reference is in range, kind-correct for handler use, and owned by the artifact.
- INV-007: Resource hard limits: no declared `ResourceContract` member exceeds its protocol hard limit.
- INV-008: Resource coverage: actual nodes, slots, constants, accessors, expressions, handlers, and expression stack requirements are `<=` declared limits.
- INV-009: Determinism/purity: validation is a bounded deterministic scan over in-memory IR and supplied contracts with no runtime I/O.
- INV-010: Engineering safety: new source preserves repository constraints: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic.

## Error Taxonomy
- ERR-001: `WorkflowError::SymbolOutOfBounds { symbol, .. }` or precise verifier equivalent when `symbol.get() >= symbols_count`.
- ERR-002: `WorkflowError::ResourceContractTooLarge { resource, .. }` when a declared resource exceeds a hard limit.
- ERR-003: `WorkflowError::ResourceContractExceeded { resource, .. }` when actual usage exceeds the declared contract.
- ERR-004: `ValidationError::ActionContractMissing { action_id, node_index, .. }` when a `Do` action lacks a supplied matching contract.
- ERR-005: `ValidationError::ActionContractOrphan { action_id, .. }` when a supplied contract is unreferenced.
- ERR-006: precise slot reference error when a slot is out of range or wrong-kind for the use site.
- ERR-007: precise constant reference error when a constant index is out of range or wrong-kind for the use site.
- ERR-008: precise handler reference error when a handler index/id is out of range, wrong-kind, or cross-artifact.
- ERR-009: diagnostic rendering/code error if any new validation variant lacks a stable diagnostic code and renderer coverage.

## Contract Signatures
```rust
pub fn validate_symbol_references(parts: &WorkflowParts) -> Result<(), WorkflowError>;
pub fn validate_resource_references(parts: &WorkflowParts) -> Result<(), WorkflowError>;
pub fn validate_action_references(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> Result<(), ValidationError>;
pub fn validate(parts: &WorkflowParts) -> Result<(), ValidationError>;
pub fn validate_with_contracts(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> Result<(), ValidationError>;
impl CompiledWorkflow { pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError>; }
```
All fallible operations are railway-oriented through `Result<T, Error>`.

## Acceptance Criteria Mapping
- AC-001: Out-of-range symbols are rejected in accessor fields, `ConstValue::Symbol`, build-object field keys, and any future symbol-bearing location.
- AC-002: `symbols_count == 0` rejects `SymbolId::new(0)` anywhere.
- AC-003: Slot, constant, handler, action, and resource references are in-range, kind-correct, and owned by the admitted artifact or supplied contract set.
- AC-004: `validate_with_contracts` rejects missing action contracts with exact action ID and node index.
- AC-005: `validate_with_contracts` rejects orphan action contracts.
- AC-006: `validate` continues to skip Gate 12 and cannot claim action-contract completeness.
- AC-007: Resource validation rejects declared-over-hard-limit and actual-usage-over-declared-limit with separate typed errors.
- AC-008: Validation remains pure, deterministic, bounded, and free of runtime YAML/JSON/HTTP.
- AC-009: Diagnostics use precise typed variants, stable codes, and exact enum assertions downstream.
- AC-010: `moon ci` plus verification gauntlet lanes are required evidence after implementation.

## Lean-Owned Clauses
- Lean-owned pure deterministic clauses: INV-001, INV-002, INV-003, INV-004, INV-005, INV-006, INV-007, INV-008, and POST-007 ownership refinement.
- Runtime shell clauses with Lean waivers: PRE-002, PRE-003, PRE-004, PRE-006, POST-004, POST-009, INV-009, INV-010, ERR-001..ERR-009, AC-008..AC-010. See `lean-contract.md`.

## Non-goals
- No production code, tests, proof code, or harness code in this repair.
- No action-contract table is added to `WorkflowParts` by this contract.
- No YAML/string-level `$input`/`$vars` reference validation; this bead is compiled numeric IR validation.
- No performance, vectorization, public API compatibility, or release-provenance claim beyond existing CI governance.
