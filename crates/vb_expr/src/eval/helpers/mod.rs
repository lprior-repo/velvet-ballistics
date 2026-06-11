#![forbid(unsafe_code)]
//! Helper function dispatch and implementations.
//!
//! Organized into:
//! - `dispatch`: helper operation dispatch from bytecode and public API
//! - `impls`: store-aware helper implementations

mod dispatch;
mod impls;

pub(crate) use dispatch::eval_helper_op_with_store;
pub use dispatch::{eval_helper, eval_helper_with_store};
