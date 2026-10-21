# vb-3l40a Admission Contract Addendum: Workflow Binding Validation

## Objective
Extend `validate_submit_admission_with_contracts` to validate action contract IDs against the actual Do actions in the submitted workflow, ensuring contracts cannot be fabricated with arbitrary IDs.

## Evidence
- Master §18 (Action ABI): Compile-time `ActionId`, ticket/outcome model, static numeric dispatch.
- Master §51 (Digest Canonicalization): Action contracts sorted by `ActionId`.
- Production `resolve_contract` (crates/vb_runtime/src/engine/action.rs): Uses `contracts.get(usize::from(action.get())).filter(|c| c.id == action)`. This requires index == ID and rejects ID mismatches.

## Registry Model
The action contract registry is a dense, sorted array where index == ActionId:
- Registry index must equal the action ID (numeric binding).
- A valid dense registry may contain unused registered IDs (contracts for actions not referenced by a particular workflow are allowed).
- Every referenced Do action ID must resolve to a contract at its exact index.
- Fabricated/sparse/reordered IDs are invalid.
- Reused Do action IDs (same action appearing multiple times in a workflow) are valid; a single contract covers all occurrences.

## Exact ID Matching Requirement
- Each referenced `ActionId` must find a contract at `contracts[usize::from(action.get())]` with `contract.id == action`.
- A contract with `ActionId::new(999)` at index 0 is rejected (index != ID).
- A contract with `ActionId::new(0)` at index 0 is accepted even if the workflow never references action 0.
- Missing a contract for a referenced Do action is rejected.

## Duplicated Action IDs
- Duplicate contract IDs (same ID at multiple indices, or same ID at multiple positions in the array) are rejected before workflow binding checks.
- Error: `RuntimeError::ContractValidationError { action, error: "duplicate action ID" }`

## Missing / Extra Contracts
- Missing contract for a referenced Do action: rejected.
- Extra contract for an unused action ID: allowed (dense registry may have unused entries).
- Fabricated action ID (contract ID doesn't match its index): rejected.
- Error for missing/fabricated: `RuntimeError::ContractValidationError { action, error: ... }`

## Capability Error Ordering
Validation order (first error wins):
1. Duplicate action IDs in contracts
2. Empty action contract names
3. Capability coverage (granted vs required)
4. Action ID workflow binding (referenced Do actions must have contracts)

## Direct Submission Paths
Both public with-contract ingress paths must validate before enqueue:
- `submit_direct_with_grants_and_contracts`
- `submit_direct_with_inputs_grants_and_contracts`

Validation occurs in `validate_submit_admission_with_contracts` before any ShardCommand is enqueued.

## Fail Before Enqueue / No Journal Mutation
- On any validation error, fail immediately before enqueuing any command.
- No run state, journal events, or admission records are created on failure.
- The rejection is purely pre-flight; the runtime observes no mutation.

## Preserve Legitimate Reused Actions
- The same action ID appearing in multiple workflow Do nodes is legitimate.
- A single contract for that action ID covers all occurrences.
- Only missing contracts for referenced actions or fabricated IDs are rejected.

## API Change
```rust
// Before
pub(crate) fn validate_submit_admission_with_contracts(
    &self,
    run: RunId,
    digest: WorkflowDigest,
    caps: CapabilitySet,
    action_contracts: &[ActionContract],
) -> RuntimeResult<()>;

// After
pub(crate) fn validate_submit_admission_with_contracts(
    &self,
    run: RunId,
    workflow: &CompiledWorkflow,
    caps: CapabilitySet,
    action_contracts: &[ActionContract],
) -> RuntimeResult<()>;
```

The runtime submit methods now pass `&workflow` instead of `workflow.digest()`.
