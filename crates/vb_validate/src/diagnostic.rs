#![forbid(unsafe_code)]
//! Public diagnostic API for vb_validate.
//!
//! Re-exports `diagnostic_from_error` and `error_code` from the internal
//! `diag_render` module so they are accessible as `vb_validate::diagnostic::*`.

#![allow(unreachable_pub)]
pub use super::diag_render::diagnostic_from_error;
pub use super::diag_render::error_code;
