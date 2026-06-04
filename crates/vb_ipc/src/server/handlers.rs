#![forbid(unsafe_code)]
//! IPC command handlers dispatched by the server.

#![allow(unused_imports)]

use vb_core::action::{ActionFailure, ActionFailureCode, RetryPolicy};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;
use vb_runtime::trace::TraceEvent;

use super::trace::typed_events_response;
use crate::server::ticket::{action_ticket_from_wire, payload_len};
use crate::server::{IpcResponse, WorkflowResolutionError, WorkflowResolver};
use crate::{IpcActionOutputPayload, IpcCommand, IpcPayload, SubmitRunPayload};

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

/// Maximum allowed size for the `AnswerAsk.answer` payload bytes.
/// Prevents unbounded deserialization of unused answer data.
const MAX_ANSWER_ASK_BYTES: usize = 65536;

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
        answer_slot,
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
    let answer_taint = match taint {
        Some(value) => value,
        None => Taint::Clean,
    };
    match runtime.answer_pending_ask_slot(run_id, answer_slot, value, answer_taint, encoded_len) {
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

#[cfg(test)]
mod answer_ask_runtime_semantics_tests {
    use super::*;
    use std::num::NonZeroUsize;
    use std::sync::Arc;
    use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::{ConstValue, SlotValue, Taint};
    use vb_core::workflow::{CompiledNode, ResourceContract, WorkflowParts};
    use vb_runtime::RuntimeError;
    use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
    use vb_runtime::shard::ShardConfig;

    fn runtime_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    fn ask_then_finish_workflow() -> Option<CompiledWorkflow> {
        let set_prompt = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: vb_core::CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let ask = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: vb_core::CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: Some(SlotIdx::ZERO),
            },
        };
        let resume = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: vb_core::CompiledNodeKind::AskResume {
                answer: SlotIdx::new(1),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("ipc_ask_then_finish"),
            digest: WorkflowDigest::from_bytes([31; 32]),
            nodes: Box::from([set_prompt, ask, resume, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(10)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn answer_payload(
        run_id: RunId,
        answer_slot: SlotIdx,
        value_bytes: Vec<u8>,
        taint: Option<Taint>,
    ) -> Option<Vec<u8>> {
        postcard::to_allocvec(&crate::IpcPayload::AnswerAsk {
            run_id,
            answer_slot,
            answer: value_bytes,
            taint,
        })
        .ok()
    }

    fn encoded_value(value: &SlotValue) -> Option<Vec<u8>> {
        postcard::to_allocvec(value).ok()
    }

    fn must_encoded_value(value: &SlotValue) -> Vec<u8> {
        match encoded_value(value) {
            Some(bytes) => bytes,
            None => panic!("test setup failed: SlotValue {value:?} must postcard encode"),
        }
    }

    fn must_answer_payload(
        run_id: RunId,
        answer_slot: SlotIdx,
        value_bytes: Vec<u8>,
        taint: Option<Taint>,
    ) -> Vec<u8> {
        match answer_payload(run_id, answer_slot, value_bytes, taint) {
            Some(payload) => payload,
            None => panic!("test setup failed: AnswerAsk IPC payload must postcard encode"),
        }
    }

    fn runtime_with_pending_ask(
        run_id: RunId,
        journal: Arc<VolatileRuntimeJournal>,
    ) -> Option<Runtime> {
        let shard_count = NonZeroUsize::new(1)?;
        let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), journal);
        let workflow = ask_then_finish_workflow()?;
        if runtime.submit_compiled(run_id, workflow) != Ok(()) {
            return None;
        }
        if runtime.tick_all() != Ok(true) {
            return None;
        }
        Some(runtime)
    }

    #[test]
    fn handle_answer_ask_accepts_valid_postcard_slot_value_and_default_clean_taint() {
        let run_id = RunId::new(3101);
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = match runtime_with_pending_ask(run_id, journal.clone()) {
            Some(runtime) => runtime,
            None => panic!("test setup failed: runtime must reach pending ask state"),
        };
        let expected_answer = must_encoded_value(&SlotValue::I64(42));
        let payload = must_answer_payload(run_id, SlotIdx::new(1), expected_answer.clone(), None);

        assert_eq!(
            handle_answer_ask(&payload, &mut runtime),
            IpcResponse::AcceptedRun { run_id: 3101 }
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);

        let snapshot = journal.snapshot();
        match snapshot {
            Ok(events) => {
                let matched = events.iter().any(|event| {
                    matches!(
                        event,
                        RuntimeJournalEvent::SlotWritten { run, slot, value, taint, .. }
                            if *run == run_id
                                && *slot == SlotIdx::new(1)
                                && *value == expected_answer
                                && *taint == Taint::Clean
                    )
                });
                assert_eq!(
                    matched, true,
                    "journal must contain exact SlotValue::I64(42) postcard bytes for answered slot"
                );
            }
            Err(e) => panic!("journal snapshot failed: {e}"),
        }
    }

    #[test]
    fn handle_answer_ask_rejects_mismatched_answer_slot_without_consuming_pending_ask() {
        let run_id = RunId::new(3102);
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = match runtime_with_pending_ask(run_id, journal.clone()) {
            Some(runtime) => runtime,
            None => panic!("test setup failed: runtime must reach pending ask state"),
        };
        let wrong_answer = must_encoded_value(&SlotValue::I64(7));
        let wrong_payload = must_answer_payload(run_id, SlotIdx::ZERO, wrong_answer, None);

        match handle_answer_ask(&wrong_payload, &mut runtime) {
            IpcResponse::RuntimeError { message } => {
                assert_eq!(message, RuntimeError::InvalidActionCompletion.to_string());
            }
            other => panic!("expected RuntimeError, got {other:?}"),
        }
        assert_eq!(runtime.counters_snapshot().runs_completed, 0);

        let valid_answer = must_encoded_value(&SlotValue::I64(8));
        let valid_payload =
            must_answer_payload(run_id, SlotIdx::new(1), valid_answer.clone(), None);
        assert_eq!(
            handle_answer_ask(&valid_payload, &mut runtime),
            IpcResponse::AcceptedRun { run_id: 3102 }
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
        let events = match journal.snapshot() {
            Ok(events) => events,
            Err(e) => panic!("journal snapshot failed after valid answer: {e}"),
        };
        let matched = events.iter().any(|event| {
            matches!(
                event,
                RuntimeJournalEvent::SlotWritten { run, slot, value, taint, .. }
                    if *run == run_id
                        && *slot == SlotIdx::new(1)
                        && *value == valid_answer
                        && *taint == Taint::Clean
            )
        });
        assert_eq!(
            matched, true,
            "valid retry must write exact SlotValue::I64(8) postcard bytes after wrong slot rejection"
        );
    }

    #[test]
    fn handle_answer_ask_rejects_absent_pending_ask() {
        let shard_count = match NonZeroUsize::new(1) {
            Some(shard_count) => shard_count,
            None => panic!("test setup failed: shard_count must be non-zero"),
        };
        let mut runtime = Runtime::new(shard_count, runtime_config());
        let run_id = RunId::new(3103);
        let answer = must_encoded_value(&SlotValue::Bool(true));
        let payload = must_answer_payload(run_id, SlotIdx::new(1), answer, None);

        match handle_answer_ask(&payload, &mut runtime) {
            IpcResponse::RuntimeError { message } => {
                assert_eq!(message, RuntimeError::RunNotFound.to_string());
            }
            other => panic!("expected RuntimeError, got {other:?}"),
        }
    }

    #[test]
    fn handle_answer_ask_rejects_malformed_slot_value_bytes_before_runtime_mutation() {
        let run_id = RunId::new(3104);
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = match runtime_with_pending_ask(run_id, journal.clone()) {
            Some(runtime) => runtime,
            None => panic!("test setup failed: runtime must reach pending ask state"),
        };
        let malformed_payload = must_answer_payload(run_id, SlotIdx::new(1), vec![255, 255], None);

        match handle_answer_ask(&malformed_payload, &mut runtime) {
            IpcResponse::RuntimeError { message } => {
                assert_eq!(
                    message,
                    "answer bytes are not valid postcard-encoded SlotValue"
                );
            }
            other => panic!("expected RuntimeError, got {other:?}"),
        }
        assert_eq!(runtime.counters_snapshot().runs_completed, 0);

        let valid_answer = must_encoded_value(&SlotValue::Bool(false));
        let valid_payload =
            must_answer_payload(run_id, SlotIdx::new(1), valid_answer.clone(), None);
        assert_eq!(
            handle_answer_ask(&valid_payload, &mut runtime),
            IpcResponse::AcceptedRun { run_id: 3104 }
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
        let events = match journal.snapshot() {
            Ok(events) => events,
            Err(e) => panic!("journal snapshot failed after malformed rejection recovery: {e}"),
        };
        let matched = events.iter().any(|event| {
            matches!(
                event,
                RuntimeJournalEvent::SlotWritten { run, slot, value, taint, .. }
                    if *run == run_id
                        && *slot == SlotIdx::new(1)
                        && *value == valid_answer
                        && *taint == Taint::Clean
            )
        });
        assert_eq!(
            matched, true,
            "pending ask must remain consumable and write exact SlotValue::Bool(false) bytes after malformed rejection"
        );
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
