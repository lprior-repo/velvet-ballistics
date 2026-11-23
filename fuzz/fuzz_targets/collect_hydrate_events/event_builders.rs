use super::*;

pub(super) fn collect_envelope_event(
    run: RunId,
    slot: SlotIdx,
    seq: EventSeq,
    _data: &[u8],
    _i: usize,
) -> Option<JournalEvent> {
    let page = ListId::new(1);
    let value = match postcard::to_allocvec(&SlotValue::List(page)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    let pagination = CollectPaginationState {
        run_id: run,
        collector_slot: slot,
        source: ListId::new(1),
        current_page: ListId::new(1),
        cursor: 0,
        page_size: 8,
        item_count: 0,
        limit: 64,
        time_limit_ms: None,
        start_millis: 0,
    };
    let encoded = match postcard::to_allocvec(&pagination) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let extra = match vb_storage::encode_slot_written_extra(Taint::Clean, Some(encoded)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt: 1,
    })
}

pub(super) fn collect_legacy_event(
    run: RunId,
    slot: SlotIdx,
    seq: EventSeq,
) -> Option<JournalEvent> {
    let page = ListId::new(1);
    let value = match postcard::to_allocvec(&SlotValue::List(page)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    // Legacy: just pagination state, no envelope prefix
    let pagination = CollectPaginationState {
        run_id: run,
        collector_slot: slot,
        source: ListId::new(1),
        current_page: ListId::new(1),
        cursor: 0,
        page_size: 8,
        item_count: 0,
        limit: 64,
        time_limit_ms: None,
        start_millis: 0,
    };
    let extra = postcard::to_allocvec(&pagination).ok();
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt: 1,
    })
}

pub(super) fn collect_corrupt_event(
    run: RunId,
    slot: SlotIdx,
    seq: EventSeq,
) -> Option<JournalEvent> {
    let value = Some(vec![1, 2, 3]);
    let mut extra = vb_storage::SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    extra.push(0xFF);
    extra.push(0x00);
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra: Some(extra),
        attempt: 1,
    })
}

pub(super) fn non_collect_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let value = Some(postcard::to_allocvec(&SlotValue::I64(42)).ok()?);
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra: None,
        attempt: 1,
    })
}

pub(super) fn mismatched_run_event(
    run: RunId,
    slot: SlotIdx,
    seq: EventSeq,
) -> Option<JournalEvent> {
    let page = ListId::new(1);
    let value = match postcard::to_allocvec(&SlotValue::List(page)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    let pagination = CollectPaginationState {
        run_id: RunId::new(999),
        collector_slot: slot,
        source: ListId::new(1),
        current_page: ListId::new(1),
        cursor: 0,
        page_size: 8,
        item_count: 0,
        limit: 64,
        time_limit_ms: None,
        start_millis: 0,
    };
    let encoded = match postcard::to_allocvec(&pagination) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let extra = match vb_storage::encode_slot_written_extra(Taint::Clean, Some(encoded)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt: 1,
    })
}

pub(super) fn mismatched_slot_event(
    run: RunId,
    slot: SlotIdx,
    seq: EventSeq,
) -> Option<JournalEvent> {
    let page = ListId::new(1);
    let value = match postcard::to_allocvec(&SlotValue::List(page)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    let pagination = CollectPaginationState {
        run_id: run,
        collector_slot: SlotIdx::new(999),
        source: ListId::new(1),
        current_page: ListId::new(1),
        cursor: 0,
        page_size: 8,
        item_count: 0,
        limit: 64,
        time_limit_ms: None,
        start_millis: 0,
    };
    let encoded = match postcard::to_allocvec(&pagination) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let extra = match vb_storage::encode_slot_written_extra(Taint::Clean, Some(encoded)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt: 1,
    })
}

pub(super) fn empty_extra_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let value = Some(postcard::to_allocvec(&SlotValue::I64(42)).ok()?);
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra: Some(vec![]),
        attempt: 1,
    })
}

pub(super) fn truncated_extra_event(
    run: RunId,
    slot: SlotIdx,
    seq: EventSeq,
) -> Option<JournalEvent> {
    let value = Some(postcard::to_allocvec(&SlotValue::I64(42)).ok()?);
    let extra = vb_storage::SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra: Some(extra),
        attempt: 1,
    })
}
