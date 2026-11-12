#![forbid(unsafe_code)]
//! Literal effect oracles. Handles come from validated frame payloads, never events.

use super::{checked, fixture::Family};
use crate::engine::types::EvidenceEvent;
use crate::primitives::collect::{CollectPaginationState, CollectStates};
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;

pub(super) fn verify(
    family: Family,
    productive: bool,
    run: &RunFrame,
    store: &ValueStore,
    source: ListId,
    collect: &CollectStates,
    events: &[EvidenceEvent],
) -> Result<(), String> {
    // Each branch checks actual values, taints and payloads before event comparison.
    let writes = match family {
        Family::ForEach => for_each_writes(productive, run, store, source)?,
        Family::Reduce => reduce_writes(productive, run, store, source)?,
        Family::Collect => collect_writes(productive, run, store, source, collect)?,
        Family::Repeat => repeat_writes(productive, run)?,
    };
    let output = match family {
        Family::ForEach => Some(SlotIdx::new(1)),
        Family::Reduce => Some(SlotIdx::new(0)),
        Family::Collect | Family::Repeat => None,
    };
    let mut expected = vec![EvidenceEvent::StepStarted {
        step: StepIdx::new(3),
    }];
    expected.extend(writes);
    expected.push(EvidenceEvent::StepSucceeded {
        step: StepIdx::new(3),
        output,
    });
    assert_eq!(
        events,
        expected.as_slice(),
        "{family:?}, productive={productive}"
    );
    Ok(())
}

fn assert_slot(run: &RunFrame, raw: u16, value: SlotValue, taint: Taint) -> Result<(), String> {
    let slot = SlotIdx::new(raw);
    assert_eq!(*checked(run.read_slot(slot))?, value, "value of slot {raw}");
    assert_eq!(checked(run.read_taint(slot))?, taint, "taint of slot {raw}");
    Ok(())
}

fn list_slot(run: &RunFrame, raw: u16) -> Result<ListId, String> {
    let slot = SlotIdx::new(raw);
    assert_eq!(checked(run.read_taint(slot))?, Taint::Secret);
    match checked(run.read_slot(slot))? {
        SlotValue::List(id) => Ok(*id),
        other => Err(format!("slot {raw} is not a list: {other:?}")),
    }
}

fn iterator(
    productive: bool,
    run: &RunFrame,
    store: &ValueStore,
    source: ListId,
    slot: u16,
) -> Result<ListId, String> {
    let id = list_slot(run, slot)?;
    let cursor = if productive { 2 } else { 1 };
    assert_eq!(
        checked(store.list(id))?,
        &[
            SlotValue::I64(i64::from(source.get())),
            SlotValue::I64(cursor),
        ]
    );
    Ok(id)
}

fn write(raw: u16, value: SlotValue, taint: Taint) -> EvidenceEvent {
    EvidenceEvent::SlotWritten {
        slot: SlotIdx::new(raw),
        value,
        taint,
        extra: None,
    }
}

fn for_each_writes(
    productive: bool,
    run: &RunFrame,
    store: &ValueStore,
    source: ListId,
) -> Result<Vec<EvidenceEvent>, String> {
    assert_slot(run, 0, SlotValue::List(source), Taint::Secret)?;
    let item = SlotValue::I64(if productive { 9 } else { 7 });
    assert_slot(run, 1, item, Taint::Secret)?;
    let iterator = iterator(productive, run, store, source, 2)?;
    Ok(if productive {
        vec![
            write(1, item, Taint::Secret),
            write(2, SlotValue::List(iterator), Taint::Secret),
        ]
    } else {
        Vec::new()
    })
}

fn reduce_writes(
    productive: bool,
    run: &RunFrame,
    store: &ValueStore,
    source: ListId,
) -> Result<Vec<EvidenceEvent>, String> {
    let item = SlotValue::I64(if productive { 9 } else { 7 });
    assert_slot(run, 0, item, Taint::Secret)?;
    assert_slot(run, 1, SlotValue::I64(42), Taint::Clean)?;
    let iterator = iterator(productive, run, store, source, 3)?;
    Ok(if productive {
        vec![
            write(0, item, Taint::Secret),
            write(3, SlotValue::List(iterator), Taint::Secret),
        ]
    } else {
        Vec::new()
    })
}

fn collect_writes(
    productive: bool,
    run: &RunFrame,
    store: &ValueStore,
    source: ListId,
    collect: &CollectStates,
) -> Result<Vec<EvidenceEvent>, String> {
    assert_slot(run, 0, SlotValue::List(source), Taint::Secret)?;
    let page = list_slot(run, 1)?;
    let state = collect.capture_state(run.run_id(), SlotIdx::new(1));
    if productive {
        assert_eq!(checked(store.list(page))?, &[SlotValue::I64(9)]);
        assert_collect_state(
            state.ok_or("missing productive collect state")?,
            run,
            source,
            page,
        );
    } else {
        assert_eq!(page, source);
        assert_eq!(checked(store.list(page))?, &[SlotValue::I64(7)]);
        assert_eq!(state, None);
    }
    Ok(vec![EvidenceEvent::SlotWritten {
        slot: SlotIdx::new(1),
        value: SlotValue::List(page),
        taint: Taint::Secret,
        extra: state,
    }])
}

fn assert_collect_state(
    state: CollectPaginationState,
    run: &RunFrame,
    source: ListId,
    page: ListId,
) {
    assert_eq!(state.run_id, run.run_id());
    assert_eq!(state.collector_slot, SlotIdx::new(1));
    assert_eq!(state.source, source);
    assert_eq!(state.current_page, page);
    assert_eq!(state.cursor, 2);
    assert_eq!(state.page_size, 1);
    assert_eq!(state.item_count, 2);
    assert_eq!(state.limit, 2);
    assert_eq!(state.time_limit_ms, None);
}

fn repeat_writes(productive: bool, run: &RunFrame) -> Result<Vec<EvidenceEvent>, String> {
    let value = SlotValue::I64(if productive {
        8_589_934_593
    } else {
        4_294_967_297
    });
    assert_slot(run, 0, value, Taint::Clean)?;
    Ok(vec![write(0, value, Taint::Clean)])
}
