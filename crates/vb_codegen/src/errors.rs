//! Shared error types for vb_codegen.

use std::fmt;

/// Error type for codegen operations.
#[derive(Debug)]
pub struct FmtError;

impl fmt::Display for FmtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "format error")
    }
}

impl std::error::Error for FmtError {}

/// Converts a fmt::Error to CodegenError::FormatBufferOverflow.
#[inline]
pub fn fmt_err(_: std::fmt::Error) -> crate::CodegenError {
    crate::CodegenError::FormatBufferOverflow
}
