#![forbid(unsafe_code)]
//! Fjall-backed journal implementation.

pub(crate) mod admission;
pub mod core;
pub(crate) use self::admission::{verify_compiled_ir_record_digest, verify_content_digest};
pub(crate) mod append;
pub(crate) mod batch;
pub(crate) mod artifact_acceptance;
mod emergency_deny;
pub use self::emergency_deny::EmergencyDenyState;
pub(crate) use self::emergency_deny::{EMERGENCY_DENY_ACTIVE_MARKER, EMERGENCY_DENY_KEY};
pub mod incident;
pub(crate) mod injection;
pub(crate) mod internal;
pub mod parse;
pub mod readonly;
pub(crate) mod replay;
pub(crate) mod source;

#[cfg(test)]
mod emergency_deny_tests;
#[cfg(test)]
mod journal_event_tests;
#[cfg(test)]
mod regression_tests_vb_1rqz7;
#[cfg(test)]
mod regression_tests_vb_xsu4g;
#[cfg(test)]
mod tests;

pub use self::core::{EventReplayLimit, FjallJournal};
pub use self::parse::parse_event;
pub use self::readonly::ReadOnlyJournal;
