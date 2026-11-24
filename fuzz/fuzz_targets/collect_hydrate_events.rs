//! Fuzz target for collect hydration from recovered journal events.
//!
//! Tests `hydrate_collect_states_from_recovered_journal` with:
//! - Mix of collect and non-collect SlotWritten events
//! - Corrupted extra bytes
//! - Mismatched run/slot/seq identifiers
//! - Out-of-order event sequences
//! - Duplicate events
//! - Truncated payloads

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_core::errors::CollectExtraHydrationFailureKind;
use vb_core::{EngineError, ListId, RunId, SlotIdx, SlotValue, Taint};
use vb_runtime::primitives::collect::{
    CollectPaginationState, hydrate_collect_states_from_recovered_journal,
};
use vb_storage::{EventSeq, JournalEvent};

const MODE_COLLECT_ENVELOPE: u8 = 0;
const MODE_COLLECT_LEGACY: u8 = 1;
const MODE_COLLECT_CORRUPT: u8 = 2;
const MODE_NON_COLLECT: u8 = 3;
const MODE_RUN_MISMATCH: u8 = 4;
const MODE_SLOT_MISMATCH: u8 = 5;
const MODE_EMPTY_EXTRA: u8 = 6;

fuzz_target!(|data: &[u8]| {
    let events = build_event_sequence(data);
    if events.is_empty() {
        return;
    }

    observe_collect_hydration(
        hydrate_collect_states_from_recovered_journal(&events),
        &events,
    );
});

fn build_event_sequence(data: &[u8]) -> Vec<JournalEvent> {
    let count = usize::from(data.first().copied().map_or(1, |byte| byte) % 8).saturating_add(1);
    data.chunks(4)
        .take(count)
        .enumerate()
        .filter_map(|(i, record)| {
            let (&mode, fields) = record.split_first()?;
            let run = RunId::new(u64::from(byte_at(fields, 0)).saturating_add(1));
            let slot = SlotIdx::new(u16::from(byte_at(fields, 1)));
            let seq = EventSeq::new(u64::from(byte_at(fields, 2)));
            match mode % 8 {
                MODE_COLLECT_ENVELOPE => collect_envelope_event(run, slot, seq, data, i),
                MODE_COLLECT_LEGACY => collect_legacy_event(run, slot, seq),
                MODE_COLLECT_CORRUPT => collect_corrupt_event(run, slot, seq),
                MODE_NON_COLLECT => non_collect_event(run, slot, seq),
                MODE_RUN_MISMATCH => mismatched_run_event(run, slot, seq),
                MODE_SLOT_MISMATCH => mismatched_slot_event(run, slot, seq),
                MODE_EMPTY_EXTRA => empty_extra_event(run, slot, seq),
                _ => truncated_extra_event(run, slot, seq),
            }
        })
        .collect()
}

#[path = "collect_hydrate_events/event_builders.rs"]
mod event_builders;

use event_builders::*;

fn byte_at(data: &[u8], index: usize) -> u8 {
    data.get(index).map_or(0, |&byte| byte)
}

fn observe_collect_hydration(
    result: Result<vb_runtime::primitives::collect::CollectStates, EngineError>,
    events: &[JournalEvent],
) {
    match result {
        Ok(states) => events.iter().for_each(|event| {
            if let JournalEvent::SlotWrittenEvent {
                run, slot, extra, ..
            } = event
                && extra.is_some()
            {
                let _state = states.find(*run, *slot, ListId::new(1));
            }
        }),
        Err(err) => assert!(
            matches!(
                err,
                EngineError::CollectExtraHydrationFailed {
                    kind: CollectExtraHydrationFailureKind::EmptyExtra
                        | CollectExtraHydrationFailureKind::DecodeFailed
                        | CollectExtraHydrationFailureKind::RunMismatch { .. }
                        | CollectExtraHydrationFailureKind::SlotMismatch { .. }
                        | CollectExtraHydrationFailureKind::CurrentPageMismatch { .. },
                    ..
                } | EngineError::InvalidCompiledWorkflow { .. }
                    | EngineError::CollectPageOrderViolation { .. }
            ),
            "Unexpected error from collect hydration: {err:?}"
        ),
    }
}
