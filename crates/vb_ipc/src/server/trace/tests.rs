#[cfg(test)]
mod tests {
    use crate::IpcPayload;
    use crate::IpcTraceEventKind;
    use crate::server::IpcResponse;
    // Items declared pub(crate) in the trace module submodules
    use crate::server::trace::event::trace_event_kind;
    use crate::server::trace::handler::handle_drain_trace;
    use crate::server::trace::response::{count_response_trace, typed_events_response};
    use std::num::NonZeroUsize;
    use vb_core::RunId;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;
    use vb_runtime::trace::TraceEvent;

    fn run_id(val: u64) -> RunId {
        RunId::new(val)
    }

    fn make_runtime() -> Runtime {
        let mut config = ShardConfig {
            step_budget_per_tick: 4,
            ..ShardConfig::default()
        };
        config.policy = vb_core::policy::RuntimePolicy::Relaxed;
        Runtime::new(NonZeroUsize::MIN, config).expect("relaxed runtime config is valid")
    }

    fn chain_workflow() -> vb_core::workflow::CompiledWorkflow {
        use vb_core::WorkflowDigest;
        use vb_core::ids::{SlotIdx, StepIdx};
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        let mut nodes = Vec::new();
        for i in 0..10 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: Some(StepIdx::new(i + 1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(10),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        });
        let parts = WorkflowParts {
            name: Box::from("chain"),
            digest: WorkflowDigest::from_bytes([4; 32]),
            nodes: Box::from(nodes),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("valid workflow")
    }

    fn submit_and_tick(runtime: &mut Runtime, run_id: RunId) {
        runtime
            .submit_direct(run_id, chain_workflow())
            .expect("submit");
        runtime.tick_all().expect("tick");
    }

    fn encode_drain_trace(run_id: RunId, max_records: u32) -> Vec<u8> {
        let payload = IpcPayload::DrainTrace {
            run_id,
            max_records,
        };
        postcard::to_allocvec(&payload).expect("encode")
    }

    fn roundtrip_ipc_trace_event_kind(kind: IpcTraceEventKind) {
        let encoded = postcard::to_allocvec(&kind).expect("encode");
        let decoded: IpcTraceEventKind = postcard::from_bytes(&encoded).expect("decode");
        assert_eq!(decoded, kind);
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

    #[test]
    fn trace_event_kind_maps_run_killed() {
        let event = TraceEvent::RunKilled { run: run_id(15) };
        let kind = trace_event_kind(&event);
        assert_eq!(kind, IpcTraceEventKind::RunKilled { run: run_id(15) });
    }

    #[test]
    fn run_killed_roundtrip_via_postcard() {
        let original = IpcTraceEventKind::RunKilled { run: run_id(42) };
        let encoded = postcard::to_allocvec(&original).expect("encode");
        let decoded: IpcTraceEventKind = postcard::from_bytes(&encoded).expect("decode");
        assert_eq!(decoded, original);
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
                panic!("expected Events, got {other:?}");
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
                panic!("expected Events, got {other:?}");
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
                panic!("expected Events, got {other:?}");
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
                panic!("expected Events, got {other:?}");
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
                panic!("expected Events, got {other:?}");
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
                panic!("expected CountOutOfRange, got {other:?}");
            }
        }
    }

    // ── handle_drain_trace tests ──

    #[test]
    fn handle_drain_trace_empty_trace_returns_count_zero() {
        let mut runtime = make_runtime();
        let run_id = RunId::new(1);
        submit_and_tick(&mut runtime, run_id);
        runtime.drain_trace();
        let payload = encode_drain_trace(run_id, 100);
        let response = handle_drain_trace(&payload, &mut runtime);
        assert_eq!(response, IpcResponse::TraceCount { count: 0 });
    }

    #[test]
    fn handle_drain_trace_max_records_zero_returns_count_zero() {
        let mut runtime = make_runtime();
        let run_id = RunId::new(1);
        submit_and_tick(&mut runtime, run_id);
        let payload = encode_drain_trace(run_id, 0);
        let response = handle_drain_trace(&payload, &mut runtime);
        assert_eq!(response, IpcResponse::TraceCount { count: 0 });
    }

    #[test]
    fn handle_drain_trace_max_records_greater_than_trace_length() {
        let mut runtime = make_runtime();
        let run_id = RunId::new(1);
        submit_and_tick(&mut runtime, run_id);
        let all_events = runtime.drain_trace();
        let count = all_events.iter().filter(|e| e.run_id() == run_id).count();

        let run_id2 = RunId::new(2);
        submit_and_tick(&mut runtime, run_id2);
        let payload = encode_drain_trace(run_id2, 1000);
        let response = handle_drain_trace(&payload, &mut runtime);
        assert_eq!(
            response,
            IpcResponse::TraceCount {
                count: u32::try_from(count).expect("count fits u32"),
            }
        );
    }

    #[test]
    fn handle_drain_trace_max_records_less_than_trace_length() {
        let mut runtime = make_runtime();
        let run_id = RunId::new(1);
        submit_and_tick(&mut runtime, run_id);
        let all_events = runtime.drain_trace();
        let count = all_events.iter().filter(|e| e.run_id() == run_id).count();
        assert!(count > 2, "need more than 2 events for this test");

        let run_id2 = RunId::new(2);
        submit_and_tick(&mut runtime, run_id2);
        let payload = encode_drain_trace(run_id2, 2);
        let response = handle_drain_trace(&payload, &mut runtime);
        assert_eq!(response, IpcResponse::TraceCount { count: 2 });
    }

    #[test]
    fn handle_drain_trace_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let response = handle_drain_trace(b"not-a-valid-payload", &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_drain_trace_nonexistent_run_returns_runtime_error() {
        let mut runtime = make_runtime();
        let run_id = RunId::new(999);
        let payload = encode_drain_trace(run_id, 100);
        let response = handle_drain_trace(&payload, &mut runtime);
        match response {
            IpcResponse::RuntimeError { message } => {
                assert!(
                    message.contains("not found"),
                    "expected 'not found' in '{message}'"
                );
            }
            other => panic!("expected RuntimeError, got {other:?}"),
        }
    }

    // ── IpcTraceEventKind serialization roundtrip tests ──

    #[test]
    fn ipc_trace_event_kind_roundtrip_step_started() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::StepStarted {
            run: run_id(1),
            step: StepIdx::new(5),
        });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_step_ended() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::StepEnded {
            run: run_id(2),
            step: StepIdx::new(3),
        });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_slot_written() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::SlotWritten {
            run: run_id(4),
            slot: SlotIdx::ZERO,
            value: vec![1, 2, 3],
        });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_action_scheduled() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::ActionScheduled {
            run: run_id(7),
            step: StepIdx::new(1),
        });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_action_completed() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::ActionCompleted {
            run: run_id(8),
            step: StepIdx::new(2),
        });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_action_failed() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::ActionFailed {
            run: run_id(9),
            step: StepIdx::new(3),
            code: vb_core::action::ActionFailureCode::Unknown,
        });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_ask_answered() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::AskAnswered {
            run: run_id(10),
            step: StepIdx::new(4),
            slot: SlotIdx::ZERO,
        });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_run_submitted() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::RunSubmitted { run: run_id(11) });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_run_finished() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::RunFinished { run: run_id(12) });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_run_failed() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::RunFailed { run: run_id(13) });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_run_cancelled() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::RunCancelled { run: run_id(14) });
    }

    #[test]
    fn ipc_trace_event_kind_roundtrip_run_killed() {
        roundtrip_ipc_trace_event_kind(IpcTraceEventKind::RunKilled { run: run_id(15) });
    }
}
