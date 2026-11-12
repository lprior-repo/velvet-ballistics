bead_id: vb-qi37.4.3
bead_title: runtime/storage: Persist run header before acknowledgement
phase: State 3 - contract
updated_at: 2026-05-11T00:00:00Z

# Contract Specification

## Context
Persist run identity, workflow digest, admission certificate/capabilities, policy/profile, and initial submitted state before any successful direct/API/CLI/IPC acknowledgement.

## Preconditions
- PRE-001: Caller provides a unique `RunId` and a compiled workflow with stable digest.
- PRE-002: Admission policy either accepts the compiled artifact or returns a typed admission error before runtime state allocation.

## Postconditions
- POST-001: Success is returned only after durable journal/header persistence succeeds.
- POST-002: Successful recovery can reconstruct run header/admission metadata by run id and digest.
- POST-003: A storage failure before header/admission append returns an error and leaves no runnable active state.

## Invariants
- INV-001: No acknowledged run lacks a persisted run header/admission record when admission is required.
- INV-002: Runtime in-memory state insertion happens after persistence of the before-ack records.

## Error Taxonomy
- `RuntimeError::AdmissionArtifactNotFound` for absent accepted artifact.
- `RuntimeError::AdmissionArtifactInvalid` for invalid accepted artifact envelope/gate proof.
- `RuntimeError::AdmissionCapabilityDenied` for missing capability.
- `RuntimeError::StorageJournalAppend` or narrower admission durability error for before-ack persistence failure.

## Contract Signatures
- `fn submit_direct(run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()>`
- `fn handle_submit_with_inputs(run: RunId, workflow: CompiledWorkflow, inputs: &[(SlotIdx, SlotValue)], caps: CapabilitySet) -> RuntimeResult<()>`

## Non-goals
- No performance claim beyond not adding unbounded work.
