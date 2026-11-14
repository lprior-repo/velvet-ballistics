#![forbid(unsafe_code)]
//! Replay step execution.

mod collect;
mod concurrent;
mod concurrent_trace;
mod concurrent_validation;
mod control;
mod dispatch;
mod for_each;
mod iter;
mod nodes;
mod reduce;
mod repeat;
mod together;
mod types;

pub use concurrent_trace::{
    finish_concurrent_trace_step, prepare_concurrent_trace_start, replay_concurrent_trace_step,
};
pub use dispatch::{replay_step, replay_step_with_collect};
pub use types::{ReplayAction, ReplayCollectStateSnapshot, ReplayCollectStates, SuspensionKind};

#[cfg(test)]
use nodes::{replay_build_list, replay_build_object, replay_copy, replay_finish, replay_jump};
#[cfg(test)]
use types::ReplayCollectState;

#[cfg(test)]
#[path = "step_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "step/primitive_tests.rs"]
mod primitive_tests;

#[cfg(test)]
#[path = "step/primitive_control_tests.rs"]
mod primitive_control_tests;

#[cfg(test)]
#[path = "step/compound_parity_tests.rs"]
mod compound_parity_tests;
