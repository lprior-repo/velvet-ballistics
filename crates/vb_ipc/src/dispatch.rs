//! Command dispatch for the IPC server.

use vb_core::action::{ActionFailure, ActionFailureCode, ActionTicket};
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::AskAnswer;
use vb_runtime::shard::AskTicket;
use vb_runtime::trace::TraceEvent;

use crate::{
    IpcCommand, IpcError, IpcFrameHeader, IpcResponse, WorkflowResolutionError, WorkflowResolver,
};

use crate::session::WorkflowResolver as WorkflowResolverTrait;

/// Decodes a postcard-encoded payload and preserves the typed IPC decode error.
pub fn decode_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, IpcResponse> {
    postcard::from_bytes(payload).map_err(|_| ipc_error_response(IpcError::PayloadDecodeFailed))
}

fn ipc_error_response(error: IpcError) -> IpcResponse {
    IpcResponse::PayloadError {
        diagnostic: error.diagnostic_code().code(),
        message: error.to_string(),
    }
}

/// Dispatch a command without a workflow resolver.
#[cfg(test)]
pub fn dispatch_command(
    header: &IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
) -> IpcResponse {
    dispatch_command_with_resolver(header, payload, runtime, None)
}

/// Dispatches a command with optional workflow resolution.
pub fn dispatch_command_with_resolver(
    header: &IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    match header.command {
        IpcCommand::Health => handle_health(),
        IpcCommand::Shutdown => handle_shutdown(runtime),
        IpcCommand::SubmitRun | IpcCommand::SubmitRunInline => {
            handle_submit_run(header, payload, runtime, resolver)
        }
        IpcCommand::CancelRun => handle_cancel_run(payload, runtime),
        IpcCommand::InspectRun => handle_inspect_run(payload, runtime),
        IpcCommand::ListEvents => handle_list_events(payload, runtime),
        IpcCommand::AnswerAsk => handle_answer_ask(payload, runtime),
        IpcCommand::CompleteAction => handle_complete_action(payload, runtime),
        IpcCommand::FailAction => handle_fail_action(payload, runtime),
        IpcCommand::DrainTrace => handle_drain_trace(payload, runtime),
    }
}

/// Handles a health request.
pub fn handle_health() -> IpcResponse {
    IpcResponse::Healthy
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
    header: &IpcFrameHeader,
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

fn step_from_ticket(ticket: u64) -> Option<StepIdx> {
    match u16::try_from(ticket) {
        Ok(step) => Some(StepIdx::new(step)),
        Err(_) => None,
    }
}

fn action_ticket_from_wire(run_id: vb_core::RunId, ticket: u64) -> Option<ActionTicket> {
    let step = step_from_ticket(ticket)?;
    Some(ActionTicket {
        run: run_id,
        step,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
    })
}

fn payload_len(len: usize) -> u32 {
    u32::try_from(len).map_or(u32::MAX, |value| value)
}

/// Handles drain-trace.
pub fn handle_drain_trace(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::DrainTrace {
        run_id,
        max_records,
    }) = decode_payload::<crate::IpcPayload>(payload)
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
    count_response(filtered.len(), IpcResponseKind::Trace)
}

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

fn typed_events_response(events: &[TraceEvent], from_sequence: u64) -> IpcResponse {
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
            typed_events.push(crate::IpcTraceEvent {
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

fn trace_event_kind(event: &TraceEvent) -> crate::IpcTraceEventKind {
    match event {
        TraceEvent::StepStarted { run, step } => crate::IpcTraceEventKind::StepStarted {
            run: *run,
            step: *step,
        },
        TraceEvent::StepEnded { run, step } => crate::IpcTraceEventKind::StepEnded {
            run: *run,
            step: *step,
        },
        TraceEvent::SlotWritten { run, slot } => crate::IpcTraceEventKind::SlotWritten {
            run: *run,
            slot: *slot,
        },
        TraceEvent::ActionScheduled { run, step } => crate::IpcTraceEventKind::ActionScheduled {
            run: *run,
            step: *step,
        },
        TraceEvent::ActionCompleted { run, step } => crate::IpcTraceEventKind::ActionCompleted {
            run: *run,
            step: *step,
        },
        TraceEvent::ActionFailed { run, step, code } => crate::IpcTraceEventKind::ActionFailed {
            run: *run,
            step: *step,
            code: *code,
        },
        TraceEvent::AskAnswered { run, step, slot } => crate::IpcTraceEventKind::AskAnswered {
            run: *run,
            step: *step,
            slot: *slot,
        },
        TraceEvent::RunSubmitted { run } => crate::IpcTraceEventKind::RunSubmitted { run: *run },
        TraceEvent::RunFinished { run } => crate::IpcTraceEventKind::RunFinished { run: *run },
        TraceEvent::RunFailed { run } => crate::IpcTraceEventKind::RunFailed { run: *run },
        TraceEvent::RunCancelled { run } => crate::IpcTraceEventKind::RunCancelled { run: *run },
    }
}

fn borrow_workflow_resolver<'a>(
    resolver: &'a mut Option<&mut dyn WorkflowResolver>,
) -> Option<&'a mut dyn WorkflowResolver> {
    match resolver {
        Some(inner) => Some(&mut **inner),
        None => None,
    }
}

fn submit_resolved_workflow(
    command: IpcCommand,
    submit: crate::SubmitRunPayload,
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
