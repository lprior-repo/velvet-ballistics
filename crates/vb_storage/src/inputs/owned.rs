#![forbid(unsafe_code)]
//! Complete immutable owned inputs, bound to the accepted input projection.

mod record;
mod retained;
mod submission;
mod wire;

pub use retained::RetainedRunInputs;
pub(in crate::inputs) use submission::SubmissionInputs;

use vb_core::root_bundle::OwnedRootBundleError;

/// Typed owned-input payload and ownership validation failure.
#[derive(Debug, thiserror::Error)]
pub enum OwnedInputError {
    /// The owned payload version is not supported.
    #[error("unsupported owned input version {version}")]
    UnsupportedVersion { version: u16 },
    /// Independently supplied identities do not match the record.
    #[error("owned input identity mismatch")]
    IdentityMismatch,
    /// Complete retained input projection differs from the accepted workflow.
    #[error("owned input projection mismatch")]
    ProjectionMismatch,
    /// The complete domain-separated digest does not match.
    #[error("owned input digest mismatch")]
    DigestMismatch,
    /// Decoded roots are not in canonical slot order.
    #[error("owned input roots are not canonical")]
    NonCanonicalRoots,
    /// Admission does not bind this complete owned record.
    #[error("owned input admission mismatch")]
    AdmissionMismatch,
    /// The real root-bundle validator rejected the complete owned state.
    #[error("owned input bundle rejected: {0}")]
    Bundle(#[source] OwnedRootBundleError),
}

/// Immutable, validated version-one retained owned input record.
///
/// This type intentionally has no Deserialize or mutable data access.
#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
pub struct OwnedRunInputsRecordV1 {
    data: Box<wire::Wire>,
}
