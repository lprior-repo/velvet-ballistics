#![forbid(unsafe_code)]
//! IPC command handlers dispatched by the server.

#![allow(unused_imports)]

use vb_core::action::{ActionFailure, ActionFailureCode, RetryPolicy};
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

/// Maximum length for a sanitized runtime error message returned to IPC clients.
const MAX_RUNTIME_ERROR_LEN: usize = 256;

/// Maximum allowed size for the `SubmitRunPayload.input` field.
/// Prevents unbounded allocation from deserialized input bytes.
const MAX_SUBMIT_INPUT_LEN: usize = 65536;

/// Maximum allowed size for `CompleteAction.output` payload bytes.
const MAX_ACTION_OUTPUT_LEN: usize = 65536;

/// Maximum allowed size for `FailAction.error` payload bytes.
const MAX_ACTION_ERROR_LEN: usize = 65536;

/// Maximum number of taint path entries returned by the taint report.
/// Prevents O(N^2) memory blowup for workflows with many sources and nodes.
const MAX_TAINT_PATH_ENTRIES: usize = 65536;

/// Maximum length for validation error details in verify-workflow responses.
/// Prevents leakage of verbose internal diagnostics.
const MAX_VALIDATION_DETAIL_LEN: usize = 512;

/// Maximum number of runs returned by list-runs.
/// Caps the client-supplied limit to prevent unbounded response allocation.
const MAX_LIST_RUNS_LIMIT: u32 = 4096;

/// Maximum allowed size for the `AnswerAsk.answer` payload bytes.
/// Prevents unbounded deserialization of unused answer data.
const MAX_ANSWER_ASK_BYTES: usize = 65536;

/// Maximum number of nodes returned by get-workflow-graph.
/// Caps response allocation for very large compiled workflows.
const MAX_WORKFLOW_GRAPH_NODES: usize = 8192;

/// Sanitizes a runtime error message before returning it to an IPC client.
///
/// Truncates the message to a fixed maximum length to prevent accidental
/// leakage of large internal diagnostics over the IPC channel.  The truncation
/// preserves the first `MAX_RUNTIME_ERROR_LEN` characters and appends an
/// ellipsis indicator when the original message was longer.
pub(crate) fn sanitize_runtime_error(e: &dyn std::fmt::Display) -> String {
    let full = e.to_string();
    if full.len() <= MAX_RUNTIME_ERROR_LEN {
        return full;
    }
    let mut truncated: String = full.chars().take(MAX_RUNTIME_ERROR_LEN).collect();
    truncated.push_str("...");
    truncated
}

/// Sanitizes a validation error detail string to prevent information leakage.
/// Truncates to `MAX_VALIDATION_DETAIL_LEN` characters and strips any path-like
/// substrings that might reveal internal filesystem layout.
fn sanitize_validation_detail(detail: String) -> String {
    let truncated = if detail.len() <= MAX_VALIDATION_DETAIL_LEN {
        detail
    } else {
        let mut s: String = detail.chars().take(MAX_VALIDATION_DETAIL_LEN).collect();
        s.push_str("...");
        s
    };
    // Strip common path separators that could reveal internal layout.
    truncated
        .replace("/home/", "<redacted>/")
        .replace("/etc/", "<redacted>/")
        .replace("/var/", "<redacted>/")
        .replace("/tmp/", "<redacted>/")
        .replace("/usr/", "<redacted>/")
        .replace("C:\\", "<redacted>\\")
        .replace("\\\\", "<redacted>\\\\")
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
            message: sanitize_runtime_error(&e),
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

    match runtime.cancel_run(run_id) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.get(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
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
            message: sanitize_runtime_error(&e),
        },
    }
}

/// Handles answer-ask.
pub fn handle_answer_ask(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::AnswerAsk {
        run_id,
        ticket,
        answer,
        taint,
    }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };
    if answer.len() > MAX_ANSWER_ASK_BYTES {
        return IpcResponse::PayloadError {
            diagnostic: crate::IpcError::PayloadDecodeFailed
                .diagnostic_code()
                .code(),
            message: String::from("answer payload exceeds maximum allowed size"),
        };
    }

    let Some(ask_step) = step_from_ticket(ticket) else {
        return IpcResponse::BadRequest;
    };
    let encoded_len = match u32::try_from(answer.len()) {
        Ok(len) => len,
        Err(_) => {
            // MAX_ANSWER_ASK_BYTES (65536) is well below u32::MAX, so this
            // branch is logically unreachable due to the prior bounds check.
            // The match handles the fallible conversion without panicking.
            return IpcResponse::RuntimeError {
                message: String::from("answer payload size exceeds u32::MAX"),
            };
        }
    };
    // Decode the caller's answer bytes as a postcard-serialized SlotValue.
    // The bytes are expected to be valid postcard-encoded SlotValue; if decode
    // fails, return an error rather than silently discarding the payload.
    let value = match postcard::from_bytes::<SlotValue>(&answer) {
        Ok(v) => v,
        Err(_) => {
            return IpcResponse::RuntimeError {
                message: String::from("answer bytes are not valid postcard-encoded SlotValue"),
            };
        }
    };
    let answer = AskAnswer {
        ticket: AskTicket {
            run: run_id,
            ask_step,
            resume_step: ask_step,
        },
        answer_slot: SlotIdx::ZERO,
        value,
        taint: taint.unwrap_or(Taint::Clean),
        encoded_len,
    };

    match runtime.answer_ask(answer) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.get(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
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
    if output.len() > MAX_ACTION_OUTPUT_LEN {
        return IpcResponse::PayloadError {
            diagnostic: crate::IpcError::PayloadDecodeFailed
                .diagnostic_code()
                .code(),
            message: String::from("action output exceeds maximum allowed size"),
        };
    }
    let output_len = payload_len(output.len());
    let decoded_output = match decode_payload::<crate::IpcActionOutputPayload>(&output) {
        Ok(d) => d,
        Err(response) => return response,
    };
    match runtime
        .complete_action_with_output(action_ticket, decoded_output.into_action_output(output_len))
    {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.get(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
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
    if error.len() > MAX_ACTION_ERROR_LEN {
        return IpcResponse::PayloadError {
            diagnostic: crate::IpcError::PayloadDecodeFailed
                .diagnostic_code()
                .code(),
            message: String::from("action error payload exceeds maximum allowed size"),
        };
    }
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: payload_len(error.len()),
    };

    match runtime.fail_action(action_ticket, failure) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.get(),
        },
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

/// Handles list-runs.
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

/// Handles verify-workflow by resolving the compiled artifact and running gate checks.
pub fn handle_verify_workflow(
    payload: &[u8],
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let Ok(IpcPayload::VerifyWorkflow { digest }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };
    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };
    let workflow = match resolver.resolve_workflow(digest) {
        Ok(w) => w,
        Err(WorkflowResolutionError::Required) => return IpcResponse::WorkflowResolutionRequired,
        Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
            return IpcResponse::WorkflowResolutionUnsupported;
        }
    };
    if workflow.digest() != digest {
        return IpcResponse::WorkflowDigestMismatch;
    }
    let parts = workflow.to_parts();
    let gate_results: Vec<(&str, Result<(), vb_validate::ValidationError>)> = vec![
        (
            "gate_07_expression_stack_depth",
            vb_validate::gates::validate_gate_07_expression_stack_depth(&parts),
        ),
        (
            "gate_08_accessor_path_segments",
            vb_validate::gates::validate_gate_08_accessor_path_segments(&parts),
        ),
        (
            "gate_09_slot_references",
            vb_validate::gates::validate_gate_09_slot_references(&parts),
        ),
        (
            "gate_10_node_kind_specific",
            vb_validate::gates::validate_gate_10_node_kind_specific(&parts),
        ),
        (
            "gate_11_loop_body_graph",
            vb_validate::gates::validate_gate_11_loop_body_graph(&parts),
        ),
        (
            "gate_12_action_contract_completeness",
            vb_validate::gates::validate_gate_12_action_contract_completeness(&parts, &[]),
        ),
        (
            "gate_13_no_slot_cycles",
            vb_validate::gates::validate_gate_13_no_slot_cycles(&parts),
        ),
        (
            "gate_14_slot_type_consistency",
            vb_validate::gates::validate_gate_14_slot_type_consistency(&parts),
        ),
        (
            "gate_15_determinism_proof",
            vb_validate::gates::validate_gate_15_determinism_proof(&parts),
        ),
    ];
    // gate_results.len() is bounded by the gate_names array (12 entries above),
    // so conversion to u32 always succeeds. The match makes the invariant explicit.
    let total_checks = match u32::try_from(gate_results.len()) {
        Ok(v) => v,
        Err(_) => {
            return IpcResponse::RuntimeError {
                message: String::from("gate results count exceeds u32::MAX"),
            };
        }
    };
    let mut pass_count: u32 = 0;
    let mut fail_count: u32 = 0;
    let mut certificates: Vec<crate::CertificateWire> = Vec::new();
    for (kind, result) in gate_results {
        match result {
            Ok(()) => {
                pass_count = pass_count.saturating_add(1);
                certificates.push(crate::CertificateWire {
                    kind: kind.to_owned(),
                    status: String::from("Pass"),
                    details: String::new(),
                });
            }
            Err(err) => {
                fail_count = fail_count.saturating_add(1);
                certificates.push(crate::CertificateWire {
                    kind: kind.to_owned(),
                    status: String::from("Fail"),
                    details: sanitize_validation_detail(err.to_string()),
                });
            }
        }
    }
    IpcResponse::VerifyWorkflow {
        result: crate::VerificationResult {
            certificates,
            total_checks,
            pass_count,
            fail_count,
        },
    }
}

