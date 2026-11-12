#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx, SlotValue, StepIdx};
use vb_storage::recovery::RecoveryError;
use vb_storage::recovery::hydrate::{hydrate_events_preconditions, hydrate_run_frame_from_events};
use vb_storage::{EventSeq, JournalEvent};

const EVENT_CHUNK: usize = 4;
const MAX_EVENTS: usize = 16;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let Some(seed) = data.first().copied() else {
        return;
    };
    let Some(count) = bounded_event_count(seed) else {
        return;
    };
    let run = RunId::new(u64::from(seed).saturating_add(1));
    let mut events: Vec<JournalEvent> = Vec::with_capacity(count);
    for event_index in 0..count {
        let chunk = chunk_for_event(data, event_index).unwrap_or(&[]);
        let Some(seq) = event_seq_from_index(event_index) else {
            return;
        };
        if let Some(event) = event_from_bytes(chunk, run, seq) {
            events.push(event);
        }
    }

    assert_eq!(hydrate_events_preconditions(&events), !events.is_empty());
    observe_hydration_result(hydrate_run_frame_from_events(&events, run), run);
});

fn bounded_event_count(seed: u8) -> Option<usize> {
    usize::from(seed).checked_rem(MAX_EVENTS)?.checked_add(1)
}

fn chunk_for_event(data: &[u8], event_index: usize) -> Option<&[u8]> {
    let offset = event_index.checked_mul(EVENT_CHUNK)?.checked_add(1)?;
    let end = offset.checked_add(EVENT_CHUNK)?;
    data.get(offset..end)
}

fn event_seq_from_index(event_index: usize) -> Option<EventSeq> {
    u64::try_from(event_index).ok().map(EventSeq::new)
}

fn event_from_bytes(bytes: &[u8], run: RunId, seq: EventSeq) -> Option<JournalEvent> {
    let mode = bytes
        .first()
        .copied()
        .and_then(|byte| byte.checked_rem(3))
        .unwrap_or(0);
    let step_byte = bytes.get(1).copied().unwrap_or(0);
    let slot_byte = bytes.get(2).copied().unwrap_or(0);
    let attempt_byte = bytes.get(3).copied().unwrap_or(1);
    let step = StepIdx::new(u16::from(step_byte));
    let slot = SlotIdx::new(u16::from(slot_byte));
    let attempt = u16::from(attempt_byte).max(1);
    match mode {
        0 => Some(JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt,
        }),
        1 => {
            let value = match postcard::to_allocvec(&SlotValue::I64(i64::from(slot_byte))) {
                Ok(encoded) => encoded,
                Err(_) => return None,
            };
            Some(JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot,
                value: Some(value),
                extra: None,
                attempt,
            })
        }
        2 => Some(JournalEvent::RunFinished {
            run,
            seq,
            result: slot,
            attempt,
        }),
        _ => None,
    }
}

fn observe_hydration_result(result: Result<vb_core::RunFrame, RecoveryError>, run: RunId) {
    match result {
        Ok(frame) => {
            assert_eq!(
                frame.run_id(),
                run,
                "hydrated frame must keep requested run id"
            );
        }
        Err(error) => assert_typed_recovery_error(error),
    }
}

fn assert_typed_recovery_error(error: RecoveryError) {
    assert!(
        matches!(
            error,
            RecoveryError::Journal(_)
                | RecoveryError::WorkflowSourceDigestMismatch { .. }
                | RecoveryError::CompiledIrDigestMismatch { .. }
                | RecoveryError::ActionAbiMismatch { .. }
                | RecoveryError::PolicyDigestMismatch { .. }
                | RecoveryError::NonIdempotentActionBlocked { .. }
                | RecoveryError::ReplayDivergence { .. }
                | RecoveryError::SlotTaintReadFailed { .. }
                | RecoveryError::CorruptSlotTaint { .. }
                | RecoveryError::NoRecoveryData { .. }
                | RecoveryError::CorruptSnapshot { .. }
                | RecoveryError::MissingSnapshot { .. }
                | RecoveryError::TerminalStateMismatch { .. }
                | RecoveryError::FrameDimensionOverflow { .. }
                | RecoveryError::UnsupportedFrameSeed { .. }
                | RecoveryError::ArtifactNotFound { .. }
                | RecoveryError::ArtifactDecodeFailed
        ),
        "unexpected non-exhaustive RecoveryError variant"
    );
}
