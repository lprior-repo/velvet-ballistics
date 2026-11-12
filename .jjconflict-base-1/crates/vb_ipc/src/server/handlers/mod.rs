#![forbid(unsafe_code)]
//! IPC command handlers dispatched by the server.

#![allow(unused_imports)]

pub mod command;
pub mod queries;
pub mod runs;
pub mod session;

pub use command::{handle_answer_ask, handle_complete_action, handle_fail_action};
pub use queries::{handle_cancel_run, handle_inspect_run, handle_list_events};
pub use runs::{handle_submit_run, handle_submit_run_inline};
pub use session::{handle_health, handle_ping, handle_shutdown};

use crate::server::IpcResponse;

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