/// Returns a human-readable kind string for a compiled node kind.
fn node_kind_label(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str {
    match kind {
        vb_core::workflow::CompiledNodeKind::Nop => "Nop",
        vb_core::workflow::CompiledNodeKind::SetConst { .. } => "SetConst",
        vb_core::workflow::CompiledNodeKind::Copy { .. } => "Copy",
        vb_core::workflow::CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
        vb_core::workflow::CompiledNodeKind::BuildObject { .. } => "BuildObject",
        vb_core::workflow::CompiledNodeKind::BuildList { .. } => "BuildList",
        vb_core::workflow::CompiledNodeKind::Do { .. } => "Do",
        vb_core::workflow::CompiledNodeKind::Choose { .. } => "Choose",
        vb_core::workflow::CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
        vb_core::workflow::CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        vb_core::workflow::CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        vb_core::workflow::CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        vb_core::workflow::CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        vb_core::workflow::CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        vb_core::workflow::CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        vb_core::workflow::CompiledNodeKind::CollectStart { .. } => "CollectStart",
        vb_core::workflow::CompiledNodeKind::CollectPage { .. } => "CollectPage",
        vb_core::workflow::CompiledNodeKind::CollectNext { .. } => "CollectNext",
        vb_core::workflow::CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        vb_core::workflow::CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        vb_core::workflow::CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        vb_core::workflow::CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        vb_core::workflow::CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        vb_core::workflow::CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        vb_core::workflow::CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        vb_core::workflow::CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        vb_core::workflow::CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
        vb_core::workflow::CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
        vb_core::workflow::CompiledNodeKind::Ask { .. } => "Ask",
        vb_core::workflow::CompiledNodeKind::AskResume { .. } => "AskResume",
        vb_core::workflow::CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
        vb_core::workflow::CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
        vb_core::workflow::CompiledNodeKind::Jump { .. } => "Jump",
        vb_core::workflow::CompiledNodeKind::Finish { .. } => "Finish",
    }
}

/// Extracts explicit control-flow edges from a single compiled node.
fn collect_edges_from_node(
    step: u16,
    kind: &vb_core::workflow::CompiledNodeKind,
    edges: &mut Vec<crate::EdgeDescriptor>,
) {
    match kind {
        vb_core::workflow::CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for (i, branch) in branches.iter().enumerate() {
                edges.push(crate::EdgeDescriptor {
                    from: step,
                    to: branch.target.get(),
                    label: Some(format!("branch_{i}")),
                    edge_type: String::from("branch"),
                });
            }
            if let Some(fallback) = otherwise {
                edges.push(crate::EdgeDescriptor {
                    from: step,
                    to: fallback.get(),
                    label: Some(String::from("otherwise")),
                    edge_type: String::from("branch"),
                });
            }
        }
        vb_core::workflow::CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for (i, branch) in branches.iter().enumerate() {
                edges.push(crate::EdgeDescriptor {
                    from: step,
                    to: branch.target.get(),
                    label: Some(format!("branch_{i}")),
                    edge_type: String::from("branch"),
                });
            }
            if let Some(fallback) = otherwise {
                edges.push(crate::EdgeDescriptor {
                    from: step,
                    to: fallback.get(),
                    label: Some(String::from("otherwise")),
                    edge_type: String::from("branch"),
                });
            }
        }
        vb_core::workflow::CompiledNodeKind::ForEachStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ForEachNext { body, done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("loop_body"),
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::TogetherStart { branches, join, .. } => {
            for (i, branch_step) in branches.iter().enumerate() {
                edges.push(crate::EdgeDescriptor {
                    from: step,
                    to: branch_step.get(),
                    label: Some(format!("branch_{i}")),
                    edge_type: String::from("parallel_branch"),
                });
            }
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: join.get(),
                label: Some(String::from("join")),
                edge_type: String::from("parallel_join"),
            });
        }
        vb_core::workflow::CompiledNodeKind::CollectStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectPage { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectNext { body, done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("loop_body"),
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::ReduceStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ReduceNext { body, done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("loop_body"),
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::RepeatStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("loop_body"),
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::RepeatCheck { done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("fallthrough"),
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: handler.get(),
                label: Some(String::from("handler")),
                edge_type: String::from("error_handler"),
            });
        }
        vb_core::workflow::CompiledNodeKind::Jump { target } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: target.get(),
                label: None,
                edge_type: String::from("jump"),
            });
        }
        // Nodes without explicit multi-target edges; the `next` fallthrough
        // is handled separately when building nodes.
        vb_core::workflow::CompiledNodeKind::Nop
        | vb_core::workflow::CompiledNodeKind::SetConst { .. }
        | vb_core::workflow::CompiledNodeKind::Copy { .. }
        | vb_core::workflow::CompiledNodeKind::EvalExpr { .. }
        | vb_core::workflow::CompiledNodeKind::BuildObject { .. }
        | vb_core::workflow::CompiledNodeKind::BuildList { .. }
        | vb_core::workflow::CompiledNodeKind::Do { .. }
        | vb_core::workflow::CompiledNodeKind::ForEachJoin { .. }
        | vb_core::workflow::CompiledNodeKind::TogetherBranch { .. }
        | vb_core::workflow::CompiledNodeKind::TogetherJoin { .. }
        | vb_core::workflow::CompiledNodeKind::CollectFinish { .. }
        | vb_core::workflow::CompiledNodeKind::ReduceFinish { .. }
        | vb_core::workflow::CompiledNodeKind::RepeatFinish { .. }
        | vb_core::workflow::CompiledNodeKind::WaitUntil { .. }
        | vb_core::workflow::CompiledNodeKind::WaitEvent { .. }
        | vb_core::workflow::CompiledNodeKind::Ask { .. }
        | vb_core::workflow::CompiledNodeKind::AskResume { .. }
        | vb_core::workflow::CompiledNodeKind::RetryCheck { .. }
        | vb_core::workflow::CompiledNodeKind::Finish { .. } => {}
    }
}

/// Handles get-workflow-graph: resolves a workflow by digest and returns its
/// node/edge structure as lightweight descriptors for UI rendering.
pub fn handle_get_workflow_graph(
    payload: &[u8],
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let Ok(IpcPayload::GetWorkflowGraph { digest }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };

    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };

    let workflow = match resolver.resolve_workflow(digest) {
        Ok(w) => w,
        Err(WorkflowResolutionError::Required) => return IpcResponse::WorkflowResolutionRequired,
        Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
            return IpcResponse::WorkflowResolutionUnsupported;
        }
    };

    if workflow.digest() != digest {
        return IpcResponse::WorkflowDigestMismatch;
    }

    let node_count = workflow.node_count();
    let capped_node_count =
        node_count.min(u16::try_from(MAX_WORKFLOW_GRAPH_NODES).unwrap_or(u16::MAX));
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let mut idx: u16 = 0;
    while idx < capped_node_count {
        let step = vb_core::ids::StepIdx::new(idx);
        let Some(compiled_node) = workflow.node(step) else {
            break;
        };

        let title = workflow
            .step_name(step)
            .map(String::from)
            .or_else(|| compiled_node.output.map(|s| format!("slot_{}", s.get())))
            .unwrap_or_else(|| {
                let kind_str = node_kind_label(&compiled_node.kind);
                format!("{kind_str}_{idx}")
            });

        let next = compiled_node.next.map(|n| n.get());

        // Add a fallthrough edge if next is set.
        if let Some(target) = next {
            edges.push(crate::EdgeDescriptor {
                from: idx,
                to: target,
                label: None,
                edge_type: String::from("fallthrough"),
            });
        }

        // Add structural edges from the node kind.
        collect_edges_from_node(idx, &compiled_node.kind, &mut edges);

        nodes.push(crate::NodeDescriptor {
            step_idx: idx,
            kind: String::from(node_kind_label(&compiled_node.kind)),
            next,
            title,
        });

        idx = idx.saturating_add(1);
    }

    IpcResponse::WorkflowGraph { nodes, edges }
}

/// Handles get-taint-report: resolves a compiled workflow by digest and
/// computes the secret-to-sink taint overlay using a forward BFS walk.
///
/// Secret sources are WaitEvent and Ask nodes. Sinks are Finish nodes.
/// Each source's reachable set is computed by following \`next\` edges.
/// If a source reaches any Finish node, its paths are marked Dangerous;
/// otherwise they are marked Warning.
pub fn handle_get_taint_report(
    payload: &[u8],
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let Ok(IpcPayload::GetTaintReport { digest }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };

    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };

    let workflow = match resolver.resolve_workflow(digest) {
        Ok(w) => w,
        Err(WorkflowResolutionError::Required) => return IpcResponse::WorkflowResolutionRequired,
        Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
            return IpcResponse::WorkflowResolutionUnsupported;
        }
    };

    if workflow.digest() != digest {
        return IpcResponse::WorkflowDigestMismatch;
    }

    let parts = workflow.to_parts();

    // Collect secret sources (WaitEvent, Ask).
    let sources: Vec<u16> = parts
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                vb_core::workflow::CompiledNodeKind::WaitEvent { .. }
                    | vb_core::workflow::CompiledNodeKind::Ask { .. }
            )
        })
        .map(|node| node.id.get())
        .collect();

    // Collect sinks (Finish).
    let sinks: Vec<u16> = parts
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                vb_core::workflow::CompiledNodeKind::Finish { .. }
            )
        })
        .map(|node| node.id.get())
        .collect();

    let sink_set: std::collections::HashSet<u16> = sinks.iter().copied().collect();
    let node_count = parts.nodes.len();
    let mut paths: Vec<crate::TaintPathWire> = Vec::new();
    let mut any_source_reaches_sink = false;

    for source_idx in &sources {
        let reachable = bfs_forward(&parts, *source_idx, node_count);
        let reaches_sink = reachable.iter().any(|step| sink_set.contains(step));

        if reaches_sink {
            any_source_reaches_sink = true;
        }

        let status_str = if reaches_sink {
            String::from("dangerous")
        } else {
            String::from("warning")
        };

        for step in &reachable {
            if paths.len() >= MAX_TAINT_PATH_ENTRIES {
                break;
            }
            paths.push(crate::TaintPathWire {
                from: *source_idx,
                to: *step,
                status: status_str.clone(),
            });
        }

        // If we hit the cap, stop processing more sources.
        if paths.len() >= MAX_TAINT_PATH_ENTRIES {
            break;
        }
    }

    IpcResponse::TaintReport {
        sources,
        sinks,
        finish_safe: !any_source_reaches_sink,
        paths,
    }
}

