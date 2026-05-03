//! IPC command handlers dispatched by the server.

#![allow(unused_imports)]

use vb_core::action::{ActionFailure, ActionFailureCode};
use vb_core::ids::SlotIdx;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket};
use vb_runtime::trace::TraceEvent;

use super::trace::typed_events_response;
use crate::server::ticket::{action_ticket_from_wire, payload_len, step_from_ticket};
use crate::server::{IpcResponse, WorkflowResolutionError, WorkflowResolver};
use crate::{IpcActionOutputPayload, IpcCommand, IpcPayload, RunListState, RunSummary, RuntimeMetrics, SubmitRunPayload};

/// Decodes a postcard-encoded payload and preserves the typed IPC decode error.
pub fn decode_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, IpcResponse> {
    postcard::from_bytes(payload)
        .map_err(|_| ipc_error_response(crate::IpcError::PayloadDecodeFailed))
}

fn ipc_error_response(error: crate::IpcError) -> IpcResponse {
    IpcResponse::PayloadError {
        diagnostic: error.diagnostic_code().code(),
        message: error.to_string(),
    }
}

/// Handles a ping/health request.
pub fn handle_ping() -> IpcResponse {
    IpcResponse::Healthy
}

/// Handles a health request.
pub fn handle_health() -> IpcResponse {
    handle_ping()
}

