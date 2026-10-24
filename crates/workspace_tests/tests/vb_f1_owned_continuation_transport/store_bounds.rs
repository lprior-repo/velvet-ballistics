use super::support::{self, TestResult};
use vb_core::value_store::transport::{
    CheckpointShape, CheckpointTransportBudget, CheckpointTransportError, CheckpointWireCursor,
    preflight_checkpoint,
};
use vb_core::{ListId, SlotValue, Taint, ValueStore};
use vb_runtime::run_continuation::{RunContinuationError, RunContinuationV1, TransportFailure};

#[test]
fn aggregate_arena_counts_are_checked_before_owned_deserialization() -> TestResult {
    let fixture = support::at_cut(false)?;
    let mut checkpoint = fixture.state.store.checkpoint_v1();
    let cap = usize::try_from(checkpoint.max_arena_entries)?;
    while checkpoint
        .symbols
        .len()
        .checked_add(checkpoint.lists.len())
        .and_then(|n| n.checked_add(checkpoint.objects.len()))
        .and_then(|n| n.checked_add(checkpoint.blobs.len()))
        .ok_or("count overflow")?
        <= cap
    {
        checkpoint.symbols.push("".into());
    }
    assert!(checkpoint.symbols.len() <= cap);
    let bytes = postcard::to_allocvec(&checkpoint)?;
    let mut cursor = CheckpointWireCursor::new(&bytes);
    let mut budget = CheckpointTransportBudget::new(67_108_864, 16_777_216)?;
    let shape = CheckpointShape {
        arena_entries: cap,
        blob_bytes: 16_777_216,
    };
    assert_eq!(
        preflight_checkpoint(&mut cursor, &mut budget, shape),
        Err(CheckpointTransportError::Bound("arena_total"))
    );
    assert!(budget.usage().allocation_bytes < 67_108_864);
    Ok(())
}

#[test]
fn existing_cycle_and_depth_validators_remain_authoritative_after_preflight() -> TestResult {
    for cycle in [false, true] {
        let mut fixture = support::at_cut(false)?;
        let authority = fixture.authority()?;
        let mut checkpoint = fixture.state.store.checkpoint_v1();
        let count = if cycle { 1 } else { 65 };
        for _ in 0..count {
            let id = u32::try_from(checkpoint.lists.len())?;
            let target = if cycle {
                id
            } else {
                id.checked_sub(1).ok_or("list predecessor")?
            };
            checkpoint
                .lists
                .push(Box::from([SlotValue::List(ListId::new(target))]));
            checkpoint.list_taints.push(Box::from([Taint::Clean]));
        }
        fixture.state.store = ValueStore::from_checkpoint_v1(checkpoint)?;
        let failure = RunContinuationV1::capture_transport(
            &fixture.state,
            &authority,
            super::golden::limits(1_048_576)?,
        )
        .err()
        .ok_or("invalid graph accepted")?;
        assert!(failure.strong_decode_started);
        let TransportFailure::Strong(RunContinuationError::RuntimeStore(error)) = failure.failure
        else {
            return Err("lost existing graph error".into());
        };
        match (*error, cycle) {
            (vb_core::root_bundle::OwnedRootBundleError::GraphCycle { .. }, true)
            | (vb_core::root_bundle::OwnedRootBundleError::GraphDepth { .. }, false) => {}
            _ => return Err("wrong graph refusal".into()),
        }
    }
    Ok(())
}
