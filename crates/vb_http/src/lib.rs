#![forbid(unsafe_code)]
//! Bounded cold HTTP observations. This crate does not grant authority or commit receipts.

mod actions;
pub mod authorization_policy;
mod decode;
mod error;
mod github;
pub mod hurl;
mod hurl_predicate_transport;
mod hurl_transport;
mod limits;
mod observation;
mod request;
pub mod request_profile;
mod transport;
mod workflows;

pub use actions::{RepositoryRef, WorkflowCountAction, WorkflowObservationFailure};
pub use decode::{DecodeError, DecodeLimits};
pub use error::{AttemptFailure, RequestRejection};
pub use hurl_predicate_transport::{
    HurlPredicateAttemptFailure, execute_hurl_with_header_predicates,
};
pub use hurl_transport::{HurlAttemptFailure, execute_hurl, execute_hurl_with_headers};
pub use limits::HttpLimits;
pub use observation::{
    HttpObservation, HttpObservationWithHeaders, ObservedResponseHeader, ResponseStatus,
};
pub use request::{JsonMediaType, PinnedGet};
pub use transport::{get_once, get_once_with_headers};
pub use workflows::{WorkflowCount, decode_workflow_count_body};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Observe one public workflow count; the caller owns authorization and durable intent.
pub fn observe_workflow_count(
    action: &WorkflowCountAction,
) -> std::result::Result<WorkflowCount, WorkflowObservationFailure> {
    let started = std::time::Instant::now();
    let observed = action.observe()?;
    if started.elapsed() > action.deadline() {
        return Err(AttemptFailure::Timeout.into());
    }
    Ok(observed)
}

#[cfg(test)]
mod workflow_tests;

#[cfg(test)]
mod test_support;

// Explicit local-only conformance; never enabled by default or invoked automatically.
#[cfg(feature = "local-conformance")]
mod local_conformance;

/// Explicit local conformance exports, available with `local-conformance` enabled.
/// These external API bindings compile without invoking the suite or opening sockets.
///
/// ```
/// use vb_http::{
///     ExecutedLocalHttpConformance, LocalCleanupState, LocalConformanceFailure,
///     poll_local_conformance_cleanup, run_local_conformance,
/// };
///
/// let run: fn() -> Result<ExecutedLocalHttpConformance, LocalConformanceFailure> =
///     run_local_conformance;
/// let poll: fn() -> LocalCleanupState = poll_local_conformance_cleanup;
/// let _bindings = (run, poll);
/// ```
#[cfg(feature = "local-conformance")]
pub use local_conformance::{
    ExecutedLocalHttpConformance, LocalCleanupState, LocalConformanceFailure,
    poll_local_conformance_cleanup, run_local_conformance,
};