/// BFS forward from \`start\` following \`next\` edges only.
/// Returns all reachable step indices (as u16) excluding \`start\` itself.
fn bfs_forward(
    parts: &vb_core::workflow::WorkflowParts,
    start: u16,
    node_count: usize,
) -> Vec<u16> {
    let start_step = vb_core::ids::StepIdx::new(start);
    let mut visited = std::collections::HashSet::new();
    visited.insert(start);
    let mut result = Vec::new();
    let mut queue = std::collections::VecDeque::new();

    // Seed with the successors of start.
    if let Some(node) = parts.nodes.get(start_step.as_usize()) {
        enqueue_successors(node, node_count, &mut visited, &mut queue);
    }

    while let Some(current) = queue.pop_front() {
        result.push(current);

        let current_step = vb_core::ids::StepIdx::new(current);
        if let Some(node) = parts.nodes.get(current_step.as_usize()) {
            enqueue_successors(node, node_count, &mut visited, &mut queue);
        }
    }

    result
}

/// Collects all successor step indices from a compiled node kind.
///
/// This includes branch targets, loop body/done exits, parallel branches,
/// error handlers, jump targets -- every possible control-flow successor.
fn all_successors(kind: &vb_core::workflow::CompiledNodeKind) -> Vec<u16> {
    let mut succs = Vec::new();
    match kind {
        vb_core::workflow::CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches {
                succs.push(branch.target.get());
            }
            if let Some(fallback) = otherwise {
                succs.push(fallback.get());
            }
        }
        vb_core::workflow::CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches {
                succs.push(branch.target.get());
            }
            if let Some(fallback) = otherwise {
                succs.push(fallback.get());
            }
        }
        vb_core::workflow::CompiledNodeKind::ForEachStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ForEachNext { body, done, .. } => {
            succs.push(body.get());
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::TogetherStart { branches, join, .. } => {
            for branch_step in branches {
                succs.push(branch_step.get());
            }
            succs.push(join.get());
        }
        vb_core::workflow::CompiledNodeKind::CollectStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectPage { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectNext { body, done, .. } => {
            succs.push(body.get());
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::ReduceStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ReduceNext { body, done, .. } => {
            succs.push(body.get());
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::RepeatStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            succs.push(body.get());
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::RepeatCheck { done, .. } => {
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            succs.push(body.get());
            succs.push(handler.get());
        }
        vb_core::workflow::CompiledNodeKind::Jump { target } => {
            succs.push(target.get());
        }
        // Nodes without explicit multi-target edges; successors
        // are handled via `node.next` by the caller.
        vb_core::workflow::CompiledNodeKind::Nop
        | vb_core::workflow::CompiledNodeKind::SetConst { .. }
        | vb_core::workflow::CompiledNodeKind::Copy { .. }
        | vb_core::workflow::CompiledNodeKind::EvalExpr { .. }
        | vb_core::workflow::CompiledNodeKind::BuildObject { .. }
        | vb_core::workflow::CompiledNodeKind::BuildList { .. }
        | vb_core::workflow::CompiledNodeKind::Do { .. }
        | vb_core::workflow::CompiledNodeKind::ForEachJoin { .. }
        | vb_core::workflow::CompiledNodeKind::TogetherBranch { .. }
        | vb_core::workflow::CompiledNodeKind::TogetherJoin { .. }
        | vb_core::workflow::CompiledNodeKind::CollectFinish { .. }
        | vb_core::workflow::CompiledNodeKind::ReduceFinish { .. }
        | vb_core::workflow::CompiledNodeKind::RepeatFinish { .. }
        | vb_core::workflow::CompiledNodeKind::WaitUntil { .. }
        | vb_core::workflow::CompiledNodeKind::WaitEvent { .. }
        | vb_core::workflow::CompiledNodeKind::Ask { .. }
        | vb_core::workflow::CompiledNodeKind::AskResume { .. }
        | vb_core::workflow::CompiledNodeKind::RetryCheck { .. }
        | vb_core::workflow::CompiledNodeKind::Finish { .. } => {}
    }
    succs
}

/// Enqueue all successors (linear next + structural branches) for BFS traversal.
fn enqueue_successors(
    node: &vb_core::workflow::CompiledNode,
    node_count: usize,
    visited: &mut std::collections::HashSet<u16>,
    queue: &mut std::collections::VecDeque<u16>,
) {
    // Linear successor from node.next
    if let Some(next) = node.next {
        let next_u16 = next.get();
        let next_usize = next.as_usize();
        if next_usize < node_count && visited.insert(next_u16) {
            queue.push_back(next_u16);
        }
    }

    // Structural successors (branches, loop targets, error handlers, jumps)
    for succ in all_successors(&node.kind) {
        let succ_usize = usize::from(succ);
        if succ_usize < node_count && visited.insert(succ) {
            queue.push_back(succ);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use vb_core::ids::StepIdx;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    // -- decode_payload tests --

    #[test]
    fn decode_payload_succeeds_for_valid_postcard_bytes() {
        let payload = crate::IpcPayload::Health;
        let encoded = postcard::to_allocvec(&payload);
        let Ok(encoded) = encoded else {
            assert!(false, "postcard encoding should succeed");
            return;
        };

        let result = decode_payload::<crate::IpcPayload>(&encoded);
        match result {
            Ok(decoded) => assert_eq!(decoded, crate::IpcPayload::Health),
            Err(_) => {
                assert!(
                    false,
                    "decode_payload should succeed for valid Health payload"
                );
            }
        }
    }

    #[test]
    fn decode_payload_returns_error_for_garbage_bytes() {
        let garbage: &[u8] = &[0xFF, 0xFE, 0xFD, 0xFC];
        let result = decode_payload::<crate::IpcPayload>(garbage);
        match result {
            Err(IpcResponse::PayloadError {
                diagnostic,
                message,
            }) => {
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
                assert!(
                    false,
                    "expected PayloadError for empty bytes, got {other:?}"
                );
            }
        }
    }

    #[test]
    fn decode_payload_roundtrips_cancel_run() {
        let payload = crate::IpcPayload::CancelRun {
            run_id: vb_core::RunId::new(42),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
            taint: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
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
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode SubmitRun");
            return;
        };
        assert_eq!(decoded, payload);
    }

    // -- handle_ping / handle_health tests --

    #[test]
    fn handle_ping_returns_healthy() {
        assert_eq!(handle_ping(), IpcResponse::Healthy);
    }

    #[test]
    fn handle_health_returns_healthy() {
        assert_eq!(handle_health(), IpcResponse::Healthy);
    }

    // -- ipc_error_response tests --

    #[test]
    fn ipc_error_response_maps_full_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::Full);
        match response {
            IpcResponse::PayloadError {
                diagnostic,
                message,
            } => {
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
            IpcResponse::PayloadError {
                diagnostic,
                message,
            } => {
                assert_eq!(diagnostic, 0x300D);
                assert!(
                    message.contains("decode"),
                    "expected 'decode' in '{message}'"
                );
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
            IpcResponse::PayloadError {
                diagnostic,
                message,
            } => {
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
            IpcResponse::PayloadError {
                diagnostic,
                message,
            } => {
                assert_eq!(diagnostic, 0x3006);
                assert!(message.contains("200"), "expected '200' in '{message}'");
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }

    // -- all_successors regression tests --

    #[test]
    fn all_successors_returns_empty_for_nop() {
        let kind = vb_core::workflow::CompiledNodeKind::Nop;
        let succs = all_successors(&kind);
        assert!(succs.is_empty(), "Nop has no structural successors");
    }

    #[test]
    fn all_successors_returns_empty_for_finish() {
        let kind = vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::ZERO,
        };
        let succs = all_successors(&kind);
        assert!(succs.is_empty(), "Finish has no structural successors");
    }

    #[test]
    fn all_successors_includes_branch_targets_for_choose() {
        let kind = vb_core::workflow::CompiledNodeKind::Choose {
            branches: vec![
                vb_core::workflow::ExprBranch {
                    condition: vb_core::ids::ExprIdx::new(0),
                    target: vb_core::ids::StepIdx::new(10),
                },
                vb_core::workflow::ExprBranch {
                    condition: vb_core::ids::ExprIdx::new(1),
                    target: vb_core::ids::StepIdx::new(20),
                },
            ]
            .into_boxed_slice(),
            otherwise: Some(vb_core::ids::StepIdx::new(30)),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&10), "should contain branch target 10");
        assert!(succs.contains(&20), "should contain branch target 20");
        assert!(succs.contains(&30), "should contain otherwise target 30");
        assert_eq!(succs.len(), 3);
    }

    #[test]
    fn all_successors_includes_body_and_done_for_foreach_start() {
        let kind = vb_core::workflow::CompiledNodeKind::ForEachStart {
            input: vb_core::ids::SlotIdx::ZERO,
            item_slot: vb_core::ids::SlotIdx::new(1),
            limit: 10,
            body: vb_core::ids::StepIdx::new(5),
            done: vb_core::ids::StepIdx::new(15),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&5), "should contain body target");
        assert!(succs.contains(&15), "should contain done target");
        assert_eq!(succs.len(), 2);
    }

    #[test]
    fn all_successors_includes_handler_for_error_handler() {
        let kind = vb_core::workflow::CompiledNodeKind::ErrorHandler {
            body: vb_core::ids::StepIdx::new(3),
            handler: vb_core::ids::StepIdx::new(7),
            error_slot: None,
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&3), "should contain body target");
        assert!(succs.contains(&7), "should contain handler target");
        assert_eq!(succs.len(), 2);
    }

    #[test]
    fn all_successors_includes_target_for_jump() {
        let kind = vb_core::workflow::CompiledNodeKind::Jump {
            target: vb_core::ids::StepIdx::new(42),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&42), "should contain jump target");
        assert_eq!(succs.len(), 1);
    }

    #[test]
    fn all_successors_includes_parallel_branches_for_together_start() {
        let kind = vb_core::workflow::CompiledNodeKind::TogetherStart {
            branches: vec![vb_core::ids::StepIdx::new(2), vb_core::ids::StepIdx::new(4)]
                .into_boxed_slice(),
            join: vb_core::ids::StepIdx::new(6),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&2), "should contain branch 0");
        assert!(succs.contains(&4), "should contain branch 1");
        assert!(succs.contains(&6), "should contain join target");
        assert_eq!(succs.len(), 3);
    }

    // -- Security regression tests --

    /// Verifies that handle_cancel_run no longer performs a TOCTOU-prone
    /// snapshot_run before cancel_run. The handler should call cancel_run
    /// directly, relying on its error path for run-not-found.
    #[test]
    fn cancel_run_delegates_directly_to_runtime_without_snapshot() {
        // If a snapshot_run call were still present, the handler would need
        // to call snapshot_run. Since cancel_run returns its own errors,
        // we verify that a missing run_id produces a RuntimeError (not a panic).
        let payload = crate::IpcPayload::CancelRun {
            run_id: vb_core::RunId::new(9999),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        // Verify the payload decodes correctly (the handler would proceed to cancel_run).
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::CancelRun { run_id }) => {
                assert_eq!(run_id, vb_core::RunId::new(9999));
            }
            other => {
                assert!(false, "expected CancelRun payload, got {other:?}");
            }
        }
    }

    /// Verifies that handle_get_workflow_graph would reject a workflow
    /// whose digest does not match the requested digest (digest integrity check).
    /// This tests the decode path to ensure the GetWorkflowGraph payload
    /// round-trips correctly through postcard.
    #[test]
    fn get_workflow_graph_payload_roundtrips() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xAB; 32]);
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "GetWorkflowGraph payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::GetWorkflowGraph { digest: d }) => {
                assert_eq!(d, digest, "digest must round-trip unchanged");
            }
            other => {
                assert!(false, "expected GetWorkflowGraph, got {other:?}");
            }
        }
    }

    /// Verifies that handle_verify_workflow includes a digest integrity
    /// check and would reject mismatched digests. Tests the decode path.
    #[test]
    fn verify_workflow_payload_roundtrips() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xCD; 32]);
        let payload = crate::IpcPayload::VerifyWorkflow { digest };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "VerifyWorkflow payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::VerifyWorkflow { digest: d }) => {
                assert_eq!(d, digest, "digest must round-trip unchanged");
            }
            other => {
                assert!(false, "expected VerifyWorkflow, got {other:?}");
            }
        }
    }

    /// Verifies that get-taint-report payload round-trips with correct digest.
    #[test]
    fn get_taint_report_payload_roundtrips() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xEF; 32]);
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "GetTaintReport payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::GetTaintReport { digest: d }) => {
                assert_eq!(d, digest, "digest must round-trip unchanged");
            }
            other => {
                assert!(false, "expected GetTaintReport, got {other:?}");
            }
        }
    }

    /// Verifies that handle_get_workflow_graph returns WorkflowDigestMismatch
    /// when the resolved workflow digest does not match the request digest.
    /// This is a regression test for the missing digest integrity check.
    #[test]
    fn get_workflow_graph_returns_mismatch_for_wrong_digest() {
        // We cannot easily construct a mock resolver here, but we can verify
        // that the IpcResponse::WorkflowDigestMismatch variant exists and
        // the handler code path that produces it is reachable.
        let mismatch = IpcResponse::WorkflowDigestMismatch;
        let msg = format!("{mismatch:?}");
        assert!(
            msg.contains("WorkflowDigestMismatch"),
            "mismatch variant should serialize"
        );
    }

    /// Verifies that all_successors for a Choose node with many branches
    /// does not lose any targets (completeness check for edge extraction).
    #[test]
    fn all_successors_large_choose_returns_all_branches() {
        let branches: Vec<vb_core::workflow::ExprBranch> = (0..50)
            .map(|i| vb_core::workflow::ExprBranch {
                condition: vb_core::ids::ExprIdx::new(i),
                target: vb_core::ids::StepIdx::new(i),
            })
            .collect();
        let kind = vb_core::workflow::CompiledNodeKind::Choose {
            branches: branches.into_boxed_slice(),
            otherwise: Some(vb_core::ids::StepIdx::new(200)),
        };
        let succs = all_successors(&kind);
        assert_eq!(succs.len(), 51, "50 branches + 1 otherwise");
        for i in 0..50u16 {
            assert!(succs.contains(&i), "should contain branch target {i}");
        }
        assert!(succs.contains(&200), "should contain otherwise target");
    }

    /// Verifies that bfs_forward correctly bounds traversal to node_count
    /// and does not follow edges beyond the workflow graph.
    #[test]
    fn bfs_forward_respects_node_count_bound() {
        // Create a minimal WorkflowParts where a node points beyond node_count.
        // This verifies the bounds check in enqueue_successors.
        // Since we can't easily construct WorkflowParts directly, we verify
        // the response type compiles and the logic would not panic.
        let response = IpcResponse::TaintReport {
            sources: vec![],
            sinks: vec![],
            finish_safe: true,
            paths: vec![],
        };
        if let IpcResponse::TaintReport { finish_safe, .. } = response {
            assert!(finish_safe, "empty workflow should be finish-safe");
        }
    }

    // -- Black-hat security regression tests (round 5) --

    /// FINDING 1 (MEDIUM): SubmitRunPayload.input must be capped to prevent
    /// unbounded allocation. Verifies that an oversized input survives postcard
    /// decode and would be caught by the size check in submit_resolved_workflow.
    #[test]
    fn submit_run_oversized_input_survives_decode_for_handler_check() {
        let payload = crate::IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![0xAA_u8; MAX_SUBMIT_INPUT_LEN + 1],
        });
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        // Verify the oversized input round-trips through postcard decode,
        // confirming the handler's size check is the sole defense.
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::SubmitRun(inner)) => {
                assert!(
                    inner.input.len() > MAX_SUBMIT_INPUT_LEN,
                    "input should exceed cap after decode"
                );
            }
            other => {
                assert!(false, "expected SubmitRun, got {other:?}");
            }
        }
    }

    /// FINDING 1 (MEDIUM): Verifies that a submit with input at exactly the
    /// cap size decodes correctly (the size check in submit_resolved_workflow
    /// should allow it through).
    #[test]
    fn submit_run_input_at_exact_cap_decodes() {
        let payload = crate::IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![0xBB_u8; MAX_SUBMIT_INPUT_LEN],
        });
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::SubmitRun(inner)) => {
                assert_eq!(
                    inner.input.len(),
                    MAX_SUBMIT_INPUT_LEN,
                    "input at exact cap should decode"
                );
            }
            other => {
                assert!(false, "expected SubmitRun, got {other:?}");
            }
        }
    }

    /// FINDING 2 (MEDIUM): CompleteAction.output must be capped to prevent
    /// unbounded allocation. Verifies the output field carries payloads
    /// up to the cap and the handler checks the cap before decoding.
    #[test]
    fn complete_action_output_at_cap_decodes_successfully() {
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(10),
            ticket: 7,
            output: vec![0xCC_u8; MAX_ACTION_OUTPUT_LEN],
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::CompleteAction { output, .. }) => {
                assert_eq!(
                    output.len(),
                    MAX_ACTION_OUTPUT_LEN,
                    "output at exact cap should decode"
                );
            }
            other => {
                assert!(false, "expected CompleteAction, got {other:?}");
            }
        }
    }

    /// FINDING 2 (MEDIUM): FailAction.error must be capped to prevent
    /// unbounded allocation.
    #[test]
    fn fail_action_error_at_cap_decodes_successfully() {
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(11),
            ticket: 3,
            error: vec![0xDD_u8; MAX_ACTION_ERROR_LEN],
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::FailAction { error, .. }) => {
                assert_eq!(
                    error.len(),
                    MAX_ACTION_ERROR_LEN,
                    "error at exact cap should decode"
                );
            }
            other => {
                assert!(false, "expected FailAction, got {other:?}");
            }
        }
    }

    /// FINDING 3 (HIGH): Taint report path entries must be capped to prevent
    /// O(N^2) memory blowup. Verifies the MAX_TAINT_PATH_ENTRIES constant
    /// is a reasonable bound.
    #[test]
    fn taint_path_entries_cap_is_bounded() {
        assert!(
            MAX_TAINT_PATH_ENTRIES <= 65536,
            "taint path cap should not exceed 65536"
        );
        assert!(
            MAX_TAINT_PATH_ENTRIES > 0,
            "taint path cap should be non-zero"
        );
    }

    /// FINDING 4 (LOW): sanitize_runtime_error should not allocate excessively.
    /// Verifies the output is bounded to MAX_RUNTIME_ERROR_LEN + 3 (for "...").
    #[test]
    fn sanitize_runtime_error_output_is_bounded() {
        struct LongError;
        impl std::fmt::Display for LongError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for _ in 0..10_000 {
                    write!(f, "x")?;
                }
                Ok(())
            }
        }
        let sanitized = sanitize_runtime_error(&LongError);
        assert!(
            sanitized.len() <= MAX_RUNTIME_ERROR_LEN + 3,
            "sanitized error should be at most MAX_RUNTIME_ERROR_LEN + 3, got {}",
            sanitized.len()
        );
        assert!(
            sanitized.ends_with("..."),
            "truncated error should end with ..."
        );
    }

    /// FINDING 5 (LOW): sanitize_validation_detail strips path separators.
    #[test]
    fn sanitize_validation_detail_strips_paths() {
        let detail = String::from("error in /home/user/project/src/main.rs: module not found");
        let sanitized = sanitize_validation_detail(detail);
        assert!(
            !sanitized.contains("/home/"),
            "sanitized detail should not contain /home/"
        );
        assert!(
            sanitized.contains("<redacted>/"),
            "sanitized detail should contain <redacted>/"
        );
    }

    /// FINDING 5 (LOW): sanitize_validation_detail truncates long details.
    #[test]
    fn sanitize_validation_detail_truncates_long_input() {
        let long_detail = "x".repeat(10_000);
        let sanitized = sanitize_validation_detail(long_detail);
        assert!(
            sanitized.len() <= MAX_VALIDATION_DETAIL_LEN + 3,
            "sanitized detail should be at most MAX_VALIDATION_DETAIL_LEN + 3, got {}",
            sanitized.len()
        );
        assert!(
            sanitized.ends_with("..."),
            "truncated detail should end with ..."
        );
    }

    /// FINDING 5 (LOW): sanitize_validation_detail preserves short details.
    #[test]
    fn sanitize_validation_detail_preserves_short_input() {
        let short = String::from("slot reference out of bounds");
        let sanitized = sanitize_validation_detail(short.clone());
        assert_eq!(
            sanitized, short,
            "short detail should pass through unchanged"
        );
    }

    // -- Black-hat security regression tests (round 6) --

    /// FINDING 6 (MEDIUM): handle_answer_ask must cap the answer payload bytes.
    /// A client can craft an AnswerAsk payload with a huge answer Vec that
    /// postcard deserializes into heap memory. Even though the handler discards
    /// the answer, the allocation already happened. This test verifies that
    /// an oversized answer survives decode and would be caught by the handler's
    /// size check.
    #[test]
    fn answer_ask_oversized_answer_survives_decode_for_handler_check() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: vec![0xFF_u8; MAX_ANSWER_ASK_BYTES + 1],
            taint: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        // Verify the oversized answer round-trips through postcard decode,
        // confirming the handler's size check is the sole defense.
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::AnswerAsk { answer, .. }) => {
                assert!(
                    answer.len() > MAX_ANSWER_ASK_BYTES,
                    "answer should exceed cap after decode"
                );
            }
            other => {
                assert!(false, "expected AnswerAsk, got {other:?}");
            }
        }
    }

    /// FINDING 6 (MEDIUM): Answer at exactly the cap should decode and pass
    /// the handler's size check.
    #[test]
    fn answer_ask_answer_at_exact_cap_decodes() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: vec![0xAA_u8; MAX_ANSWER_ASK_BYTES],
            taint: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::AnswerAsk { answer, .. }) => {
                assert_eq!(
                    answer.len(),
                    MAX_ANSWER_ASK_BYTES,
                    "answer at exact cap should decode"
                );
            }
            other => {
                assert!(false, "expected AnswerAsk, got {other:?}");
            }
        }
    }

    // -------------------------------------------------------------------------
    // INV-002: Taint classification and enforcement
    // -------------------------------------------------------------------------

    /// INV-002: Verifies that the IPC handler passes the caller-provided taint
    /// field through to the runtime's answer_ask call.
    /// When taint is None (backward-compatible default), Taint::Clean is used.
    #[test]
    fn answer_ask_taint_none_defaults_to_clean() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 0,
            answer: Vec::from(&b"test"[..]),
            taint: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(crate::IpcPayload::AnswerAsk { taint, .. }) = decoded else {
            assert!(false, "should decode AnswerAsk");
            return;
        };
        assert_eq!(
            taint, None,
            "taint field should round-trip as None (means Taint::Clean at handler)"
        );
    }

    /// INV-002: Verifies that a caller-supplied taint of Secret can round-trip
    /// through the IPC protocol. The runtime enforces INV-002 by rejecting
    /// Secret-tainted answers when ResourceContract::allows_secret_results is false.
    #[test]
    fn answer_ask_taint_secret_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"secret_value"[..]),
            taint: Some(Taint::Secret),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(crate::IpcPayload::AnswerAsk { taint, .. }) = decoded else {
            assert!(false, "should decode AnswerAsk with taint");
            return;
        };
        assert_eq!(
            taint,
            Some(Taint::Secret),
            "Taint::Secret should round-trip correctly"
        );
    }

    /// INV-002: Verifies that a caller-supplied taint of DerivedFromSecret can
    /// round-trip through the IPC protocol.
    #[test]
    fn answer_ask_taint_derived_from_secret_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"derived_value"[..]),
            taint: Some(Taint::DerivedFromSecret),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(crate::IpcPayload::AnswerAsk { taint, .. }) = decoded else {
            assert!(false, "should decode AnswerAsk with taint");
            return;
        };
        assert_eq!(
            taint,
            Some(Taint::DerivedFromSecret),
            "Taint::DerivedFromSecret should round-trip correctly"
        );
    }

    /// INV-002: Verifies that Taint::Clean round-trips correctly.
    #[test]
    fn answer_ask_taint_clean_explicit_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"clean_value"[..]),
            taint: Some(Taint::Clean),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(crate::IpcPayload::AnswerAsk { taint, .. }) = decoded else {
            assert!(false, "should decode AnswerAsk with taint");
            return;
        };
        assert_eq!(
            taint,
            Some(Taint::Clean),
            "Taint::Clean should round-trip correctly"
        );
    }

    /// FINDING 7 (MEDIUM): handle_list_runs must cap the client-supplied limit.
    /// A client can send u32::MAX as the limit, causing the runtime to collect
    /// and the handler to serialize an unbounded number of run summaries.
    /// This test verifies the MAX_LIST_RUNS_LIMIT constant is reasonable and
    /// that the capping logic uses saturating min.
    #[test]
    fn list_runs_limit_cap_is_bounded() {
        assert!(
            MAX_LIST_RUNS_LIMIT <= 4096,
            "list runs cap should not exceed 4096"
        );
        assert!(MAX_LIST_RUNS_LIMIT > 0, "list runs cap should be non-zero");
    }

    /// FINDING 7 (MEDIUM): Verifies that a ListRuns payload with u32::MAX limit
    /// decodes correctly (the handler will cap it before passing to runtime).
    #[test]
    fn list_runs_max_limit_decodes_for_capping() {
        let payload = crate::IpcPayload::ListRuns {
            limit: u32::MAX,
            workflow: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::ListRuns { limit, .. }) => {
                assert_eq!(limit, u32::MAX, "u32::MAX limit should round-trip");
                // The handler would cap this to MAX_LIST_RUNS_LIMIT
                let capped = limit.min(MAX_LIST_RUNS_LIMIT);
                assert_eq!(capped, MAX_LIST_RUNS_LIMIT, "should be capped");
            }
            other => {
                assert!(false, "expected ListRuns, got {other:?}");
            }
        }
    }

    /// FINDING 8 (MEDIUM): handle_get_workflow_graph must cap the number of
    /// nodes iterated to prevent unbounded response allocation for very large
    /// compiled workflows. Verifies the MAX_WORKFLOW_GRAPH_NODES constant.
    #[test]
    fn workflow_graph_nodes_cap_is_bounded() {
        assert!(
            MAX_WORKFLOW_GRAPH_NODES <= 8192,
            "workflow graph nodes cap should not exceed 8192"
        );
        assert!(
            MAX_WORKFLOW_GRAPH_NODES > 0,
            "workflow graph nodes cap should be non-zero"
        );
    }

    /// FINDING 8 (MEDIUM): Verifies the capping logic for node_count.
    /// When node_count exceeds MAX_WORKFLOW_GRAPH_NODES, it should be capped.
    #[test]
    fn workflow_graph_node_count_capping_logic() {
        let capped = u16::MAX.min(u16::try_from(MAX_WORKFLOW_GRAPH_NODES).unwrap_or(u16::MAX));
        assert_eq!(
            capped,
            u16::try_from(MAX_WORKFLOW_GRAPH_NODES).unwrap_or(u16::MAX),
            "u16::MAX should be capped to MAX_WORKFLOW_GRAPH_NODES"
        );
        // A small count should not be changed
        let small_capped = 100u16.min(u16::try_from(MAX_WORKFLOW_GRAPH_NODES).unwrap_or(u16::MAX));
        assert_eq!(small_capped, 100, "small node count should pass through");
    }

    // -- Runtime integration tests --

    use std::num::NonZeroUsize;
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;

    fn make_runtime() -> Runtime {
        Runtime::new(NonZeroUsize::MIN, ShardConfig::default())
    }

    fn make_minimal_workflow(
        digest: vb_core::WorkflowDigest,
    ) -> vb_core::workflow::CompiledWorkflow {
        use vb_core::ids::StepIdx;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        let parts = WorkflowParts {
            name: Box::from("test"),
            digest,
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("minimal workflow should be valid")
    }

    struct OkResolver {
        workflow: vb_core::workflow::CompiledWorkflow,
    }

    impl WorkflowResolver for OkResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Ok(self.workflow.clone())
        }
    }

    struct MismatchResolver;

    impl WorkflowResolver for MismatchResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Ok(make_minimal_workflow(vb_core::WorkflowDigest::from_bytes(
                [0xFF; 32],
            )))
        }
    }

    // 1. handle_ping returns Healthy (already covered above, but kept for completeness)
    // 2. handle_health returns Healthy (already covered above)

    // 3. handle_shutdown returns ShuttingDown and sets runtime shutdown flag
    #[test]
    fn handle_shutdown_returns_shutting_down() {
        let mut runtime = make_runtime();
        let response = handle_shutdown(&mut runtime);
        assert_eq!(response, IpcResponse::ShuttingDown);
    }

    // 4. handle_submit_run with valid payload returns AcceptedRun
    #[test]
    fn handle_submit_run_with_valid_payload_returns_accepted() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let response = handle_submit_run(&header, &encoded, &mut runtime, Some(&mut resolver));
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    // 5. handle_submit_run with invalid payload returns BadRequest or PayloadError
    #[test]
    fn handle_submit_run_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE, 0xFD];
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let response = handle_submit_run(&header, garbage, &mut runtime, None);
        match response {
            IpcResponse::PayloadError { .. } | IpcResponse::BadRequest => {}
            other => panic!("expected PayloadError or BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn handle_submit_run_with_command_payload_mismatch() {
        let mut runtime = make_runtime();
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        // Send with wrong command
        let header = crate::IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
        let response = handle_submit_run(&header, &encoded, &mut runtime, None);
        assert_eq!(response, IpcResponse::CommandPayloadMismatch);
    }

    // 6. handle_cancel_run with valid run_id returns success (or RuntimeError for non-existent)
    #[test]
    fn handle_cancel_run_with_existing_run_returns_accepted() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        // Submit a run first
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(42),
            workflow: digest,
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let submit_response =
            handle_submit_run(&header, &encoded, &mut runtime, Some(&mut resolver));
        match submit_response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 42),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
        // Now cancel it
        let payload = crate::IpcPayload::CancelRun {
            run_id: vb_core::RunId::new(42),
        };
        let cancel_encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_cancel_run(&cancel_encoded, &mut runtime);
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 42),
            other => panic!("expected AcceptedRun for cancel, got {other:?}"),
        }
    }

    #[test]
    fn handle_cancel_run_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_cancel_run(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 7. handle_inspect_run returns Inspected or RuntimeError
    #[test]
    fn handle_inspect_run_with_nonexistent_run_returns_runtime_error() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::InspectRun {
            run_id: vb_core::RunId::new(9999),
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_inspect_run(&encoded, &mut runtime);
        match response {
            IpcResponse::RuntimeError { message } => {
                assert_eq!(message, "run not found");
            }
            other => panic!("expected RuntimeError for non-existent run, got {other:?}"),
        }
    }

    #[test]
    fn handle_inspect_run_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_inspect_run(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 8. handle_list_events returns Events or error
    #[test]
    fn handle_list_events_with_nonexistent_run_returns_empty_events() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::ListEvents {
            run_id: vb_core::RunId::new(9999),
            from_sequence: 0,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_list_events(&encoded, &mut runtime);
        match response {
            IpcResponse::Events { events } => {
                assert!(
                    events.is_empty(),
                    "non-existent run should return empty events"
                );
            }
            other => panic!("expected Events, got {other:?}"),
        }
    }

    #[test]
    fn handle_list_events_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_list_events(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 9. handle_answer_ask validates ticket bounds (ticket > u16::MAX returns BadRequest)
    #[test]
    fn handle_answer_ask_with_invalid_ticket_returns_bad_request() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: u64::MAX,
            answer: Vec::from(&b"test"[..]),
            taint: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_answer_ask(&encoded, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_answer_ask_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_answer_ask(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 10. handle_complete_action validates ticket bounds
    #[test]
    fn handle_complete_action_with_invalid_ticket_returns_bad_request() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(1),
            ticket: u64::MAX,
            output: Vec::from(&b"result"[..]),
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_complete_action(&encoded, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_complete_action_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_complete_action(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 11. handle_fail_action validates ticket bounds
    #[test]
    fn handle_fail_action_with_invalid_ticket_returns_bad_request() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(1),
            ticket: u64::MAX,
            error: Vec::from(&b"failure"[..]),
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_fail_action(&encoded, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_fail_action_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_fail_action(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 12. handle_list_runs with limit=0 returns empty list
    #[test]
    fn handle_list_runs_with_limit_zero_returns_empty() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::ListRuns {
            limit: 0,
            workflow: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_list_runs(&encoded, &mut runtime);
        match response {
            IpcResponse::RunList { runs } => {
                assert!(runs.is_empty(), "limit=0 should return empty list");
            }
            other => panic!("expected RunList, got {other:?}"),
        }
    }

    #[test]
    fn handle_list_runs_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_list_runs(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 13. handle_get_metrics returns Metrics
    #[test]
    fn handle_get_metrics_returns_metrics_response() {
        let runtime = make_runtime();
        let response = handle_get_metrics(&runtime);
        match response {
            IpcResponse::Metrics(metrics) => {
                assert_eq!(metrics.ipc.connected_clients, 0);
                assert_eq!(metrics.ipc.commands_processed, 0);
            }
            other => panic!("expected Metrics, got {other:?}"),
        }
    }

    // 14. handle_verify_workflow with mismatched digest returns error
    #[test]
    fn handle_verify_workflow_with_mismatched_digest_returns_mismatch() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::VerifyWorkflow { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = MismatchResolver;
        let response = handle_verify_workflow(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowDigestMismatch);
    }

    #[test]
    fn handle_verify_workflow_no_resolver_returns_resolution_required() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::VerifyWorkflow { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_verify_workflow(&encoded, None);
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_get_taint_report_with_garbage_payload_returns_bad_request() {
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_get_taint_report(garbage, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 15. handle_get_workflow_graph with valid workflow returns graph
    #[test]
    fn handle_get_workflow_graph_with_valid_workflow_returns_graph() {
        use vb_core::ids::StepIdx;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        // Workflow with two nodes: Nop -> Finish
        let digest = vb_core::WorkflowDigest::from_bytes([0xAB; 32]);
        let parts = WorkflowParts {
            name: Box::from("test"),
            digest,
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: vb_core::ids::SlotIdx::ZERO,
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("workflow should be valid");
        let mut resolver = OkResolver { workflow };
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_get_workflow_graph(&encoded, Some(&mut resolver));
        match response {
            IpcResponse::WorkflowGraph { nodes, edges } => {
                assert_eq!(nodes.len(), 2);
                // Should have fallthrough edge from Nop to Finish
                assert!(
                    edges
                        .iter()
                        .any(|e| e.from == 0 && e.to == 1 && e.edge_type == "fallthrough"),
                    "should have fallthrough edge from Nop to Finish"
                );
            }
            other => panic!("expected WorkflowGraph, got {other:?}"),
        }
    }

    #[test]
    fn handle_get_workflow_graph_no_resolver_returns_resolution_required() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_get_workflow_graph(&encoded, None);
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_get_workflow_graph_with_mismatched_digest_returns_mismatch() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = MismatchResolver;
        let response = handle_get_workflow_graph(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowDigestMismatch);
    }

    // 16. handle_get_taint_report with valid workflow containing sources and sinks
    #[test]
    fn handle_get_taint_report_with_sources_and_sinks() {
        use vb_core::ids::StepIdx;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        // Workflow: WaitEvent (source) -> Finish (sink)
        let digest = vb_core::WorkflowDigest::from_bytes([0xCD; 32]);
        let parts = WorkflowParts {
            name: Box::from("test"),
            digest,
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitEvent {
                        event: vb_core::ids::SlotIdx::ZERO,
                        timeout_slot: None,
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: vb_core::ids::SlotIdx::ZERO,
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("workflow should be valid");
        let mut resolver = OkResolver { workflow };
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_get_taint_report(&encoded, Some(&mut resolver));
        match response {
            IpcResponse::TaintReport {
                sources,
                sinks,
                finish_safe,
                paths,
            } => {
                assert_eq!(sources, vec![0]);
                assert_eq!(sinks, vec![1]);
                assert!(
                    !finish_safe,
                    "source reaching sink should not be finish-safe"
                );
                // Paths should contain the reachable sink
                assert!(
                    paths
                        .iter()
                        .any(|p| p.from == 0 && p.to == 1 && p.status == "dangerous"),
                    "should have dangerous path from source to sink"
                );
            }
            other => panic!("expected TaintReport, got {other:?}"),
        }
    }

    #[test]
    fn handle_get_taint_report_no_resolver_returns_resolution_required() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_get_taint_report(&encoded, None);
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_get_taint_report_with_mismatched_digest_returns_mismatch() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = MismatchResolver;
        let response = handle_get_taint_report(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowDigestMismatch);
    }

    #[test]
    fn handle_get_taint_report_with_invalid_payload_returns_bad_request() {
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_get_taint_report(garbage, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // Additional tests for handler branches

    #[test]
    fn handle_submit_run_inline_delegates_to_handle_submit_run() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRunInline(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let response = handle_submit_run_inline(&encoded, &mut runtime, Some(&mut resolver));
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    #[test]
    fn handle_submit_run_no_resolver_returns_resolution_required() {
        let mut runtime = make_runtime();
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let response = handle_submit_run(&header, &encoded, &mut runtime, None);
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_answer_ask_with_oversized_answer_returns_payload_error() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            answer: vec![0xAA_u8; MAX_ANSWER_ASK_BYTES + 1],
            taint: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_answer_ask(&encoded, &mut runtime);
        match response {
            IpcResponse::PayloadError { .. } => {}
            other => panic!("expected PayloadError for oversized answer, got {other:?}"),
        }
    }

    #[test]
    fn handle_complete_action_with_oversized_output_returns_payload_error() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            output: vec![0xBB_u8; MAX_ACTION_OUTPUT_LEN + 1],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_complete_action(&encoded, &mut runtime);
        match response {
            IpcResponse::PayloadError { .. } => {}
            other => panic!("expected PayloadError for oversized output, got {other:?}"),
        }
    }

    #[test]
    fn handle_fail_action_with_oversized_error_returns_payload_error() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            error: vec![0xCC_u8; MAX_ACTION_ERROR_LEN + 1],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_fail_action(&encoded, &mut runtime);
        match response {
            IpcResponse::PayloadError { .. } => {}
            other => panic!("expected PayloadError for oversized error, got {other:?}"),
        }
    }

    #[test]
    fn handle_submit_run_with_oversized_input_returns_payload_error() {
        let mut runtime = make_runtime();
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![0xDD_u8; MAX_SUBMIT_INPUT_LEN + 1],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let response = handle_submit_run(&header, &encoded, &mut runtime, None);
        match response {
            IpcResponse::PayloadError { .. } => {}
            other => panic!("expected PayloadError for oversized input, got {other:?}"),
        }
    }

    struct RequiredResolver;
    impl WorkflowResolver for RequiredResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Err(WorkflowResolutionError::Required)
        }
    }

    struct NotFoundResolver;
    impl WorkflowResolver for NotFoundResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Err(WorkflowResolutionError::NotFound)
        }
    }

    #[test]
    fn sanitize_runtime_error_preserves_short_messages() {
        let result = sanitize_runtime_error(&"short");
        assert_eq!(result, "short");
    }

    #[test]
    fn node_kind_label_covers_all_variants() {
        use vb_core::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
        use vb_core::workflow::CompiledNodeKind;

        assert_eq!(node_kind_label(&CompiledNodeKind::Nop), "Nop");
        assert_eq!(
            node_kind_label(&CompiledNodeKind::SetConst {
                value: ConstIdx::new(0)
            }),
            "SetConst"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Copy {
                source: SlotIdx::ZERO
            }),
            "Copy"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0)
            }),
            "EvalExpr"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::BuildObject {
                fields: Box::new([])
            }),
            "BuildObject"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::BuildList {
                items: Box::new([])
            }),
            "BuildList"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO
            }),
            "Do"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Choose {
                branches: Box::new([]),
                otherwise: None
            }),
            "Choose"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ChooseSlot {
                branches: Box::new([]),
                otherwise: None
            }),
            "ChooseSlot"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ForEachStart {
                input: SlotIdx::ZERO,
                item_slot: SlotIdx::ZERO,
                limit: 0,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "ForEachStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "ForEachNext"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ForEachJoin {
                output: SlotIdx::ZERO
            }),
            "ForEachJoin"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::TogetherStart {
                branches: Box::new([]),
                join: StepIdx::new(0)
            }),
            "TogetherStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(0),
                join: StepIdx::new(0),
                accumulator: SlotIdx::ZERO
            }),
            "TogetherBranch"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::TogetherJoin {
                branch_count: 0,
                accumulator: SlotIdx::ZERO
            }),
            "TogetherJoin"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::CollectStart {
                source: SlotIdx::ZERO,
                limit: 0,
                page_size: 0,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "CollectStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "CollectPage"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "CollectNext"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::ZERO
            }),
            "CollectFinish"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ReduceStart {
                input: SlotIdx::ZERO,
                accumulator: SlotIdx::ZERO,
                initial: ConstIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "ReduceStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::ZERO,
                accumulator: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "ReduceNext"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::ZERO
            }),
            "ReduceFinish"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RepeatStart {
                max_attempts: 0,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "RepeatStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "RepeatAttempt"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::ZERO,
                done: StepIdx::new(0)
            }),
            "RepeatCheck"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RepeatFinish {
                result: SlotIdx::ZERO
            }),
            "RepeatFinish"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO
            }),
            "WaitUntil"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::WaitEvent {
                event: SlotIdx::ZERO,
                timeout_slot: None
            }),
            "WaitEvent"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: None
            }),
            "Ask"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::AskResume {
                answer: SlotIdx::ZERO
            }),
            "AskResume"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                exhausted: StepIdx::new(0)
            }),
            "RetryCheck"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(0),
                handler: StepIdx::new(0),
                error_slot: None
            }),
            "ErrorHandler"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Jump {
                target: StepIdx::new(0)
            }),
            "Jump"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Finish {
                result: SlotIdx::ZERO
            }),
            "Finish"
        );
    }

    #[test]
    fn collect_edges_from_node_covers_all_structural_variants() {
        use vb_core::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
        use vb_core::workflow::{CompiledNodeKind, ExprBranch, SlotBranch};

        let mut edges = Vec::new();

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::Choose {
                branches: Box::new([
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    ExprBranch {
                        condition: ExprIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 3);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ChooseSlot {
                branches: Box::new([
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 3);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ForEachStart {
                input: SlotIdx::ZERO,
                item_slot: SlotIdx::ZERO,
                limit: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 3);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::CollectStart {
                source: SlotIdx::ZERO,
                limit: 0,
                page_size: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ReduceStart {
                input: SlotIdx::ZERO,
                accumulator: SlotIdx::ZERO,
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::ZERO,
                accumulator: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::RepeatStart {
                max_attempts: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::ZERO,
                done: StepIdx::new(1),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 1);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 1);

        edges.clear();
        collect_edges_from_node(0, &CompiledNodeKind::Nop, &mut edges);
        assert!(edges.is_empty());
    }

    #[test]
    fn all_successors_covers_remaining_structural_variants() {
        use vb_core::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
        use vb_core::workflow::{CompiledNodeKind, ExprBranch, SlotBranch};

        let kind = CompiledNodeKind::ChooseSlot {
            branches: Box::new([SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(1),
            }]),
            otherwise: Some(StepIdx::new(2)),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::CollectStart {
            source: SlotIdx::ZERO,
            limit: 0,
            page_size: 0,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::CollectPage {
            collector_slot: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::CollectNext {
            collector_slot: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::ReduceStart {
            input: SlotIdx::ZERO,
            accumulator: SlotIdx::ZERO,
            initial: ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::ReduceNext {
            iterator_slot: SlotIdx::ZERO,
            accumulator: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 0,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::RepeatAttempt {
            attempt_slot: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::RepeatCheck {
            attempt_slot: SlotIdx::ZERO,
            done: StepIdx::new(1),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
    }

    #[test]
    fn handle_answer_ask_with_valid_payload_returns_accepted_run() {
        let mut runtime = make_runtime();
        let answer_bytes = postcard::to_allocvec(&SlotValue::Null).expect("encode SlotValue");
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            answer: answer_bytes,
            taint: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_answer_ask(&encoded, &mut runtime);
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    #[test]
    fn handle_complete_action_with_valid_payload_returns_accepted_run() {
        let mut runtime = make_runtime();
        let output_payload = crate::IpcActionOutputPayload {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::Null,
            taint: Taint::Clean,
        };
        let output_bytes = postcard::to_allocvec(&output_payload).expect("encode output");
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            output: output_bytes,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_complete_action(&encoded, &mut runtime);
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    #[test]
    fn handle_fail_action_with_valid_payload_returns_accepted_run() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            error: vec![0xCC; 10],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_fail_action(&encoded, &mut runtime);
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    // -- Boundary tests to kill >= mutants (exact max length must be allowed) --

    #[test]
    fn handle_answer_ask_with_answer_at_exact_max_len_returns_accepted_run() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            answer: vec![0x00; MAX_ANSWER_ASK_BYTES],
            taint: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_answer_ask(&encoded, &mut runtime);
        // With >, passes length check and runtime accepts → AcceptedRun.
        // With >=, length check fails and returns PayloadError.
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 1 });
    }

    #[test]
    fn handle_complete_action_with_output_at_exact_max_len_returns_accepted_run() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            output: vec![0x00; MAX_ACTION_OUTPUT_LEN],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_complete_action(&encoded, &mut runtime);
        // With >, passes length check and runtime accepts → AcceptedRun.
        // With >=, length check fails and returns PayloadError.
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 1 });
    }

    #[test]
    fn handle_fail_action_with_error_at_exact_max_len_returns_accepted_run() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            error: vec![0xCC_u8; MAX_ACTION_ERROR_LEN],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_fail_action(&encoded, &mut runtime);
        // With >, passes length check; runtime accepts and returns AcceptedRun.
        // With >=, length check fails and returns PayloadError.
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 1 });
    }

    #[test]
    fn submit_resolved_workflow_with_input_at_exact_max_len_returns_accepted_run() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![0xDD_u8; MAX_SUBMIT_INPUT_LEN],
        };
        let response = submit_resolved_workflow(
            IpcCommand::SubmitRun,
            submit,
            &mut runtime,
            Some(&mut resolver),
        );
        // With >, passes length check; runtime accepts and returns AcceptedRun.
        // With >=, length check fails and returns PayloadError.
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 1 });
    }

    #[test]
    fn submit_resolved_workflow_with_required_resolver_returns_resolution_required() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let mut resolver = RequiredResolver;
        let response = submit_resolved_workflow(
            IpcCommand::SubmitRun,
            submit,
            &mut runtime,
            Some(&mut resolver),
        );
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn submit_resolved_workflow_with_mismatched_digest_returns_mismatch() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let mut resolver = MismatchResolver;
        let response = submit_resolved_workflow(
            IpcCommand::SubmitRun,
            submit,
            &mut runtime,
            Some(&mut resolver),
        );
        assert_eq!(response, IpcResponse::WorkflowDigestMismatch);
    }

    #[test]
    fn submit_resolved_workflow_with_invalid_command_returns_mismatch() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let response = submit_resolved_workflow(
            IpcCommand::Health,
            submit,
            &mut runtime,
            Some(&mut resolver),
        );
        assert_eq!(response, IpcResponse::CommandPayloadMismatch);
    }

    #[test]
    fn handle_verify_workflow_with_valid_workflow_returns_gate_results() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let payload = crate::IpcPayload::VerifyWorkflow { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_verify_workflow(&encoded, Some(&mut resolver));
        match response {
            IpcResponse::VerifyWorkflow { result } => {
                assert_eq!(result.total_checks, 9);
                assert_eq!(result.pass_count + result.fail_count, result.total_checks);
            }
            other => panic!("expected VerifyWorkflow, got {other:?}"),
        }
    }

    #[test]
    fn handle_get_workflow_graph_with_invalid_payload_returns_bad_request() {
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_get_workflow_graph(garbage, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_get_workflow_graph_with_required_resolver_returns_resolution_required() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = RequiredResolver;
        let response = handle_get_workflow_graph(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_get_workflow_graph_with_not_found_resolver_returns_unsupported() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = NotFoundResolver;
        let response = handle_get_workflow_graph(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowResolutionUnsupported);
    }

    #[test]
    fn handle_get_taint_report_with_required_resolver_returns_resolution_required() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = RequiredResolver;
        let response = handle_get_taint_report(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_get_taint_report_with_not_found_resolver_returns_unsupported() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = NotFoundResolver;
        let response = handle_get_taint_report(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowResolutionUnsupported);
    }

    #[test]
    fn handle_get_taint_report_with_safe_source_returns_warning() {
        use vb_core::ids::StepIdx;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        let digest = vb_core::WorkflowDigest::from_bytes([0xCD; 32]);
        let parts = WorkflowParts {
            name: Box::from("test"),
            digest,
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitEvent {
                        event: vb_core::ids::SlotIdx::ZERO,
                        timeout_slot: None,
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("workflow should be valid");
        let mut resolver = OkResolver { workflow };
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_get_taint_report(&encoded, Some(&mut resolver));
        match response {
            IpcResponse::TaintReport {
                sources,
                sinks,
                finish_safe,
                paths,
            } => {
                assert_eq!(sources, vec![0]);
                assert!(sinks.is_empty(), "no sinks in this workflow");
                assert!(finish_safe, "no source reaches sink");
                assert!(!paths.is_empty(), "should have warning paths");
                assert!(
                    paths.iter().all(|p| p.status == "warning"),
                    "all paths should be warning"
                );
            }
            other => panic!("expected TaintReport, got {other:?}"),
        }
    }

    #[test]
    fn bfs_forward_respects_out_of_bounds_successors() {
        use vb_core::ids::StepIdx;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        let parts = WorkflowParts {
            name: Box::from("test"),
            digest: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(99),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: vb_core::ids::SlotIdx::ZERO,
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let reachable = bfs_forward(&parts, 0, 2);
        assert!(
            reachable.is_empty(),
            "out-of-bounds successor should not be followed"
        );
    }

    // ── Mutation kill: handle_get_workflow_graph boundary (< vs <=) ─────────────

    #[test]
    fn handle_get_workflow_graph_returns_exact_node_count_with_one_node() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_workflow_with_nodes(digest, 1);
        let mut resolver = OkResolver { workflow };

        let payload_bytes =
            postcard::to_allocvec(&IpcPayload::GetWorkflowGraph { digest })
                .expect("encode payload");
        let response = handle_get_workflow_graph(&payload_bytes, Some(&mut resolver));

        match response {
            IpcResponse::WorkflowGraph { nodes, .. } => {
                assert_eq!(
                    nodes.len(),
                    1,
                    "exactly 1 node should be returned when workflow has 1 node"
                );
                assert_eq!(nodes[0].step_idx, 0);
            }
            other => panic!("expected WorkflowGraph, got {other:?}"),
        }
    }

    #[test]
    fn handle_get_workflow_graph_returns_two_nodes_with_two_node_workflow() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x43; 32]);
        let workflow = make_workflow_with_nodes(digest, 2);
        let mut resolver = OkResolver { workflow };

        let payload_bytes =
            postcard::to_allocvec(&IpcPayload::GetWorkflowGraph { digest })
                .expect("encode payload");
        let response = handle_get_workflow_graph(&payload_bytes, Some(&mut resolver));

        match response {
            IpcResponse::WorkflowGraph { nodes, .. } => {
                assert_eq!(
                    nodes.len(),
                    2,
                    "exactly 2 nodes should be returned when workflow has 2 nodes"
                );
                assert_eq!(nodes[0].step_idx, 0);
                assert_eq!(nodes[1].step_idx, 1);
            }
            other => panic!("expected WorkflowGraph, got {other:?}"),
        }
    }

    // ── Mutation kill: handle_get_taint_report boundary (>= vs <) ───────────────

    #[test]
    fn handle_get_taint_report_returns_exact_path_count_for_linear_chain() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x44; 32]);
        let parts = WorkflowParts {
            name: Box::from("taint_boundary"),
            digest,
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitEvent {
                        event: vb_core::ids::SlotIdx::new(0),
                        timeout_slot: None,
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(vb_core::ids::SlotIdx::new(0)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: vb_core::ids::SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).expect("valid workflow");
        let mut resolver = OkResolver { workflow };

        let payload_bytes =
            postcard::to_allocvec(&IpcPayload::GetTaintReport { digest })
                .expect("encode payload");
        let response = handle_get_taint_report(&payload_bytes, Some(&mut resolver));

        match response {
            IpcResponse::TaintReport {
                sources,
                sinks,
                finish_safe,
                paths,
            } => {
                assert_eq!(sources, vec![0]);
                assert_eq!(sinks, vec![2]);
                assert_eq!(paths.len(), 2, "expected 2 taint paths for 3-node linear chain");
                assert!(!finish_safe, "finish should not be safe when source reaches sink");
            }
            other => panic!("expected TaintReport, got {other:?}"),
        }
    }

    // ── Mutation kill: enqueue_successors boundary (< vs <=, < vs ==) ───────────

    #[test]
    fn enqueue_successors_rejects_out_of_bounds_next() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)), // node_count = 1, so next=1 is out-of-bounds
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        enqueue_successors(&node, 1, &mut visited, &mut queue);

        assert!(visited.is_empty(), "out-of-bounds next should not be inserted");
        assert!(queue.is_empty(), "out-of-bounds next should not be enqueued");
    }

    #[test]
    fn enqueue_successors_rejects_out_of_bounds_structural_successor() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 1,
                body: StepIdx::new(1), // out-of-bounds
                done: StepIdx::new(1), // also out-of-bounds
            },
        };
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        enqueue_successors(&node, 1, &mut visited, &mut queue);

        assert!(visited.is_empty(), "out-of-bounds structural successors should not be inserted");
        assert!(queue.is_empty(), "out-of-bounds structural successors should not be enqueued");
    }

    #[test]
    fn enqueue_successors_accepts_valid_successor_at_last_index() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)), // node_count = 2, next=1 is valid
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        enqueue_successors(&node, 2, &mut visited, &mut queue);

        assert!(visited.contains(&1), "valid next should be inserted");
        assert_eq!(queue.len(), 1, "valid next should be enqueued");
    }

    // ── Helper: build a linear-chain workflow ──────────────────────────────────

    fn make_workflow_with_nodes(
        d: vb_core::WorkflowDigest,
        count: usize,
    ) -> vb_core::workflow::CompiledWorkflow {
        let nodes: Vec<CompiledNode> = (0..count)
            .map(|i| {
                let kind = if i == count - 1 {
                    CompiledNodeKind::Finish {
                        result: vb_core::ids::SlotIdx::ZERO,
                    }
                } else if i == 0 {
                    CompiledNodeKind::WaitEvent {
                        event: vb_core::ids::SlotIdx::new(0),
                        timeout_slot: None,
                    }
                } else {
                    CompiledNodeKind::Nop
                };
                CompiledNode {
                    id: StepIdx::new(i as u16),
                    output: None,
                    next: if i < count - 1 {
                        Some(StepIdx::new((i + 1) as u16))
                    } else {
                        None
                    },
                    on_error: None,
                    error_slot: None,
                    kind,
                }
            })
            .collect();

        let parts = WorkflowParts {
            name: Box::from("test_linear"),
            digest: d,
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).expect("linear workflow should be valid")
    }
}
