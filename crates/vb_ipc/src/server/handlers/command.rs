#![forbid(unsafe_code)]
//! Command handlers that advance run state.

use vb_core::action::{ActionFailure, ActionFailureCode, RetryPolicy};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::AskAnswer;

use super::{
    MAX_ACTION_ERROR_LEN, MAX_ACTION_OUTPUT_LEN, MAX_ANSWER_ASK_BYTES, decode_payload,
    sanitize_runtime_error,
};
use crate::server::IpcResponse;
use crate::server::ticket::{payload_len, step_from_ticket};

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
    let Some(ticket) = runtime.lookup_pending_ask_ticket(run_id, ask_step) else {
        return IpcResponse::BadRequest;
    };
    let answer = AskAnswer {
        answer_slot: ticket.answer_slot(),
        ticket,
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

    let Some(action_ticket) = runtime.lookup_pending_action_ticket(run_id, ticket) else {
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

    let Some(action_ticket) = runtime.lookup_pending_action_ticket(run_id, ticket) else {
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
