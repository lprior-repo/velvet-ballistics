//! `drain-trace` command handler.
//!
//! This is the imperative shell boundary: parses payload, queries the runtime,
//! filters and truncates trace events, and returns a count-only response.

use vb_runtime::runtime::Runtime;
use vb_runtime::trace::TraceEvent;

use super::super::IpcResponse;
use super::super::handlers::{decode_payload, sanitize_runtime_error};
use super::response::count_response_trace;
use crate::IpcPayload;

/// Handles the `drain-trace` IPC command.
///
/// 1. Decodes the drain-trace payload.
/// 2. Validates the target run exists via snapshot.
/// 3. Drains all trace events from the runtime.
/// 4. Filters to `run_id` and takes the first `max_records`.
/// 5. Returns a count-based response.
pub(crate) fn handle_drain_trace(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::DrainTrace {
        run_id,
        max_records,
    }) = decode_payload::<IpcPayload>(payload)
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
        Ok(_) => {
            return IpcResponse::RuntimeError {
                message: String::from("unexpected inspect response"),
            };
        }
        Err(e) => {
            return IpcResponse::RuntimeError {
                message: sanitize_runtime_error(&e),
            };
        }
    }

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
    count_response_trace(filtered.len())
}
