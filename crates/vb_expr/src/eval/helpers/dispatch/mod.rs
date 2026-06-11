#![forbid(unsafe_code)]
//! Helper function dispatch.
//!
//! Organized into:
//! - `bytecode`: bytecode-level helper dispatch
//! - `api`: public API helper evaluation
//! - `args`: argument count validation

mod api;
mod args;
mod bytecode;

pub use api::{eval_helper, eval_helper_with_store};
pub(crate) use bytecode::eval_helper_op_with_store;
