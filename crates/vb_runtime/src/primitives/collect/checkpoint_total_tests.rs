use super::super::CollectPaginationState;
use super::{CollectStates, CollectStatesCheckpointV1};
use vb_core::ids::{ListId, RunId, SlotIdx};

fn state(run: RunId, collector_slot: SlotIdx, cursor: usize) -> CollectPaginationState {
    CollectPaginationState {
        run_id: run,
        collector_slot,
        source: ListId::new(1),
        current_page: ListId::new(3),
        cursor,
        page_size: 5,
        item_count: 10,
        limit: 20,
        time_limit_ms: None,
        start_millis: 77,
    }
}

#[test]
fn collect_checkpoint_roundtrips_total_items_consumed() -> Result<(), String> {
    let mut states = CollectStates::new();
    states
        .upsert(state(RunId::new(7), SlotIdx::new(2), 4))
        .map_err(|error| format!("first upsert failed: {error}"))?;
    states
        .upsert(state(RunId::new(3), SlotIdx::new(1), 6))
        .map_err(|error| format!("second upsert failed: {error}"))?;
    states.remove(RunId::new(3), SlotIdx::new(1));
    let checkpoint = states.checkpoint_v1();
    assert_eq!(checkpoint.total_items_consumed, 10);
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded = postcard::from_bytes(&bytes)
        .map_err(|error| format!("checkpoint decode failed: {error}"))?;
    let restored = CollectStates::from_checkpoint_v1(decoded)
        .map_err(|error| format!("checkpoint restore failed: {error:?}"))?;
    assert_eq!(restored.total_items_consumed(), 10);
    assert_eq!(restored.checkpoint_v1(), checkpoint);
    Ok(())
}

#[test]
fn collect_checkpoint_without_total_field_fails_closed() -> Result<(), String> {
    #[derive(serde::Serialize)]
    struct OldCheckpoint {
        entries: Vec<CollectPaginationState>,
        lineages: Vec<super::CollectPageLineageCheckpointV1>,
    }
    let old = OldCheckpoint {
        entries: vec![state(RunId::new(7), SlotIdx::new(2), 4)],
        lineages: Vec::new(),
    };
    let bytes = postcard::to_allocvec(&old)
        .map_err(|error| format!("old checkpoint encode failed: {error}"))?;
    let decoded: Result<CollectStatesCheckpointV1, _> = postcard::from_bytes(&bytes);
    assert_eq!(decoded.is_err(), true);
    Ok(())
}

fn page_state(run: RunId, collector_slot: SlotIdx, current_page: u32) -> CollectPaginationState {
    CollectPaginationState {
        run_id: run,
        collector_slot,
        source: ListId::new(1),
        current_page: ListId::new(current_page),
        cursor: 4,
        page_size: 5,
        item_count: 10,
        limit: 20,
        time_limit_ms: None,
        start_millis: 77,
    }
}

#[test]
fn collect_checkpoint_roundtrips_total_pages_delivered() -> Result<(), String> {
    let mut states = CollectStates::new();
    states
        .upsert(page_state(RunId::new(7), SlotIdx::new(2), 3))
        .map_err(|error| format!("first upsert failed: {error}"))?;
    states
        .upsert(page_state(RunId::new(7), SlotIdx::new(2), 8))
        .map_err(|error| format!("second upsert failed: {error}"))?;
    states
        .upsert(page_state(RunId::new(3), SlotIdx::new(1), 3))
        .map_err(|error| format!("third upsert failed: {error}"))?;
    states.remove(RunId::new(3), SlotIdx::new(1));
    let checkpoint = states.checkpoint_v1();
    assert_eq!(checkpoint.total_pages_delivered, 3);
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded = postcard::from_bytes(&bytes)
        .map_err(|error| format!("checkpoint decode failed: {error}"))?;
    let restored = CollectStates::from_checkpoint_v1(decoded)
        .map_err(|error| format!("checkpoint restore failed: {error:?}"))?;
    assert_eq!(restored.total_pages_delivered(), 3);
    assert_eq!(restored.checkpoint_v1(), checkpoint);
    Ok(())
}

#[test]
fn collect_checkpoint_without_pages_field_fails_closed() -> Result<(), String> {
    #[derive(serde::Serialize)]
    struct OldCheckpoint {
        entries: Vec<CollectPaginationState>,
        lineages: Vec<super::CollectPageLineageCheckpointV1>,
        total_items_consumed: u64,
    }
    let old = OldCheckpoint {
        entries: vec![page_state(RunId::new(7), SlotIdx::new(2), 3)],
        lineages: Vec::new(),
        total_items_consumed: 4,
    };
    let bytes = postcard::to_allocvec(&old)
        .map_err(|error| format!("old checkpoint encode failed: {error}"))?;
    let decoded: Result<CollectStatesCheckpointV1, _> = postcard::from_bytes(&bytes);
    assert!(decoded.is_err());
    Ok(())
}
