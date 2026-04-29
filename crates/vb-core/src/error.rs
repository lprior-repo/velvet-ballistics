#![forbid(unsafe_code)]

//! Backward-compatible error module. Prefer [`crate::errors`].

pub use crate::errors::{CoreError, CoreResult, EngineError};
