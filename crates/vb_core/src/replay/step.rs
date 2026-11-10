#![forbid(unsafe_code)]
//! Replay step execution.

mod collect;
mod dispatch;
mod nodes;
mod repeat;
mod types;

pub use dispatch::{replay_step, replay_step_with_collect};
pub use types::{ReplayAction, ReplayCollectStates, SuspensionKind};

#[cfg(test)]
use nodes::{replay_build_list, replay_build_object, replay_copy, replay_finish, replay_jump};
#[cfg(test)]
use types::ReplayCollectState;

#[cfg(test)]
#[path = "step_tests.rs"]
mod tests;