/// Handles shutdown.
pub fn handle_shutdown(runtime: &mut Runtime) -> IpcResponse {
    match runtime.shutdown_graceful() {
        Ok(()) => IpcResponse::ShuttingDown,
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles submit-run commands after resolving the compiled workflow explicitly.
pub fn handle_submit_run(
    header: &crate::IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let decoded = match decode_payload::<crate::IpcPayload>(payload) {
        Ok(d) => d,
        Err(response) => return response,
    };

    match (header.command, decoded) {
        (IpcCommand::SubmitRun, crate::IpcPayload::SubmitRun(submit))
        | (IpcCommand::SubmitRunInline, crate::IpcPayload::SubmitRunInline(submit)) => {
            submit_resolved_workflow(header.command, submit, runtime, resolver)
        }
        _ => IpcResponse::CommandPayloadMismatch,
    }
}

/// Handles inline submit-run commands.
pub fn handle_submit_run_inline(
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 0, 0);
    handle_submit_run(&header, payload, runtime, resolver)
}

/// Handles cancel-run.
pub fn handle_cancel_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::CancelRun { run_id }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.snapshot_run(run_id, 0) {
        Ok(vb_runtime::shard::InspectResponse::Found(_)) => {}
        Ok(vb_runtime::shard::InspectResponse::NotFound { .. }) => {
            return IpcResponse::RuntimeError {
                message: String::from("run not found"),
            };
        }
        Err(e) => {
            return IpcResponse::RuntimeError {
                message: e.to_string(),
            };
        }
    }

    match runtime.cancel_run(run_id) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles inspect-run.
pub fn handle_inspect_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::InspectRun { run_id }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.snapshot_run(run_id, 0) {
        Ok(vb_runtime::shard::InspectResponse::Found(_snapshot)) => IpcResponse::Inspected {
            run_id: run_id.as_u64(),
        },
        Ok(vb_runtime::shard::InspectResponse::NotFound { .. }) => IpcResponse::RuntimeError {
            message: String::from("run not found"),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles list-events.
pub fn handle_list_events(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::ListEvents {
        run_id,
        from_sequence,
    }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.list_events(run_id) {
        Ok(events) => typed_events_response(&events, from_sequence),
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles answer-ask.
pub fn handle_answer_ask(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::AnswerAsk { run_id, ticket, .. }) =
        decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    let Some(ask_step) = step_from_ticket(ticket) else {
        return IpcResponse::BadRequest;
    };
    let answer = AskAnswer {
        ticket: AskTicket {
            run: run_id,
            ask_step,
            resume_step: ask_step,
        },
        answer_slot: SlotIdx::ZERO,
        value: SlotValue::Null,
        taint: Taint::Clean,
    };

    match runtime.answer_ask(answer) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles complete-action.
pub fn handle_complete_action(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::CompleteAction {
        run_id,
        ticket,
        output,
    }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    let Some(action_ticket) = action_ticket_from_wire(run_id, ticket) else {
        return IpcResponse::BadRequest;
    };
    let output_len = payload_len(output.len());
    let decoded_output = match decode_payload::<crate::IpcActionOutputPayload>(&output) {
        Ok(d) => d,
        Err(response) => return response,
    };
    match runtime
        .complete_action_with_output(action_ticket, decoded_output.into_action_output(output_len))
    {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles fail-action.
pub fn handle_fail_action(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::FailAction {
        run_id,
        ticket,
        error,
    }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    let Some(action_ticket) = action_ticket_from_wire(run_id, ticket) else {
        return IpcResponse::BadRequest;
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retryable: false,
        taint: Taint::Clean,
        detail: None,
        encoded_len: payload_len(error.len()),
    };

    match runtime.fail_action(action_ticket, failure) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

pub fn submit_resolved_workflow(
    command: IpcCommand,
    submit: SubmitRunPayload,
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };
    let workflow = match resolver.resolve_workflow(submit.workflow) {
        Ok(workflow) => workflow,
        Err(WorkflowResolutionError::Required) => return IpcResponse::WorkflowResolutionRequired,
        Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
            return IpcResponse::WorkflowResolutionUnsupported;
        }
    };
    if workflow.digest() != submit.workflow {
        return IpcResponse::WorkflowDigestMismatch;
    }
    let result = match command {
        IpcCommand::SubmitRun => runtime.submit_compiled(submit.run_id, workflow),
        IpcCommand::SubmitRunInline => runtime.submit_direct(submit.run_id, workflow),
        _ => return IpcResponse::CommandPayloadMismatch,
    };
    match result {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: submit.run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles list-runs.
pub fn handle_list_runs(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::ListRuns { limit, workflow }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };

    let active_summaries = runtime.list_active_runs(limit, workflow);
    let runs: Vec<RunSummary> = active_summaries
        .into_iter()
        .map(|summary| RunSummary {
            run_id: summary.run_id,
            workflow: summary.workflow,
            state: RunListState::Active,
            submitted_seq: 0,
            finished_seq: None,
            step_count: summary.step_count,
            steps_completed: summary.steps_completed,
        })
        .collect();

    IpcResponse::RunList { runs }
}

/// Handles get-metrics.
pub fn handle_get_metrics(runtime: &Runtime) -> IpcResponse {
    let snapshot = runtime.collect_metrics();
    let shards: Vec<crate::ShardMetrics> = snapshot
        .shards
        .into_iter()
        .map(|s| crate::ShardMetrics {
            shard_id: s.shard_id,
            active_runs: s.active_runs,
            ready_queue_depth: s.command_queue_depth,
            action_queue_depth: s.command_queue_remaining,
            timer_count: s.pending_timers,
            frame_pool_free: s.frame_pool_free,
            frame_pool_total: s.frame_pool_total,
            trace_ring_fill_pct: s.trace_ring_fill_pct,
            steps_total: s.counters.steps_executed,
            actions_total: s.counters.runs_completed.saturating_add(s.counters.runs_failed),
        })
        .collect();

    let totals = crate::AggregateMetrics {
        runs_active: snapshot.runs_active,
        runs_waiting: snapshot.runs_waiting,
        runs_failed_total: snapshot.runs_failed_total,
        runs_finished_total: snapshot.runs_finished_total,
    };

    IpcResponse::Metrics(RuntimeMetrics {
        journal: crate::JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: snapshot.runs_finished_total.saturating_add(snapshot.runs_failed_total),
        },
        ipc: crate::IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        shards,
        totals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── decode_payload tests ──

    #[test]
    fn decode_payload_succeeds_for_valid_postcard_bytes() {
        let payload = crate::IpcPayload::Health;
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "postcard encoding should succeed");
        let Ok(encoded) = encoded else { return };

        let result = decode_payload::<crate::IpcPayload>(&encoded);
        match result {
            Ok(decoded) => assert_eq!(decoded, crate::IpcPayload::Health),
            Err(_) => {
                assert!(false, "decode_payload should succeed for valid Health payload");
            }
        }
    }

    #[test]
    fn decode_payload_returns_error_for_garbage_bytes() {
        let garbage: &[u8] = &[0xFF, 0xFE, 0xFD, 0xFC];
        let result = decode_payload::<crate::IpcPayload>(garbage);
        match result {
            Err(IpcResponse::PayloadError { diagnostic, message }) => {
                assert!(!message.is_empty(), "error message should not be empty");
                assert_eq!(diagnostic, 0x300D);
            }
            other => {
                assert!(false, "expected PayloadError for garbage, got {other:?}");
            }
        }
    }

    #[test]
    fn decode_payload_returns_error_for_empty_bytes() {
        let result = decode_payload::<crate::IpcPayload>(&[]);
        match result {
            Err(IpcResponse::PayloadError { .. }) => {}
            other => {
                assert!(false, "expected PayloadError for empty bytes, got {other:?}");
            }
        }
    }

    #[test]
    fn decode_payload_roundtrips_cancel_run() {
        let payload = crate::IpcPayload::CancelRun {
            run_id: vb_core::RunId::new(42),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode CancelRun");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_drain_trace() {
        let payload = crate::IpcPayload::DrainTrace {
            run_id: vb_core::RunId::new(7),
            max_records: 500,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode DrainTrace");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_shutdown() {
        let payload = crate::IpcPayload::Shutdown;
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode Shutdown");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_list_events() {
        let payload = crate::IpcPayload::ListEvents {
            run_id: vb_core::RunId::new(33),
            from_sequence: 100,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode ListEvents");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_inspect_run() {
        let payload = crate::IpcPayload::InspectRun {
            run_id: vb_core::RunId::new(55),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode InspectRun");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_answer_ask() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(3),
            ticket: 42,
            answer: Vec::from(&b"yes"[..]),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode AnswerAsk");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_complete_action() {
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(10),
            ticket: 7,
            output: Vec::from(&b"result"[..]),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode CompleteAction");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_fail_action() {
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(11),
            ticket: 3,
            error: Vec::from(&b"failure"[..]),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode FailAction");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_get_metrics() {
        let payload = crate::IpcPayload::GetMetrics;
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode GetMetrics");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_list_runs() {
        let payload = crate::IpcPayload::ListRuns {
            limit: 50,
            workflow: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode ListRuns");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_submit_run() {
        let payload = crate::IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: vb_core::RunId::new(99),
            workflow: vb_core::WorkflowDigest::from_bytes([0xAA; 32]),
            input: Vec::from(&b"input"[..]),
        });
        let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode SubmitRun");
            return;
        };
        assert_eq!(decoded, payload);
    }

    // ── handle_ping / handle_health tests ──

    #[test]
    fn handle_ping_returns_healthy() {
        assert_eq!(handle_ping(), IpcResponse::Healthy);
    }

    #[test]
    fn handle_health_returns_healthy() {
        assert_eq!(handle_health(), IpcResponse::Healthy);
    }

    // ── ipc_error_response tests ──

    #[test]
    fn ipc_error_response_maps_full_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::Full);
        match response {
            IpcResponse::PayloadError { diagnostic, message } => {
                assert_eq!(diagnostic, 0x3001);
                assert!(message.contains("full"), "expected 'full' in '{message}'");
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }

    #[test]
    fn ipc_error_response_maps_decode_failed_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::PayloadDecodeFailed);
        match response {
            IpcResponse::PayloadError { diagnostic, message } => {
                assert_eq!(diagnostic, 0x300D);
                assert!(message.contains("decode"), "expected 'decode' in '{message}'");
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }

    #[test]
    fn ipc_error_response_maps_invalid_magic_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::InvalidMagic { actual: 0xBAD });
        match response {
            IpcResponse::PayloadError { diagnostic, message } => {
                assert_eq!(diagnostic, 0x3004);
                assert!(message.contains("magic"), "expected 'magic' in '{message}'");
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }

    #[test]
    fn ipc_error_response_maps_unknown_command_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::UnknownCommand(200));
        match response {
            IpcResponse::PayloadError { diagnostic, message } => {
                assert_eq!(diagnostic, 0x3006);
                assert!(message.contains("200"), "expected '200' in '{message}'");
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }
}
