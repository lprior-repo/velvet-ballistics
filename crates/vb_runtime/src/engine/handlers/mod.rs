#![forbid(unsafe_code)]

//! Node execution handler modules organized by CompiledNodeKind variant.
//!
//! Each submodule maps to a specific node kind family and re-exports
//! its handler functions for consumption by the engine dispatch layer.

mod action;
mod collect;
mod core;
mod error_handler;
mod for_each;
mod reduce;
mod repeat;
mod together;
mod util;
mod wait_ask;

// Re-export all handler functions for engine dispatch
pub(crate) use action::{handle_do, handle_retry_check};
pub(crate) use collect::{
    handle_collect_finish, handle_collect_next, handle_collect_page, handle_collect_start,
};
pub(crate) use core::handle_core_step_once;
pub(crate) use error_handler::handle_error_handler;
pub(crate) use for_each::{handle_for_each_join, handle_for_each_next, handle_for_each_start};
pub(crate) use reduce::{handle_reduce_finish, handle_reduce_next, handle_reduce_start};
pub(crate) use repeat::{
    handle_repeat_attempt, handle_repeat_check, handle_repeat_finish, handle_repeat_start,
};
pub(crate) use together::{
    handle_together_branch, handle_together_join, handle_together_start,
};
pub(crate) use wait_ask::{handle_ask, handle_ask_resume, handle_wait_event, handle_wait_until};
