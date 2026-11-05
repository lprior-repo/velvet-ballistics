use super::{CollectStates, CollectStatesCheckpointV1, CollectStatesCheckpointV1Error};
use crate::primitives::collect::CollectPaginationState;
use vb_core::ids::{ListId, RunId, SlotIdx};

fn state(run: RunId, collector_slot: SlotIdx) -> CollectPaginationState {
    CollectPaginationState {
        run_id: run,
        collector_slot,
        source: ListId::new(1),
        current_page: ListId::new(3),
        cursor: 4,
        page_size: 5,
        item_count: 10,
        limit: 20,
        time_limit_ms: Some(100),
        start_millis: 77,
    }
}

#[test]
fn collect_checkpoint_roundtrips_canonical_state_and_lineage() -> Result<(), String> {
    let mut states = CollectStates::new();
    states
        .upsert(state(RunId::new(7), SlotIdx::new(2)))
        .map_err(|error| format!("first state setup failed: {error}"))?;
    states
        .upsert(state(RunId::new(3), SlotIdx::new(1)))
        .map_err(|error| format!("second state setup failed: {error}"))?;
    states
        .upsert(CollectPaginationState {
            current_page: ListId::new(8),
            ..state(RunId::new(7), SlotIdx::new(2))
        })
        .map_err(|error| format!("lineage setup failed: {error}"))?;
    states
        .upsert(CollectPaginationState {
            current_page: ListId::new(9),
            ..state(RunId::new(7), SlotIdx::new(2))
        })
        .map_err(|error| format!("lineage setup failed: {error}"))?;

    let checkpoint = states.checkpoint_v1();
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded = postcard::from_bytes(&bytes)
        .map_err(|error| format!("checkpoint decode failed: {error}"))?;
    let restored = CollectStates::from_checkpoint_v1(decoded)
        .map_err(|error| format!("checkpoint restore failed: {error:?}"))?;

    assert_eq!(restored.checkpoint_v1(), checkpoint);
    Ok(())
}

#[test]
fn collect_checkpoint_rejects_orphan_lineage() {
    let checkpoint = CollectStatesCheckpointV1 {
        entries: Vec::new(),
        total_items_consumed: 0,
        total_pages_delivered: 0,
        lineages: vec![super::CollectPageLineageCheckpointV1 {
            run_id: RunId::new(1),
            collector_slot: SlotIdx::new(0),
            previous_page: None,
            stale_pages: Vec::new(),
        }],
    };

    assert_eq!(
        CollectStates::from_checkpoint_v1(checkpoint),
        Err(CollectStatesCheckpointV1Error::OrphanLineage)
    );
}
