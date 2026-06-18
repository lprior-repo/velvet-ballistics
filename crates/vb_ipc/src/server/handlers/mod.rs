#![forbid(unsafe_code)]
//! IPC command handlers dispatched by the server.
//!
//! Split into focused submodules by domain responsibility:
//! - `utilities`: payload decoding, error responses, sanitisation
//! - `lifecycle`: ping, health, shutdown
//! - `runs`: submit-run, cancel-run, inspect-run, list-events
//! - `actions`: complete-action, fail-action, answer-ask
//! - `tests`: handler-specific tests

// Internal submodules.
mod actions;
mod lifecycle;
mod runs;
mod utilities;

#[cfg(test)]
mod tests;

// ── Public re-exports for server/mod.rs callers ──

pub use actions::handle_answer_ask;
pub use actions::handle_complete_action;
pub use actions::handle_fail_action;
pub use lifecycle::handle_health;
pub use lifecycle::handle_ping;
pub use lifecycle::handle_shutdown;
pub use runs::handle_cancel_run;
pub use runs::handle_inspect_run;
pub use runs::handle_list_events;

// Direct path through runs::submit to avoid `pub use runs::submit::...` being unreachable-pub.
// These are re-exported so callers (dispatch.rs, server/mod.rs, impl_.rs) can use them.
pub use runs::submit::SubmitCommand;
pub use runs::submit::handle_submit_run;
pub use runs::submit::handle_submit_run_inline;
pub use runs::submit::submit_resolved_workflow;

// Re-export utilities needed by trace/handler.rs
pub use utilities::decode_payload;
pub use utilities::sanitize_runtime_error;
