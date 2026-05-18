#![forbid(unsafe_code)]
//! Query handlers that read or submit run state.

use vb_core::ids::SlotIdx;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket};
use vb_runtime::trace::TraceEvent;

use super::trace::typed_events_response;
use crate::server::ticket::{action_ticket_from_wire, payload_len, step_from_ticket};
use crate::server::{IpcResponse, WorkflowResolutionError, WorkflowResolver};
use crate::{
    IpcActionOutputPayload, IpcCommand, IpcPayload, RunListState, RunSummary, RuntimeMetrics,
    SubmitRunPayload,
};

const MAX_SUBMIT_INPUT_LEN: usize = 65536;
const MAX_ACTION_OUTPUT_LEN: usize = 65536;
const MAX_ACTION_ERROR_LEN: usize = 65536;
const MAX_ANSWER_ASK_BYTES: usize = 65536;
const MAX_LIST_RUNS_LIMIT: u32 = 4096;

pub fn decode_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, IpcResponse> {
    postcard::from_bytes(payload).map_err(|_| ipc_error_response(crate::IpcError::PayloadDecodeFailed))
}

fn ipc_error_response(error: crate::IpcError) -> IpcResponse {
    IpcResponse::PayloadError {
        diagnostic: error.diagnostic_code().code(),
        message: error.to_string(),
    }
}

pub fn sanitize_runtime_error(e: &dyn std::fmt::Display) -> String {
    let full = e.to_string();
    if full.len() <= 256 {
        return full;
    }
    let mut truncated: String = full.chars().take(256).collect();
    truncated.push_str("...");
    truncated
}

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

pub fn handle_submit_run_inline(
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 0, 0);
    handle_submit_run(&header, payload, runtime, resolver)
}

pub fn handle_cancel_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::CancelRun { run_id }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.cancel_run(run_id) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.get(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
        },
    }
}

pub fn handle_inspect_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::InspectRun { run_id }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.snapshot_run(run_id, 0) {
        Ok(vb_runtime::shard::InspectResponse::Found(_snapshot)) => IpcResponse::Inspected {
            run_id: run_id.get(),
        },
        Ok(vb_runtime::shard::InspectResponse::NotFound { .. }) => IpcResponse::RuntimeError {
            message: String::from("run not found"),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
        },
    }
}

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
            message: sanitize_runtime_error(&e),
        },
    }
}

pub fn submit_resolved_workflow(
    command: IpcCommand,
    submit: SubmitRunPayload,
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    if submit.input.len() > MAX_SUBMIT_INPUT_LEN {
        return IpcResponse::PayloadError {
            diagnostic: crate::IpcError::PayloadDecodeFailed
                .diagnostic_code()
                .code(),
            message: String::from("submit input exceeds maximum allowed size"),
        };
    }
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
            run_id: submit.run_id.get(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
        },
    }
}

pub fn handle_list_runs(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::ListRuns { limit, workflow }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };

    let capped_limit = limit.min(MAX_LIST_RUNS_LIMIT);
    let active_summaries = runtime.list_active_runs(capped_limit, workflow);
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
            actions_total: s
                .counters
                .runs_completed
                .saturating_add(s.counters.runs_failed),
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
            total_runs: snapshot
                .runs_finished_total
                .saturating_add(snapshot.runs_failed_total),
        },
        ipc: crate::IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        shards,
        totals,
    })
}
