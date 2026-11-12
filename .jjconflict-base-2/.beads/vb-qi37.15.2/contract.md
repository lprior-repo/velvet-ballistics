bead_id: vb-qi37.15.2
bead_title: cli: Add submit command and job ledger
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# Contract Specification

## Preconditions
- PRE-001: Workflow and input-bin paths exist and are readable.
- PRE-002: `--db` names a writable journal location.
- PRE-003: Durability mode is one of strict, journaled, none.

## Postconditions
- POST-001: `submit` persists workflow/run metadata before reporting success.
- POST-002: Successful output includes run identifier, workflow digest, step count, durability/status.
- POST-003: Later `inspect`/`events` can observe submitted run ledger data.

## Invariants
- INV-001: No success is printed before required ledger writes succeed.
- INV-002: Bad profiles/sinks/paths fail non-interactively with diagnostics.

## Error Taxonomy
- ERR-001: Input read failure.
- ERR-002: Workflow compile failure.
- ERR-003: Journal open/write failure.
- ERR-004: Unknown durability/profile/sink argument.

## Contract Signatures
- submit_workflow(workflow: &Path, input_bin: &Path, db: &Path, durability: DurabilityMode) -> Result<SubmitReceipt, SubmitError>

## TLA+-Owned Clauses
- TLA-POST-001 models submit ledger ordering: request accepted only after metadata persisted.

## Verus-Owned Clauses
- None for current shell/storage adapter scope; runtime storage API invariants remain in `vb_storage` tests and formal follow-ups.
