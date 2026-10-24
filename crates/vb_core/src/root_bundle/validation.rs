//! Owned root bundle constructor validation phases 1-3b.
//!
//! Validates cap consistency, arena entry counts, checkpoint integrity,
//! and per-blob workflow caps before root slot and graph validation.

use crate::errors::CoreError;
use crate::ids::BlobId;
use crate::limits::MAX_BLOB_BYTES_PER_VALUE;
use crate::root_bundle::errors::OwnedRootBundleError;
use crate::value_store::ValueStoreCheckpointV1;
use crate::value_store::validate_checkpoint;
use crate::workflow::CompiledWorkflow;

/// Validate cap consistency, arena counts, checkpoint, and blob caps.
///
/// Enforces phases 1, 2, 3, and 3b in strict order.
pub(super) fn preflight(
    checkpoint: &ValueStoreCheckpointV1,
    workflow: &CompiledWorkflow,
) -> Result<(), OwnedRootBundleError> {
    phase1_cap_nonzero_equal(checkpoint, workflow)?;
    phase2_arena_count(checkpoint, workflow)?;
    phase3_checkpoint(checkpoint)?;
    phase3b_blob_caps(checkpoint, workflow)?;
    Ok(())
}

/// Phase 1: Cap nonzero and equal.
fn phase1_cap_nonzero_equal(
    checkpoint: &ValueStoreCheckpointV1,
    workflow: &CompiledWorkflow,
) -> Result<(), OwnedRootBundleError> {
    let checkpoint_cap = checkpoint.max_arena_entries;
    let workflow_max = u64::from(workflow.resource_contract().max_slots);

    if checkpoint_cap == 0 || workflow_max == 0 {
        return Err(OwnedRootBundleError::CapMismatch {
            checkpoint_cap,
            workflow_max_slots: workflow_max,
        });
    }

    if checkpoint_cap != workflow_max {
        return Err(OwnedRootBundleError::CapMismatch {
            checkpoint_cap,
            workflow_max_slots: workflow_max,
        });
    }

    Ok(())
}

fn convert_arena_len(len: usize) -> Result<u64, OwnedRootBundleError> {
    u64::try_from(len).map_err(|_| OwnedRootBundleError::ArenaCountOverflow)
}

fn arena_counts(checkpoint: &ValueStoreCheckpointV1) -> Result<[u64; 4], OwnedRootBundleError> {
    Ok([
        convert_arena_len(checkpoint.symbols.len())?,
        convert_arena_len(checkpoint.lists.len())?,
        convert_arena_len(checkpoint.objects.len())?,
        convert_arena_len(checkpoint.blobs.len())?,
    ])
}

/// Phase 2: Arena count within cap.
fn phase2_arena_count(
    checkpoint: &ValueStoreCheckpointV1,
    workflow: &CompiledWorkflow,
) -> Result<(), OwnedRootBundleError> {
    let counts = arena_counts(checkpoint)?;
    let total = checked_arena_count(counts)?;
    let cap = u64::from(workflow.resource_contract().max_slots);

    if total > cap {
        return Err(OwnedRootBundleError::ArenaLimit {
            count: total,
            limit: cap,
        });
    }

    Ok(())
}

/// Phase 3: Checkpoint validation.
fn phase3_checkpoint(checkpoint: &ValueStoreCheckpointV1) -> Result<(), OwnedRootBundleError> {
    validate_checkpoint(checkpoint).map_err(OwnedRootBundleError::Checkpoint)
}

fn convert_blob_len(len: usize) -> Result<u64, OwnedRootBundleError> {
    u64::try_from(len).map_err(|_| {
        OwnedRootBundleError::Checkpoint(CoreError::InternalInvariantViolation {
            reason: "blob length exceeds u64",
        })
    })
}

fn convert_blob_index(index: usize) -> Result<BlobId, OwnedRootBundleError> {
    Ok(BlobId::new(u64::try_from(index).map_err(|_| {
        OwnedRootBundleError::Checkpoint(CoreError::InternalInvariantViolation {
            reason: "blob index exceeds u64",
        })
    })?))
}

fn compute_blob_limit(workflow_cap: u64) -> Result<u64, OwnedRootBundleError> {
    let max = u64::try_from(MAX_BLOB_BYTES_PER_VALUE).map_err(|_| {
        OwnedRootBundleError::Checkpoint(CoreError::InternalInvariantViolation {
            reason: "MAX_BLOB_BYTES_PER_VALUE exceeds u64",
        })
    })?;
    Ok(if workflow_cap < max {
        workflow_cap
    } else {
        max
    })
}

fn validate_blob(blob: &[u8], index: usize, limit: u64) -> Result<(), OwnedRootBundleError> {
    let blob_id = convert_blob_index(index)?;
    let blob_len = convert_blob_len(blob.len())?;
    if blob_len > limit {
        return Err(OwnedRootBundleError::BlobLimit {
            blob: blob_id,
            len: blob_len,
            limit,
        });
    }
    Ok(())
}

/// Phase 3b: Per-blob workflow cap.
fn phase3b_blob_caps(
    checkpoint: &ValueStoreCheckpointV1,
    workflow: &CompiledWorkflow,
) -> Result<(), OwnedRootBundleError> {
    let limit = compute_blob_limit(workflow.resource_contract().max_blob_bytes)?;
    for (index, blob) in checkpoint.blobs.iter().enumerate() {
        validate_blob(blob, index, limit)?;
    }
    Ok(())
}

/// Pure checked arena count computation for parameterized overflow tests.
pub fn checked_arena_count(counts: [u64; 4]) -> Result<u64, OwnedRootBundleError> {
    let mut total = 0_u64;
    for count in counts {
        total = total
            .checked_add(count)
            .ok_or(OwnedRootBundleError::ArenaCountOverflow)?;
    }
    Ok(total)
}
