#![forbid(unsafe_code)]

//! Node execution dispatch. Maps each `CompiledNodeKind` variant to its
//! dedicated per-kind handler.
//!
//! Split into focused submodules:
//! - `budget`: retry-attempt counter and overflow handling.
//! - `dispatch`: the main `execute_node_full` dispatcher.
//! - `handlers`: shared attempt-slot reader for retry budget.
//! - `handlers_compound`: for_each / together / collect / reduce /
//!   repeat handler helpers.
//! - `handlers_suspend`: wait / ask / do / error-handler handlers.
//! - `signals`: `EngineSignal`-fallback conversion.

mod budget;
mod dispatch;
mod handlers;
mod handlers_compound;
mod handlers_suspend;
mod signals;

pub use dispatch::execute_node_full;
