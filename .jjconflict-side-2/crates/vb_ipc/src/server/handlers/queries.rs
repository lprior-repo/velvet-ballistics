#![forbid(unsafe_code)]
//! Run query handlers.

use vb_runtime::runtime::Runtime;

use super::{decode_payload, sanitize_runtime_error};
use crate::server::IpcResponse;
use crate::server::trace::typed_events_response;

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
        Ok(vb_runtime::shard::InspectResponse::Found(snapshot)) => IpcResponse::Inspected {
            run_id: snapshot.run.get(),
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
