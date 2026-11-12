# Contract Specification: vb-nsnc

## Bead Scope

- Bead: `vb-nsnc`
- Title: `verifier/runtime: Define capability contract schema`
- Primary artifact under contract: compiled `ActionContract.required_capabilities: Box<[Capability]>`.
- Primary implementation boundary for this bead: cold-path validation in `vb_validate` for action contract capability schema, wired through `ValidationPipeline::validate_with_contracts` and the live `gates::validate_gate_12_action_contract_completeness` path.
- Secondary boundary: diagnostic and CLI/UI error rendering for any new `ValidationError` variants.
- Runtime admission enforcement is specified as a follow-on integration contract only if admission APIs receive accepted artifact capability data. This bead must not move capability checks into runtime hot paths.

## Context Read

- `crates/vb_core/src/capability.rs` defines `Capability { name: Box<str>, action: ActionId }` and `CapabilitySet::grants` semantics: exact name or dotted child name, grant name must be non-empty, and action IDs must match.
- `crates/vb_core/src/action.rs` defines `ActionContract` with `required_capabilities` as the canonical declaration site.
- `crates/vb_validate/src/gates.rs` is the active public gate path re-exported by `shared.rs`; `gate_12_14_15.rs` is test-only/parallel and must not be the only edited path.
- `crates/vb_validate/src/lib.rs` currently has `ActionContractMissing` and `ActionContractOrphan` but no capability schema errors.
- `crates/vb_runtime/src/admission.rs` stores grants and exposes `check_capability`, but `admit_run` lacks contract requirements.
- Master document requires actions to declare required capabilities, operators to grant capabilities, and admission to deny ungranted capabilities. Capability validation remains cold-path.

## Domain Terms

- Capability requirement: a `Capability` inside one `ActionContract.required_capabilities`.
- Capability grant: a `Capability` stored in `RunAdmission.granted_capabilities`.
- Capability name: dotted, hierarchical permission string such as `network.github` or `secrets.read.github_token`.
- Capability action: the `ActionId` scope for a requirement or grant.
- Contract owner action: the enclosing `ActionContract.id` for a required capability.
- Schema-valid capability requirement: a requirement whose name grammar, action relation, and uniqueness satisfy this contract.

## Assumptions

- `ActionId::get()` returns a bounded integer suitable for diagnostics without casts.
- Existing `CapabilitySet::grants` matching is source of truth and must not be changed by this bead.
- Empty required capability lists are valid and mean the action needs no operator grants.
- Capability names are cold-path strings; validation may inspect bytes/chars but must remain bounded.
- The implementation may define constants in `vb_validate`; no runtime JSON, YAML, or HTTP parsing is introduced.

## Open Questions

- Exact maximum capability name length is not present in source. This contract sets `MAX_CAPABILITY_NAME_BYTES = 128` unless the implementation discovers an existing stricter master-contract constant.
- Whether duplicate action contracts for the same `ActionId` are already rejected elsewhere is unclear. This bead only specifies duplicate capability requirements within each `ActionContract`.

## Capability Name Schema

A capability name is valid iff all conditions hold:

1. Byte length is in `1..=128`.
2. It is ASCII only.
3. It is composed of one or more segments separated by a single dot (`.`).
4. Each segment is non-empty.
5. Each segment starts with ASCII lowercase `a..z`.
6. After the first byte, a segment may contain ASCII lowercase `a..z`, ASCII digit `0..9`, or underscore `_`.
7. No leading dot, trailing dot, doubled dot, spaces, hyphens, slashes, colons, uppercase letters, non-ASCII bytes, or control bytes are allowed.

Valid examples: `network`, `network.github`, `secrets.read.github_token`, `fs.read_tmp2`.

Invalid examples: ``, `.network`, `network.`, `network..github`, `Network`, `network-github`, `network:github`, `secrets/read`, `network github`, `netwørk`.

## Invariants

- I1: Every `Do` action referenced by compiled workflow parts has exactly at least one matching `ActionContract` as already enforced by gate 12.
- I2: Every supplied `ActionContract` is used by at least one `Do` node as already enforced by gate 12.
- I3: Every required capability name is schema-valid according to the capability name schema.
- I4: Every required capability action equals the enclosing `ActionContract.id`.
- I5: A single `ActionContract.required_capabilities` list contains no duplicate `(name, action)` pair.
- I6: Schema validation never changes `ActionContract`, `Capability`, `WorkflowParts`, or grant matching semantics.
- I7: Capability schema validation is cold-path only and does not allocate or format in runtime hot execution after run admission.
- I8: All fallible validation APIs return `Result<T, ValidationError>` and never panic.
- I9: First error wins: validation returns the first deterministic capability schema violation in contract iteration order and capability iteration order.
- I10: Existing missing/orphan action contract behavior remains unchanged.

## Preconditions

- P1: Caller passes a fully constructed `WorkflowParts` reference and an action contract slice reference.
- P2: Capability schema validation runs only as part of `validate_with_contracts` or a gate-12 subcheck that has access to `ActionContract` data.
- P3: The implementation treats `required_capabilities` as trusted Rust structs but untrusted content.
- P4: Diagnostic conversion and CLI rendering are updated for every new `ValidationError` variant.

