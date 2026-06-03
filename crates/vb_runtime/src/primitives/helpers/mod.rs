#![forbid(unsafe_code)]
//! Shared helper functions for primitive handlers.

mod jump;
mod list;
mod output;

pub(crate) use jump::{jump_to, jump_to_body, jump_to_next};
pub(crate) use list::{empty_list, expect_list, tail_items};
pub(crate) use output::require_output;
pub type RunFrame = vb_core::frame::RunFrame;

#[cfg(test)]
mod tests;
