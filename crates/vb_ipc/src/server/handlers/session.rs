#![forbid(unsafe_code)]
//! Session lifecycle handlers.

use vb_runtime::runtime::Runtime;

use super::IpcResponse;
use crate::server::handlers::sanitize_runtime_error;

pub fn handle_ping() -> IpcResponse {
    IpcResponse::Healthy
}

pub fn handle_health() -> IpcResponse {
    handle_ping()
}

pub fn handle_shutdown(runtime: &mut Runtime) -> IpcResponse {
    match runtime.shutdown_graceful() {
        // The typed facade reports suspended work; this wire acknowledgement
        // means command quiescence and durable journal drain succeeded.
        Ok(_report) => IpcResponse::ShuttingDown,
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
        },
    }
}
