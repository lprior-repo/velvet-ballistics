# vb-qi37.6 Contract Specification

## Startup citations

- `/home/lewis/.agents/skills/rust-contract/SKILL.md` lines 12-26 require contract-first design, TLA+ for temporal behavior, Verus-first Rust core obligations, machine-readable proof obligations, review, and no implementation/proof/test code. `.agents` wins.
- `/home/lewis/.claude/skills/rust-contract/SKILL.md` contains the same version 2.6.0 content and rules.

## Context

- Bead: `vb-qi37.6`, verifier/runtime capability model enforcement.
- Workspace: `/home/lewis/src/vb-qi37-6` only.
- Inputs read: State 2 `STATE.md`, `baseline-report.md`, `codebase-map.md`, and `delivery-scope.jsonl`.
- Current code facts: `CapabilitySet::grants` is exact-name and exact-action only; runtime admission requires gate count 15; storage `submit_artifact` writes gate count 2 and empty `AcceptedArtifact.required_capabilities`; public `Runtime` submit APIs pass `CapabilitySet::empty()`; shard drive forwards `&[]` action contracts; UI view carries `required_capabilities`.

## Domain terms

- Required capability: `(name, action_id)` pair declared by an `ActionContract` and persisted into `AcceptedArtifact.required_capabilities`.
- Grant: caller-provided `(name, action_id)` in a `CapabilitySet`.
- Exact grant: grant name equals required name byte-for-byte and grant action equals required action.
- Hierarchical grant: parent prefix such as `network` for `network.github`; forbidden by this contract.
- Accepted artifact: storage record containing digest, IR bytes, verification proof, sequence, and required capabilities.

## Assumptions

- Strict least privilege is intentional: no hierarchical/prefix grants and no extra grants at runtime admission.
- Strict/Journaled runtime admission remains fail-closed until storage and runtime agree on canonical gate count.
- Relaxed policy may skip accepted-artifact capability checks but must not be used as evidence for Strict/Journaled capability enforcement.

## Preconditions

- PRE-001: Every non-Relaxed submitted workflow artifact must have a persisted `AcceptedArtifact` envelope for its digest.
- PRE-002: Every `AcceptedArtifact.verification.gate_count` admitted under Strict/Journaled runtime policy must equal the canonical gate count `15`; any other value, including storage's current `2`, is rejected.
- PRE-003: Every `AcceptedArtifact.required_capabilities` value must be derived from validated `ActionContract.required_capabilities`, not defaulted to empty when any action requires capabilities.
- PRE-004: Every public runtime submit path for Strict/Journaled capability-protected workflows must accept or otherwise bind a non-empty caller grant set before admission.
- PRE-005: Every runtime Do execution path must receive the validated action-contract slice corresponding to the compiled workflow.
- PRE-006: UI `ActionDescriptionView.required_capabilities` must come from the same action-contract source as storage persistence and runtime enforcement.

## Postconditions

- POST-001: `CapabilitySet::grants(required)` returns true only for exact name equality and exact action equality; hierarchical, partial-prefix, sibling-prefix, empty-name, and action-mismatch grants return false.
- POST-002: Strict/Journaled admission returns `ArtifactInvalidGateCount` and allocates no run frame when the accepted artifact gate count is not `15`.
- POST-003: Strict/Journaled admission returns `CapabilityDenied` and allocates no run frame when grant cardinality differs from required-capability cardinality.
- POST-004: Strict/Journaled admission returns `CapabilityDenied` and allocates no run frame when any required capability lacks an exact grant.
- POST-005: Successful Strict/Journaled admission stores the admitted digest, run id, policy, and exact granted capabilities in `RunAdmission` and journals `RunAdmission` only after admission succeeds.
- POST-006: Do execution with a contract checks all required capabilities before emitting an action ticket.
- POST-007: Do execution without an action contract fails closed with `CapabilityDenied` requiring `__contract_required__` and does not produce `AwaitingAction`.
- POST-008: Legacy admission/existence-only paths must not bypass denial or delegation for Strict/Journaled runtime submit flows.
- POST-009: UI action descriptions serialize the same required capability set enforced by storage/runtime.

## Invariants

- INV-001: Capability grant semantics are exact-only; no parent, child, lexical prefix, sibling prefix, or empty-name grant confers authority.
- INV-002: Runtime least privilege is cardinality-exact; extra grants and missing grants are both denial cases.
- INV-003: Strict/Journaled accepted-artifact gate count is a single runtime/storage contract value; mismatch fails closed until repaired.
- INV-004: Required capabilities are never silently erased between action-contract validation, accepted-artifact persistence, admission, shard state, engine execution, and UI projection.
- INV-005: Admission denial is atomic: no run frame, no run state insertion, and no `RunAdmission` journal event.
- INV-006: Shard drive must never execute a Do node by bypassing contracts; missing contracts deny by construction.
- INV-007: Legacy `admit_run` / artifact-exists-only APIs are not acceptable evidence for capability-protected Strict/Journaled admission.
- INV-008: Public Runtime APIs expose a grant path or reject capability-protected workflows; `CapabilitySet::empty()` is valid only for artifacts with zero required capabilities.

## Error taxonomy

- `AdmissionError::ArtifactInvalidGateCount { found, required }`: PRE-002 / INV-003 violation.
- `AdmissionError::CapabilityDenied { action, required, granted }`: missing, extra, hierarchical, action-mismatch, no-contract, or count-mismatch denial.
- `AdmissionError::ArtifactEnvelopeDecodeFailed`: persisted accepted artifact cannot be decoded.
- `AdmissionError::ArtifactInvalidProofFlag { flag }`: required proof flag false.
- `RuntimeError::AdmissionCapabilityDenied`: shard-level mapping of admission denial.
- `EngineError::CapabilityDenied`: engine-level Do denial.

## Contract signatures

- `fn grants(granted: &CapabilitySet, required: &Capability) -> bool`
- `fn check_capability(action: ActionId, required: &Capability, granted: &CapabilitySet) -> Result<(), AdmissionError>`
- `fn submit_artifact(journal: &FjallJournal, workflow: &CompiledWorkflow, policy: RuntimePolicy) -> Result<AcceptedArtifact, JournalError>`
- `fn admit_artifact_run(store: &dyn AcceptedArtifactStore, policy: RuntimePolicy, run_id: RunId, digest: WorkflowDigest, caps: CapabilitySet) -> Result<RunAdmission, AdmissionError>`
- `fn execute_do(..., registry_contracts: &[ActionContract], granted: &CapabilitySet, ...) -> Result<RuntimeSignal, RuntimeEngineError>`
- `fn execute_do_without_contract(..., granted: &CapabilitySet, ...) -> Result<RuntimeSignal, RuntimeEngineError>`

## Verus-owned clauses

- INV-001, INV-002, POST-001, POST-003, POST-004: pure capability matching/cardinality model.
- PRE-003, INV-004: pure extraction model from action contracts to required-capability multiset.

## TLA+-owned clauses

- INV-003, INV-005, INV-006, INV-008, POST-002, POST-005, POST-006, POST-007, POST-008: admission-to-run lifecycle and fail-closed sequencing.

## Theorem-owned clauses

- None required beyond Verus. Lean is reserved only if Verus cannot express the exact grant lattice/cardinality theorem after proof-writing discovery.

## Non-goals

- No hierarchical grants.
- No wildcard grants.
- No proof claim is PASS in State 3; all verification rows are planned.
