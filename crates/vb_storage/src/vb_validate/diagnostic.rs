#![forbid(unsafe_code)]
//! Public diagnostic API for vb_validate.
//!
//! Re-exports `diagnostic_from_error` and `error_code` from the internal
//! `diag_render` module so they are accessible as `vb_validate::diagnostic::*`.

#![allow(unreachable_pub)]
pub use crate::vb_validate::diag::diag_render::diagnostic_from_error;
pub use crate::vb_validate::diag::diag_render::error_code;
