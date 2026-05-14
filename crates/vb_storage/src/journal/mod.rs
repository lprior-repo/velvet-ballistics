#![forbid(unsafe_code)]
//! Fjall-backed journal implementation.

pub mod core;
pub(crate) mod admission;
pub(crate) use self::admission::verify_content_digest;
pub(crate) mod append;
pub(crate) mod batch;
pub(crate) mod injection;
pub(crate) mod internal;
pub(crate) mod replay;
pub(crate) mod source;

#[cfg(test)]
mod tests;

pub use self::core::FjallJournal;
