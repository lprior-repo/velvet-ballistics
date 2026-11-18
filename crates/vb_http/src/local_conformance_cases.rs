//! Closed local-only cases. Witness construction follows all cases and confirmed reap.
use super::*;
use crate::{
    DecodeError, DecodeLimits, HttpLimits, WorkflowCountAction, WorkflowObservationFailure,
};

const SCHEMA: &str = "velvet.http.local-conformance.v1";

#[derive(Debug)]
enum CaseFailure {
    Fixture(FixtureFailure),
    Check(&'static str),
}

impl From<FixtureFailure> for CaseFailure {
    fn from(value: FixtureFailure) -> Self {
        Self::Fixture(value)
    }
}

/// Evidence of this process executing the closed local profile and reaping its worker.
/// Not TLS/remote qualification, persistence, cryptographic attestation or authority.
/// No public constructor, report conversion, Deserialize, Default or Clone exists.
///
/// ```compile_fail
/// use vb_http::ExecutedLocalHttpConformance;
/// let forged = ExecutedLocalHttpConformance {
///     receipts: Vec::new(), schema: "caller report", source_fingerprint: [0; 32],
/// };
/// ```
///
/// ```compile_fail
/// let forged: vb_http::ExecutedLocalHttpConformance = Default::default();
/// ```
#[derive(Debug)]
pub struct ExecutedLocalHttpConformance {
    receipts: Vec<&'static str>,
    schema: &'static str,
    source_fingerprint: [u8; 32],
}

impl ExecutedLocalHttpConformance {
    pub fn case_receipts(&self) -> &[&'static str] {
        &self.receipts
    }
    pub const fn schema(&self) -> &'static str {
        self.schema
    }
    pub const fn source_fingerprint(&self) -> &[u8; 32] {
        &self.source_fingerprint
    }
}

/// Failures contain static case labels only, never response bodies or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalConformanceFailure {
    Case(&'static str),
    FixtureClosed(&'static str),
    CleanupPending,
}

impl std::fmt::Display for LocalConformanceFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Case(label) => write!(formatter, "local conformance case failed: {label}"),
            Self::FixtureClosed(label) => write!(
                formatter,
                "local fixture failed without pending cleanup: {label}"
            ),
            Self::CleanupPending => {
                formatter.write_str("local fixture cleanup pending; worker retained")
            }
        }
    }
}

impl std::error::Error for LocalConformanceFailure {}

impl From<CaseFailure> for LocalConformanceFailure {
    fn from(error: CaseFailure) -> Self {
        match error {
            CaseFailure::Check(label) => Self::Case(label),
            CaseFailure::Fixture(FixtureFailure::CleanupPending) => Self::CleanupPending,
            CaseFailure::Fixture(FixtureFailure::Closed(error)) => {
                Self::FixtureClosed(match error {
                    FixtureError::Io(_) => "I/O",
                    FixtureError::Endpoint(_) => "endpoint",
                    FixtureError::Deadline => "deadline",
                    FixtureError::Bounds => "bounds",
                    FixtureError::Resources => "resources",
                    FixtureError::IncompleteRequest => "incomplete request",
                    FixtureError::ExtraConnection => "extra connection",
                    FixtureError::WorkerPanic => "worker panic",
                    FixtureError::OperationPanic => "operation panic",
                })
            }
        }
    }
}

/// Explicitly execute only the fixed local-loopback profile. Opens ephemeral local
/// listeners and uses the production HTTP adapter. Never grants runtime admission.
/// CleanupPending retains worker custody; use poll_local_conformance_cleanup.
/// There is no universal bound on mutex acquisition, scheduling or OS join.
pub fn run_local_conformance() -> Result<ExecutedLocalHttpConformance, LocalConformanceFailure> {
    run_local_stage().map_err(LocalConformanceFailure::from)
}

fn require(value: bool, case: &'static str) -> Result<(), CaseFailure> {
    if value {
        Ok(())
    } else {
        Err(CaseFailure::Check(case))
    }
}

fn limits(body: usize, headers: usize) -> Result<HttpLimits, CaseFailure> {
    HttpLimits::new(body, headers, Duration::from_millis(500))
        .map_err(|_| CaseFailure::Check("built-in HTTP limits"))
}

fn decode() -> Result<DecodeLimits, CaseFailure> {
    DecodeLimits::new(1024, 8, 128, 128).map_err(|_| CaseFailure::Check("built-in decode limits"))
}

#[path = "local_conformance_checks.rs"]
mod checks;
#[path = "local_conformance_fingerprint.rs"]
mod fingerprint;
#[path = "local_conformance_suite.rs"]
mod suite;
#[cfg(test)]
use checks::PROFILE_HEADERS;
use checks::{digest_separation, profile, profile_digest_separation, timed_body_case};
use fingerprint::source_fingerprint;
#[cfg(test)]
use fingerprint::{SOURCE_FILES, fingerprint};
use suite::run_local_stage;
#[cfg(test)]
#[path = "local_conformance_cases_tests.rs"]
mod tests;
