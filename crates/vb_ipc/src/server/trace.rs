//! Trace event helpers.

use vb_runtime::runtime::Runtime;
use vb_runtime::trace::TraceEvent;

use super::handlers::decode_payload;
use crate::{IpcTraceEvent, IpcTraceEventKind};
use crate::IpcPayload;
use crate::server::IpcResponse;

enum IpcResponseKind {
    Trace,
}

fn count_response(count: usize, kind: IpcResponseKind) -> IpcResponse {
    match u32::try_from(count) {
        Ok(value) => match kind {
            IpcResponseKind::Trace => IpcResponse::TraceCount { count: value },
        },
        Err(_) => IpcResponse::CountOutOfRange {
            actual: count,
            limit: u32::MAX,
        },
    }
}

pub fn typed_events_response(events: &[TraceEvent], from_sequence: u64) -> IpcResponse {
    let mut typed_events = Vec::with_capacity(events.len());
    let mut index = 0usize;
    while index < events.len() {
        let Ok(sequence) = u64::try_from(index) else {
            return IpcResponse::CountOutOfRange {
                actual: index,
                limit: u32::MAX,
            };
        };
        if sequence >= from_sequence {
            let Some(event) = events.get(index) else {
                return IpcResponse::CountOutOfRange {
                    actual: index,
                    limit: u32::MAX,
                };
            };
            typed_events.push(IpcTraceEvent {
                sequence,
                kind: trace_event_kind(event),
            });
        }
        index = match index.checked_add(1) {
            Some(next) => next,
            None => {
                return IpcResponse::CountOutOfRange {
                    actual: index,
                    limit: u32::MAX,
                };
            }
        };
    }
    IpcResponse::Events {
        events: typed_events,
    }
}

pub fn trace_event_kind(event: &TraceEvent) -> IpcTraceEventKind {
    match event {
        TraceEvent::StepStarted { run, step } => IpcTraceEventKind::StepStarted {
            run: *run,
            step: *step,
        },
        TraceEvent::StepEnded { run, step } => IpcTraceEventKind::StepEnded {
            run: *run,
            step: *step,
        },
        TraceEvent::SlotWritten { run, slot } => IpcTraceEventKind::SlotWritten {
            run: *run,
            slot: *slot,
        },
        TraceEvent::ActionScheduled { run, step } => IpcTraceEventKind::ActionScheduled {
            run: *run,
            step: *step,
        },
        TraceEvent::ActionCompleted { run, step } => IpcTraceEventKind::ActionCompleted {
            run: *run,
            step: *step,
        },
        TraceEvent::ActionFailed { run, step, code } => IpcTraceEventKind::ActionFailed {
            run: *run,
            step: *step,
            code: *code,
        },
        TraceEvent::AskAnswered { run, step, slot } => IpcTraceEventKind::AskAnswered {
            run: *run,
            step: *step,
            slot: *slot,
        },
        TraceEvent::RunSubmitted { run } => IpcTraceEventKind::RunSubmitted { run: *run },
        TraceEvent::RunFinished { run } => IpcTraceEventKind::RunFinished { run: *run },
        TraceEvent::RunFailed { run } => IpcTraceEventKind::RunFailed { run: *run },
        TraceEvent::RunCancelled { run } => IpcTraceEventKind::RunCancelled { run: *run },
    }
}

pub fn count_response_trace(count: usize) -> IpcResponse {
    count_response(count, IpcResponseKind::Trace)
}

/// Handles drain-trace.
pub fn handle_drain_trace(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::DrainTrace {
        run_id,
        max_records,
    }) = decode_payload::<IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    let all_events = runtime.drain_trace();
    let max = match usize::try_from(max_records) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    };
    let filtered: Vec<TraceEvent> = all_events
        .into_iter()
        .filter(|event| event.run_id() == run_id)
        .take(max)
        .collect();
    count_response_trace(filtered.len())
}
