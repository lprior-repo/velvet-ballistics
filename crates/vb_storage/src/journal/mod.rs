#![forbid(unsafe_code)]
//! Fjall-backed journal implementation.

pub(crate) mod admission;
pub mod core;
pub(crate) use self::admission::verify_content_digest;
pub(crate) mod append;
pub(crate) mod batch;
pub mod incident;
pub(crate) mod injection;
pub(crate) mod internal;
pub mod parse;
pub mod readonly;
pub(crate) mod replay;
pub(crate) mod source;

#[cfg(test)]
mod journal_event_tests;
#[cfg(test)]
mod regression_tests_vb_1rqz7;
#[cfg(test)]
mod tests;

pub use self::core::{EventReplayLimit, FjallJournal};
pub use self::parse::parse_event;
pub use self::readonly::ReadOnlyJournal;
