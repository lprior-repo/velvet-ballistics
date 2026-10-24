// checkpoint_blob.rs
// Tests OwnedRootBundle checkpoint validation: list taint mismatch, blob limit, exact-bound success.

use bytes::Bytes;
use vb_core::errors::CoreError;
use vb_core::ids::{BlobId, SlotIdx};
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn list_taint_count_mismatch() -> TestResult<()> {
    // Given: workflow with one I64 input slot
    let inputs = vec![CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::I64,
    }];
    let wf = fixture(1, inputs, 8, 4)?;

    // Given: checkpoint with one list but no corresponding list_taints entry
    let mut cp = checkpoint(8);
    cp.lists.push(Box::from([SlotValue::Null]));
    // list_taints intentionally empty -> mismatch

    // When: creating bundle with a scalar root
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);
    let result = OwnedRootBundle::new(roots, cp, &wf);

    // Then: Checkpoint error with InternalInvariantViolation for list_taint_count_mismatch
    let err = result.err().ok_or("expected error")?;
    match err {
        OwnedRootBundleError::Checkpoint(CoreError::InternalInvariantViolation { reason }) => {
            if reason != "list_taint_count_mismatch" {
                return Err(format!("wrong reason: {reason}").into());
            }
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn blob_limit_enforced() -> TestResult<()> {
    // Given: workflow with one I64 input slot, blob cap 4
    let inputs = vec![CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::I64,
    }];
    let wf = fixture(1, inputs, 8, 4)?;

    // Given: checkpoint with a 5-byte blob exceeding the 4-byte limit
    let mut cp = checkpoint(8);
    cp.blobs.push(Bytes::from_static(b"12345"));

    // When: creating bundle with a scalar root
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);
    let result = OwnedRootBundle::new(roots, cp, &wf);

    // Then: Checkpoint error with BlobLimit for blob 0
    let err = result.err().ok_or("expected error")?;
    match err {
        OwnedRootBundleError::BlobLimit { blob, len, limit } => {
            if blob != BlobId::new(0_u64) || len != 5 || limit != 4 {
                return Err(format!("wrong blob limit fields: {blob:?} {len} {limit}").into());
            }
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn exact_bound_blob_success() -> TestResult<()> {
    // Given: workflow with one Blob input slot, blob cap 4
    let inputs = vec![CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::Blob,
    }];
    let wf = fixture(1, inputs.clone(), 8, 4)?;

    // Given: checkpoint with a 4-byte blob at the exact limit
    let mut cp = checkpoint(8);
    cp.blobs.push(Bytes::from_static(b"1234"));
    let saved_cp = cp.clone();

    // When: creating bundle with a blob root referencing that blob
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::Blob(BlobId::new(0_u64)),
        Taint::Clean,
    )]);
    let bundle = OwnedRootBundle::new(roots, cp, &wf)?;

    // Then: bundle has the expected roots
    let expected_roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([(
        SlotIdx::new(0),
        SlotValue::Blob(BlobId::new(0_u64)),
        Taint::Clean,
    )]);
    if bundle.roots() != expected_roots.as_ref() {
        return Err("roots mismatch".into());
    }

    // Then: bundle checkpoint matches saved checkpoint
    if bundle.checkpoint() != &saved_cp {
        return Err("checkpoint mismatch".into());
    }

    // Then: bundle matches the workflow
    if !bundle.matches_workflow(&wf) {
        return Err("workflow mismatch".into());
    }

    Ok(())
}
