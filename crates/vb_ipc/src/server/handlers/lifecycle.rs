#![forbid(unsafe_code)]
//! Lifecycle (session) handlers: ping, health, shutdown.
//!
//! These commands do not interact with runs or actions — they probe the
//! runtime's basic availability.

use vb_runtime::runtime::Runtime;

use crate::server::IpcResponse;
use crate::server::handlers::utilities::sanitize_runtime_error;

/// Handles a ping/health request.
pub fn handle_ping() -> IpcResponse {
    IpcResponse::Healthy
}

/// Handles a health request.
pub fn handle_health() -> IpcResponse {
    handle_ping()
}

/// Handles graceful shutdown of the runtime.
pub fn handle_shutdown(runtime: &mut Runtime) -> IpcResponse {
    match runtime.shutdown_graceful() {
        Ok(()) => IpcResponse::ShuttingDown,
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
        },
    }
}
