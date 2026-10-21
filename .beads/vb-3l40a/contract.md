# vb-3l40a Runtime Compile Repair Contract

## Objective
Restore `vb_runtime` crate compilation after command and input-slot API drift from vb-eyij7 migration. Preserve recovery, admission, persistence-before-ack, input slot admission, and action ABI digest semantics. Do not stub methods, drop fields, disable tests, or broaden lint allowances.

## Known Compile Errors (22 errors, 2 warnings)
From `cargo check -p vb_runtime` baseline on origin/main.

### Error Group 1: Missing RecoverRunCommand Type (E0425/E0422)
- **Location**: `recovery/full/product.rs:239` and `240`
- **Issue**: `recovery::ResumableRecoveryProduct::into_recover_command()` returns `crate::shard::types::RecoverRunCommand` which does not exist.
- **Intended API**: A typed recovery command struct carrying run, frame, artifact_digest, workflow_digest, next_seq, collect_states, boundary.
- **Contract**: Define `RecoverRunCommand` in `shard/types.rs` with fields:
  - `run: RunId`
  - `frame: RunFrame`
  - `artifact_digest: WorkflowDigest`
  - `workflow_digest: WorkflowDigest`
  - `next_seq: EventSeq`
  - `collect_states: CollectStates`
  - `boundary: RecoveredRunBoundary`

### Error Group 2: validate_submit_admission_with_contracts Missing (E0599)
- **Location**: `runtime.rs:274` and `298`
- **Issue**: Runtime calls `shard.validate_submit_admission_with_contracts(run, digest, caps, &action_contracts)` but Shard only has `validate_submit_admission`.
- **Intended API**: Admission validation with action contract validation for submit-with-contracts paths.
- **Contract**: Add `Shard::validate_submit_admission_with_contracts(run, digest, caps, action_contracts) -> RuntimeResult<()>` to shard lifecycle. Builds admission like existing method but also validates action contract IDs against the workflow.

### Error Group 3: ShardCommand::Recover Field Drift (E0559 x4)
- **Location**: `runtime.rs:675-679`
- **Issue**: `enqueue_recovered_product` tries to construct `ShardCommand::Recover` with `artifact_digest`, `next_seq`, `collect_states`, `boundary` fields that do not exist on the variant.
- **Current variant fields**: `run`, `frame`, `workflow_digest`
- **Intended fields**: `run`, `frame`, `artifact_digest`, `workflow_digest`, `next_seq`, `collect_states`, `boundary`
- **Contract**: Expand `ShardCommand::Recover` to carry full recovery context. The shard's Recover handler must use all these fields for correct recovery: artifact for workflow lookup, journal seq for persistence, collect states for pagination, boundary for recovery classification.

### Error Group 4: WorkflowParts Missing input_slots in Tests (E0063 x9)
- **Location**: Multiple `shard/tests/chunk_*` files
- **Issue**: Test code constructs `WorkflowParts` without the `input_slots` field which exists in `vb_core`.
- **Intended API**: `input_slots: Box<[CompiledInputSlot]>` is a required field (Serde default only for backward compatibility).
- **Contract**: All test constructors of `WorkflowParts` must provide `input_slots: Box::new([])` or appropriate slot definitions.

### Error Group 5: action_abi_digest Methods Missing (E0599 x3)
- **Location**: `transitions.rs:113`, `transitions.rs:142`, `transitions.rs:234` and `lifecycle/chunk_002_parts/chunk_002_drive_core.rs:142`
- **Issue**: Calls to `self.action_abi_digests_remove(run)` and `self.action_abi_digest_for_run_action(run, action_id)` but methods do not exist.
- **Intended API**: 
  - `action_abi_digests_remove(run)` - cleanup of per-run action ABI tracking when run transitions to terminal
  - `action_abi_digest_for_run_action(run, action)` - compute ABI digest for an action on a run from its action contract
- **Contract**: Implement both methods on Shard. ABI digest = blake3(postcard(ActionContract)). Lookup action contract by ActionId from the run's action_contracts, compute and return digest.

### Error Group 6: RuntimeInputSlotKindMismatch Not Matched (E0004 x2)
- **Location**: `error/diagnostics.rs:52`, `error/diagnostics.rs:127`
- **Issue**: Match on RuntimeError is non-exhaustive; `RuntimeInputSlotKindMismatch { .. }` variant is defined but not handled in diagnostics formatting.
- **Intended API**: Error variant for input slot type mismatches, formatted as "input_slot_kind_mismatch" in diagnostics.
- **Contract**: Add match arms in both `error_code()` and `error_category()` methods.

### Error Group 7: SlotValue Non-Exhaustive Match (E0004)
- **Location**: `shard/helpers.rs:61`
- **Issue**: Match on non_exhaustive `SlotValue` enum is missing catch-all `_` arm.
- **Intended API**: `slot_value_kind(value)` should return `InputSlotKind` for all SlotValue variants including future ones.
- **Contract**: Add `&_ => InputSlotKind::Blob` (or appropriate default) as fallback match arm.

## Safety Invariants
- `RecoverRunCommand` is constructed only by recovery machinery from verified journal replay.
- Action ABI digest is computed from serialized contract bytes, ensuring contract versioning.
- Recovery fields (next_seq, collect_states, boundary) enable correct resumption semantics.
- Input slot validation prevents schema drift between compiled workflow and submitted values.
- No new behavior; only restoring intended API surface that drifted during migration.

## Testing Strategy
- Restore compilation as first gate.
- Run shard-level tests that exercise recovery paths, admission with contracts, and action ABI checking.
- Verify input slot admission rejects mismatched slot kinds.
- No formal proofs, mutation testing, or broad Kani/Verus runs.
