#![forbid(unsafe_code)]
//! Command handlers that advance run state.

use vb_core::action::{ActionFailure, ActionFailureCode, RetryPolicy};
use vb_core::value::Taint;
use vb_runtime::runtime::Runtime;

use super::{MAX_ACTION_ERROR_LEN, MAX_ACTION_OUTPUT_LEN, decode_payload, sanitize_runtime_error};
use crate::server::IpcResponse;
use crate::server::ticket::{payload_len, step_from_ticket};

pub use super::answer::handle_answer_ask;

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
    let decoded_output = match decode_payload::<crate::IpcActionOutputPayload>(&output) {
        Ok(d) => d,
        Err(response) => return response,
    };
    let Some(output_len) = postcard::serialize_with_flavor(
        &decoded_output.value,
        postcard::ser_flavors::Size::default(),
    )
    .ok()
    .and_then(|len| u32::try_from(len).ok()) else {
        return IpcResponse::PayloadError {
            diagnostic: crate::IpcError::PayloadEncodeFailed
                .diagnostic_code()
                .code(),
            message: crate::IpcError::PayloadEncodeFailed.to_string(),
        };
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

/// Handles a bound manual-gate decision submitted over IPC.
pub fn handle_submit_bound_gate_decision(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::SubmitBoundGateDecision { run_id, input }) =
        decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };
    match runtime.submit_bound_gate_decision_from_local_socket(run_id, input) {
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
