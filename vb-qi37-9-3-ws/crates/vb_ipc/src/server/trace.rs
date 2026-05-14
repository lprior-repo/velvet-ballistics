#![forbid(unsafe_code)]
//! Trace event helpers.

use vb_runtime::runtime::Runtime;
use vb_runtime::trace::TraceEvent;

use super::handlers::decode_payload;
use crate::IpcPayload;
use crate::server::IpcResponse;
use crate::{IpcTraceEvent, IpcTraceEventKind};

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
        TraceEvent::SlotWritten { run, slot, value } => IpcTraceEventKind::SlotWritten {
            run: *run,
            slot: *slot,
            value: value.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::RunId;
    use vb_core::ids::{SlotIdx, StepIdx};

    fn run_id(val: u64) -> RunId {
        RunId::new(val)
    }

    // ── trace_event_kind mapping tests ──

    #[test]
    fn trace_event_kind_maps_step_started() {
        let event = TraceEvent::StepStarted {
            run: run_id(1),
            step: StepIdx::new(5),
        };
        let kind = trace_event_kind(&event);
        assert_eq!(
            kind,
            IpcTraceEventKind::StepStarted {
                run: run_id(1),
                step: StepIdx::new(5),
            }
        );
    }

    #[test]
    fn trace_event_kind_maps_step_ended() {
        let event = TraceEvent::StepEnded {
            run: run_id(2),
            step: StepIdx::new(3),
        };
        let kind = trace_event_kind(&event);
        assert_eq!(
            kind,
            IpcTraceEventKind::StepEnded {
                run: run_id(2),
                step: StepIdx::new(3),
            }
        );
    }

    #[test]
    fn trace_event_kind_maps_slot_written() {
        let event = TraceEvent::SlotWritten {
            run: run_id(4),
            slot: SlotIdx::ZERO,
            value: Vec::new(),
        };
        let kind = trace_event_kind(&event);
        assert_eq!(
            kind,
            IpcTraceEventKind::SlotWritten {
                run: run_id(4),
                slot: SlotIdx::ZERO,
                value: Vec::new(),
            }
        );
    }

    #[test]
    fn trace_event_kind_maps_action_scheduled() {
        let event = TraceEvent::ActionScheduled {
            run: run_id(7),
            step: StepIdx::new(1),
        };
        let kind = trace_event_kind(&event);
        assert_eq!(
            kind,
            IpcTraceEventKind::ActionScheduled {
                run: run_id(7),
                step: StepIdx::new(1),
            }
        );
    }

    #[test]
    fn trace_event_kind_maps_action_completed() {
        let event = TraceEvent::ActionCompleted {
            run: run_id(8),
            step: StepIdx::new(2),
        };
        let kind = trace_event_kind(&event);
        assert_eq!(
            kind,
            IpcTraceEventKind::ActionCompleted {
                run: run_id(8),
                step: StepIdx::new(2),
            }
        );
    }

    #[test]
    fn trace_event_kind_maps_action_failed() {
        let event = TraceEvent::ActionFailed {
            run: run_id(9),
            step: StepIdx::new(3),
            code: vb_core::action::ActionFailureCode::Unknown,
        };
        let kind = trace_event_kind(&event);
        assert_eq!(
            kind,
            IpcTraceEventKind::ActionFailed {
                run: run_id(9),
                step: StepIdx::new(3),
                code: vb_core::action::ActionFailureCode::Unknown,
            }
        );
    }

    #[test]
    fn trace_event_kind_maps_ask_answered() {
        let event = TraceEvent::AskAnswered {
            run: run_id(10),
            step: StepIdx::new(4),
            slot: SlotIdx::ZERO,
        };
        let kind = trace_event_kind(&event);
        assert_eq!(
            kind,
            IpcTraceEventKind::AskAnswered {
                run: run_id(10),
                step: StepIdx::new(4),
                slot: SlotIdx::ZERO,
            }
        );
    }

    #[test]
    fn trace_event_kind_maps_run_submitted() {
        let event = TraceEvent::RunSubmitted { run: run_id(11) };
        let kind = trace_event_kind(&event);
        assert_eq!(kind, IpcTraceEventKind::RunSubmitted { run: run_id(11) });
    }

    #[test]
    fn trace_event_kind_maps_run_finished() {
        let event = TraceEvent::RunFinished { run: run_id(12) };
        let kind = trace_event_kind(&event);
        assert_eq!(kind, IpcTraceEventKind::RunFinished { run: run_id(12) });
    }

    #[test]
    fn trace_event_kind_maps_run_failed() {
        let event = TraceEvent::RunFailed { run: run_id(13) };
        let kind = trace_event_kind(&event);
        assert_eq!(kind, IpcTraceEventKind::RunFailed { run: run_id(13) });
    }

    #[test]
    fn trace_event_kind_maps_run_cancelled() {
        let event = TraceEvent::RunCancelled { run: run_id(14) };
        let kind = trace_event_kind(&event);
        assert_eq!(kind, IpcTraceEventKind::RunCancelled { run: run_id(14) });
    }

    // ── typed_events_response tests ──

    #[test]
    fn typed_events_response_returns_empty_for_no_events() {
        let events: Vec<TraceEvent> = Vec::new();
        let response = typed_events_response(&events, 0);
        match response {
            IpcResponse::Events { events: evts } => {
                assert!(evts.is_empty(), "no events should produce empty response");
            }
            other => {
                assert!(false, "expected Events, got {other:?}");
            }
        }
    }

    #[test]
    fn typed_events_response_returns_all_events_from_sequence_zero() {
        let events = vec![
            TraceEvent::RunSubmitted { run: run_id(1) },
            TraceEvent::RunFinished { run: run_id(1) },
        ];
        let response = typed_events_response(&events, 0);
        match response {
            IpcResponse::Events { events: evts } => {
                assert_eq!(evts.len(), 2);
                assert_eq!(evts[0].sequence, 0);
                assert_eq!(evts[1].sequence, 1);
            }
            other => {
                assert!(false, "expected Events, got {other:?}");
            }
        }
    }

    #[test]
    fn typed_events_response_filters_by_from_sequence() {
        let events = vec![
            TraceEvent::RunSubmitted { run: run_id(1) },
            TraceEvent::RunFinished { run: run_id(1) },
            TraceEvent::RunFailed { run: run_id(2) },
        ];
        let response = typed_events_response(&events, 1);
        match response {
            IpcResponse::Events { events: evts } => {
                assert_eq!(evts.len(), 2);
                assert_eq!(evts[0].sequence, 1);
                assert_eq!(evts[1].sequence, 2);
            }
            other => {
                assert!(false, "expected Events, got {other:?}");
            }
        }
    }

    #[test]
    fn typed_events_response_filters_all_when_from_sequence_exceeds_count() {
        let events = vec![TraceEvent::RunSubmitted { run: run_id(1) }];
        let response = typed_events_response(&events, 10);
        match response {
            IpcResponse::Events { events: evts } => {
                assert!(
                    evts.is_empty(),
                    "from_sequence beyond count should yield empty"
                );
            }
            other => {
                assert!(false, "expected Events, got {other:?}");
            }
        }
    }

    #[test]
    fn typed_events_response_preserves_event_kind_mapping() {
        let events = vec![TraceEvent::StepStarted {
            run: run_id(1),
            step: StepIdx::new(0),
        }];
        let response = typed_events_response(&events, 0);
        match response {
            IpcResponse::Events { events: evts } => {
                assert_eq!(evts.len(), 1);
                assert_eq!(
                    evts[0].kind,
                    IpcTraceEventKind::StepStarted {
                        run: run_id(1),
                        step: StepIdx::new(0),
                    }
                );
            }
            other => {
                assert!(false, "expected Events, got {other:?}");
            }
        }
    }

    // ── count_response_trace tests ──

    #[test]
    fn count_response_trace_returns_trace_count_for_small_count() {
        let response = count_response_trace(42);
        assert_eq!(response, IpcResponse::TraceCount { count: 42 });
    }

    #[test]
    fn count_response_trace_returns_trace_count_for_zero() {
        let response = count_response_trace(0);
        assert_eq!(response, IpcResponse::TraceCount { count: 0 });
    }

    #[test]
    fn count_response_trace_returns_count_out_of_range_for_exceeding_u32() {
        let response = count_response_trace(u32::MAX as usize + 1);
        match response {
            IpcResponse::CountOutOfRange { actual, limit } => {
                assert_eq!(actual, u32::MAX as usize + 1);
                assert_eq!(limit, u32::MAX);
            }
            other => {
                assert!(false, "expected CountOutOfRange, got {other:?}");
            }
        }
    }
}
