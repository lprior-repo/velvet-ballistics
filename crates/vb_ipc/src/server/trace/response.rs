//! Response shape helpers and typed-event construction.
//!
//! - `count_response` / `count_response_trace`: produce count-based IPC responses.
//! - `typed_events_response`: converts raw runtime events into a typed event list.

use vb_runtime::trace::TraceEvent;

use super::super::IpcResponse;
use super::event::trace_event_kind;
use crate::IpcTraceEvent;

/// Internal discriminator for response-kind dispatch.
enum IpcResponseKind {
    Trace,
}

/// Build a count-based IPC response for the given kind.
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

/// Convenience wrapper: count response specialised for trace domains.
pub(crate) fn count_response_trace(count: usize) -> IpcResponse {
    count_response(count, IpcResponseKind::Trace)
}

/// Convert raw trace events into a typed, sequence-indexed IPC response.
///
/// Events before `from_sequence` are skipped; overflow at any index produces
/// a `CountOutOfRange` response.
pub(crate) fn typed_events_response(events: &[TraceEvent], from_sequence: u64) -> IpcResponse {
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
