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
    EdgeType, IpcActionOutputPayload, IpcCommand, IpcPayload, RunListState, RunSummary,
    RuntimeMetrics, SubmitRunPayload, TaintPathStatus,
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
        // Handle unknown future InspectResponse variants conservatively.
        #[allow(unreachable_code)]
        Ok(_) => IpcResponse::RuntimeError {
            message: String::from("unknown inspect response variant"),
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
    let gate_results: Vec<(crate::GateKind, Result<(), vb_validate::ValidationError>)> = vec![
        (
            crate::GateKind::Gate07ExpressionStackDepth,
            vb_validate::gates::validate_gate_07_expression_stack_depth(&parts),
        ),
        (
            crate::GateKind::Gate08AccessorPathSegments,
            vb_validate::gates::validate_gate_08_accessor_path_segments(&parts),
        ),
        (
            crate::GateKind::Gate09SlotReferences,
            vb_validate::gates::validate_gate_09_slot_references(&parts),
        ),
        (
            crate::GateKind::Gate10NodeKindSpecific,
            vb_validate::gates::validate_gate_10_node_kind_specific(&parts),
        ),
        (
            crate::GateKind::Gate11LoopBodyGraph,
            vb_validate::gates::validate_gate_11_loop_body_graph(&parts),
        ),
        (
            crate::GateKind::Gate12ActionContractCompleteness,
            vb_validate::gates::validate_gate_12_action_contract_completeness(&parts, &[]),
        ),
        (
            crate::GateKind::Gate13NoSlotCycles,
            vb_validate::gates::validate_gate_13_no_slot_cycles(&parts),
        ),
        (
            crate::GateKind::Gate14SlotTypeConsistency,
            vb_validate::gates::validate_gate_14_slot_type_consistency(&parts),
        ),
        (
            crate::GateKind::Gate15DeterminismProof,
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
                    kind,
                    status: crate::PassFail::Pass,
                    details: String::new(),
                });
            }
            Err(err) => {
                fail_count = fail_count.saturating_add(1);
                certificates.push(crate::CertificateWire {
                    kind,
                    status: crate::PassFail::Fail,
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
        // Handle unknown future CompiledNodeKind variants conservatively.
        #[allow(unreachable_code)]
        _ => "Unknown",
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
                    edge_type: crate::EdgeType::Branch,
                });
            }
            if let Some(fallback) = otherwise {
                edges.push(crate::EdgeDescriptor {
                    from: step,
                    to: fallback.get(),
                    label: Some(String::from("otherwise")),
                    edge_type: crate::EdgeType::Branch,
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
                    edge_type: crate::EdgeType::Branch,
                });
            }
            if let Some(fallback) = otherwise {
                edges.push(crate::EdgeDescriptor {
                    from: step,
                    to: fallback.get(),
                    label: Some(String::from("otherwise")),
                    edge_type: crate::EdgeType::Branch,
                });
            }
        }
        vb_core::workflow::CompiledNodeKind::ForEachStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ForEachNext { body, done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: crate::EdgeType::LoopBody,
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: crate::EdgeType::LoopExit,
            });
        }
        vb_core::workflow::CompiledNodeKind::TogetherStart { branches, join, .. } => {
            for (i, branch_step) in branches.iter().enumerate() {
                edges.push(crate::EdgeDescriptor {
                    from: step,
                    to: branch_step.get(),
                    label: Some(format!("branch_{i}")),
                    edge_type: crate::EdgeType::ParallelBranch,
                });
            }
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: join.get(),
                label: Some(String::from("join")),
                edge_type: crate::EdgeType::ParallelJoin,
            });
        }
        vb_core::workflow::CompiledNodeKind::CollectStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectPage { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectNext { body, done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: crate::EdgeType::LoopBody,
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: crate::EdgeType::LoopExit,
            });
        }
        vb_core::workflow::CompiledNodeKind::ReduceStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ReduceNext { body, done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: crate::EdgeType::LoopBody,
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: crate::EdgeType::LoopExit,
            });
        }
        vb_core::workflow::CompiledNodeKind::RepeatStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: crate::EdgeType::LoopBody,
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: crate::EdgeType::LoopExit,
            });
        }
        vb_core::workflow::CompiledNodeKind::RepeatCheck { done, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: crate::EdgeType::LoopExit,
            });
        }
        vb_core::workflow::CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: crate::EdgeType::Fallthrough,
            });
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: handler.get(),
                label: Some(String::from("handler")),
                edge_type: crate::EdgeType::ErrorHandler,
            });
        }
        vb_core::workflow::CompiledNodeKind::Jump { target } => {
            edges.push(crate::EdgeDescriptor {
                from: step,
                to: target.get(),
                label: None,
                edge_type: crate::EdgeType::Jump,
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
        // Handle unknown future CompiledNodeKind variants: no edges to report.
        #[allow(unreachable_code)]
        _ => {}
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
                edge_type: crate::EdgeType::Fallthrough,
            });
        }

        // Add structural edges from the node kind.
        collect_edges_from_node(idx, &compiled_node.kind, &mut edges);

        nodes.push(crate::NodeDescriptor {
            step_idx: idx,
            kind: crate::NodeKind::from(node_kind_label(&compiled_node.kind)),
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

        let status = if reaches_sink {
            crate::TaintPathStatus::Dangerous
        } else {
            crate::TaintPathStatus::Warning
        };

        for step in &reachable {
            if paths.len() >= MAX_TAINT_PATH_ENTRIES {
                break;
            }
            paths.push(crate::TaintPathWire {
                from: *source_idx,
                to: *step,
                status,
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
        // Handle unknown future CompiledNodeKind variants: no additional successors.
        #[allow(unreachable_code)]
        _ => {}
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
#[path = "handlers/tests.rs"]
mod tests;
