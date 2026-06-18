#![forbid(unsafe_code)]
//! Shared IPC utilities for the handlers submodule.
//!
//! Provides payload decoding, error response construction, and runtime
//! error sanitisation — helpers used by every handler module.

use crate::server::IpcResponse;

/// Decodes a postcard-encoded payload and preserves the typed IPC decode error.
pub fn decode_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, IpcResponse> {
    postcard::from_bytes(payload)
        .map_err(|_| ipc_error_response(crate::IpcError::PayloadDecodeFailed))
}

/// Builds a typed IPC payload-error response from an `IpcError`.
fn ipc_error_response(error: crate::IpcError) -> IpcResponse {
    IpcResponse::PayloadError {
        diagnostic: error.diagnostic_code().code(),
        message: error.to_string(),
    }
}

/// Maximum length for a sanitized runtime error message returned to IPC clients.
const MAX_RUNTIME_ERROR_LEN: usize = 256;

/// Sanitizes a runtime error message before returning it to an IPC client.
///
/// Truncates the message to a fixed maximum length to prevent accidental
/// leakage of large internal diagnostics over the IPC channel.  The truncation
/// preserves the first `MAX_RUNTIME_ERROR_LEN` characters and appends an
/// ellipsis indicator when the original message was longer.
pub fn sanitize_runtime_error(e: &dyn std::fmt::Display) -> String {
    let full = e.to_string();
    if full.len() <= MAX_RUNTIME_ERROR_LEN {
        return full;
    }
    let mut truncated: String = full.chars().take(MAX_RUNTIME_ERROR_LEN).collect();
    truncated.push_str("...");
    truncated
}
