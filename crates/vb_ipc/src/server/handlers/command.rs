#![forbid(unsafe_code)]
//! Command handlers that advance run state.

use vb_core::action::{ActionFailure, ActionFailureCode, RetryPolicy};
use vb_core::ids::SlotIdx;
use vb_core::value::Taint;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket};

use super::query::{decode_payload, sanitize_runtime_error};
use crate::server::ticket::{action_ticket_from_wire, payload_len, step_from_ticket};
use crate::IpcPayload;

const MAX_ANSWER_ASK_BYTES: usize = 65536;
const MAX_ACTION_OUTPUT_LEN: usize = 65536;
const MAX_ACTION_ERROR_LEN: usize = 65536;

pub fn handle_answer_ask(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::AnswerAsk {
        run_id,
        ticket,
        answer,
        taint,
    }) = decode_payload::<IpcPayload>(payload)
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
            return IpcResponse::RuntimeError {
                message: String::from("answer payload size exceeds u32::MAX"),
            };
        }
    };
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

pub fn handle_complete_action(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::CompleteAction {
        run_id,
        ticket,
        output,
    }) = decode_payload::<IpcPayload>(payload)
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

pub fn handle_fail_action(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::FailAction {
        run_id,
        ticket,
        error,
    }) = decode_payload::<IpcPayload>(payload)
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