## Postconditions

- Q1: `validate_with_contracts(parts, contracts)` returns `Ok(())` only when existing gate 12 completeness/orphan checks pass and every required capability satisfies I3-I5.
- Q2: Empty `required_capabilities` lists remain accepted when the containing action contract is otherwise valid and used.
- Q3: An empty capability name returns a typed validation error and cannot silently become an impossible-to-grant requirement.
- Q4: A capability whose `action` differs from its enclosing contract id returns a typed validation error.
- Q5: Duplicate capability requirements in one contract return a typed validation error.
- Q6: Invalid name grammar returns a typed validation error that includes action id, capability index, and rejected name.
- Q7: Diagnostics expose stable E05xx verifier codes and human-readable messages for all new variants.
- Q8: Existing `ActionContractMissing` and `ActionContractOrphan` tests still pass without changed semantics.

## Typed Error Taxonomy

Add these variants to `ValidationError` unless an equivalent variant already exists:

- `CapabilityNameEmpty { action_id: usize, capability_index: usize }`
  - When: a required capability name has byte length 0.
  - Diagnostic message: `capability name is empty for action {action_id} at required_capabilities[{capability_index}]`.
  - Suggested code: next verifier code after `NON_DETERMINISTIC_PATH`, e.g. `0x050D`.
- `CapabilityNameTooLong { action_id: usize, capability_index: usize, len: usize, max: usize }`
  - When: name length exceeds `MAX_CAPABILITY_NAME_BYTES`.
  - Diagnostic message: `capability name too long for action {action_id} at required_capabilities[{capability_index}]: {len} > {max}`.
  - Suggested code: `0x050E`.
- `CapabilityNameInvalid { action_id: usize, capability_index: usize, name: String }`
  - When: name is non-empty but violates ASCII/segment/character grammar.
  - Diagnostic message: `invalid capability name for action {action_id} at required_capabilities[{capability_index}]: {name}`.
  - Suggested code: `0x050F`.
- `CapabilityActionMismatch { contract_action_id: usize, capability_action_id: usize, capability_index: usize }`
  - When: a required capability action does not equal the enclosing contract id.
  - Diagnostic message: `capability action {capability_action_id} does not match contract action {contract_action_id} at required_capabilities[{capability_index}]`.
  - Suggested code: `0x0510`.
- `CapabilityDuplicate { action_id: usize, first_index: usize, duplicate_index: usize, name: String }`
  - When: the same `(name, action)` appears more than once inside one action contract.
  - Diagnostic message: `duplicate capability requirement for action {action_id}: {name} at required_capabilities[{first_index}] and required_capabilities[{duplicate_index}]`.
  - Suggested code: `0x0511`.

All variants must be `Debug + Clone + PartialEq + Eq + Error` through the existing enum derive. Every variant must map through `diagnostic.rs`, test diagnostic modules, and CLI-facing formatting if the CLI matches variants explicitly.

## Contract Signatures

The implementation may choose exact visibility, but the fallible contract must be equivalent to:

```rust
pub const MAX_CAPABILITY_NAME_BYTES: usize = 128;

pub fn validate_gate_12_action_contract_completeness(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Result<(), ValidationError>;

fn validate_action_contract_capability_schema(
    contract: &ActionContract,
) -> Result<(), ValidationError>;

fn validate_required_capability(
    contract_action: ActionId,
    capability_index: usize,
    capability: &Capability,
) -> Result<(), ValidationError>;

fn validate_capability_name(
    action_id: ActionId,
    capability_index: usize,
    name: &str,
) -> Result<(), ValidationError>;

fn validate_no_duplicate_capability_requirements(
    contract: &ActionContract,
) -> Result<(), ValidationError>;
```

If admission-time preflight is added in a later bead, it must be a cold-path API that receives declared requirements explicitly:

```rust
pub fn check_required_capabilities(
    requirements: &[Capability],
    granted: &CapabilitySet,
) -> Result<(), AdmissionError>;
```

## Acceptance Criteria

- AC1: `.beads/vb-nsnc/contract.md` is non-empty and records this contract.
- AC2: The implemented validator is reachable from `vb_validate::shared::validate_with_contracts` through the live `gates.rs` path.
- AC3: `gate_12_14_15.rs`, if kept, is either synchronized or not relied on as the only changed implementation.
- AC4: Valid empty capability lists and valid dotted names pass.
- AC5: Empty names, invalid grammar, too-long names, action mismatch, and duplicate requirements fail with specific `ValidationError` variants.
- AC6: Diagnostics and CLI/user-facing rendering cover every new error.
- AC7: `CapabilitySet::grants` behavior remains unchanged: exact or dotted hierarchy match plus same `ActionId`; empty grant names grant nothing.
- AC8: No production code uses `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg`.
- AC9: No runtime core JSON/YAML/HTTP parsing is added.
- AC10: `moon ci` remains the canonical final quality gate for the implementing state.

## Martin Fowler Given/When/Then Scenarios

### Scenario 1: accepts action contract with no required capabilities
Given a compiled workflow with one `Do` node for action `1` and one matching `ActionContract` whose `required_capabilities` is empty.
When `validate_with_contracts` runs.
Then validation succeeds.

### Scenario 2: accepts dotted capability requirement matching contract action
Given a matching `ActionContract` with id `1` and `required_capabilities = [Capability { name: "network.github", action: 1 }]`.
When gate 12 capability schema validation runs.
Then validation succeeds.

### Scenario 3: rejects empty capability name
Given a matching `ActionContract` with id `1` and `required_capabilities = [Capability { name: "", action: 1 }]`.
When gate 12 capability schema validation runs.
Then validation fails with `ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 }`.

### Scenario 4: rejects invalid capability grammar
Given a matching `ActionContract` with id `1` and `required_capabilities = [Capability { name: "network:github", action: 1 }]`.
When gate 12 capability schema validation runs.
Then validation fails with `ValidationError::CapabilityNameInvalid` for action `1`, index `0`, and name `network:github`.

### Scenario 5: rejects too-long capability name
Given a matching `ActionContract` with id `1` and one capability name whose byte length is `129`.
When gate 12 capability schema validation runs.
Then validation fails with `ValidationError::CapabilityNameTooLong { len: 129, max: 128, .. }`.

### Scenario 6: rejects capability action mismatch
Given an `ActionContract` with id `1` and `required_capabilities = [Capability { name: "network", action: 2 }]`.
When gate 12 capability schema validation runs.
Then validation fails with `ValidationError::CapabilityActionMismatch { contract_action_id: 1, capability_action_id: 2, capability_index: 0 }`.

### Scenario 7: rejects duplicate capability requirement in one contract
Given an `ActionContract` with id `1` and two required capabilities with identical name `network` and action `1`.
When gate 12 capability schema validation runs.
Then validation fails with `ValidationError::CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 1, name: "network" }`.

### Scenario 8: preserves missing contract failure precedence
Given a workflow with a `Do` node for action `5` and no matching contract.
When `validate_with_contracts` runs.
Then validation fails with existing `ActionContractMissing` before any capability schema check for absent data.

### Scenario 9: preserves orphan contract failure semantics
Given a workflow with no `Do` node for action `9` and a supplied contract for action `9`.
When `validate_with_contracts` runs.
Then validation fails with existing `ActionContractOrphan` unless an earlier schema violation exists in deterministic contract order; the implementation must document and test the chosen order.

### Scenario 10: diagnostic conversion covers capability schema errors
Given each new capability schema `ValidationError` variant.
When `diagnostic_from_error` and CLI validation rendering run.
Then each returns a stable E05xx code and non-empty human-readable message without panicking.

## Proof Obligations

- PO1: Name grammar proof: tests cover empty, one segment, multiple segments, leading dot, trailing dot, doubled dot, uppercase, hyphen, colon, slash, whitespace, non-ASCII, digit-start segment, underscore-start segment, and max length boundaries 1, 128, 129.
- PO2: Action relation proof: a capability requirement cannot name another action id than the enclosing contract.
- PO3: Duplicate proof: duplicate detection compares both name and action within a single contract and reports deterministic first/duplicate indexes.
- PO4: Pipeline proof: a test proves `shared::validate_with_contracts` invokes the new schema validator, not only a private helper.
- PO5: Regression proof: existing missing/orphan contract tests remain green.
- PO6: Diagnostic proof: every new error variant maps to a diagnostic code and renderable message.
- PO7: Bounded-resource proof: validation loops are finite over provided slices and strings; no recursion, unbounded background tasks, runtime HTTP/YAML/JSON, or hot-path allocation is added.
- PO8: Safety proof: implementation contains no forbidden macros/functions and no unchecked indexing; all slice access is by iteration or checked access.

## Out-of-Scope Boundaries

- Do not implement production code or tests in this State 3 artifact.
- Do not change `CapabilitySet::grants` matching semantics.
- Do not add colon-based, glob, regex, or partial-prefix capability matching.
- Do not add runtime core JSON/YAML/HTTP handling.
- Do not remove execution-time defense-in-depth capability checks in this bead unless a separate architecture bead explicitly owns the migration to admission-only enforcement.
- Do not redesign `ActionContract` fields or action ABI serialization.
- Do not implement accepted artifact persistence or admission API redesign here.

## Risk Notes

- Active gate code is in `gates.rs`; editing only `gate_12_14_15.rs` would leave public validation unchanged.
- Master document says admission-time-only capability checking, while current runtime also checks before dispatch. Removing runtime checks prematurely can create a gap until admission receives declared requirements.
- Adding validation errors without diagnostic and CLI mappings can break UI/CLI integration despite correct validator logic.
- A length bound not already present in code is a contract decision; changing it later affects fixtures and UI snapshots.
- Duplicate requirements are semantically harmless but noisy; rejecting them improves deterministic artifacts and operator grant review.

## Non-Goals

- No performance claim is made by this contract.
- No bead state is closed by this artifact.
- No implementation patch is included.
