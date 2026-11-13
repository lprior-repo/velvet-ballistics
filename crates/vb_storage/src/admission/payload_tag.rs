#![forbid(unsafe_code)]
//! Strict-admission payload tags (MASTER §77.10).

use super::AcceptedArtifact;
use crate::compat::CURRENT_IR_VERSION;

/// Payload tag for one submitted artifact, mirroring the Verus model in
/// `verification/verus/production_inner/accepted_run_atomic_admission_production.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PayloadTag {
    /// Prepared artifact passed verification at the current IR version.
    AcceptedArtifact,
    /// Non-compiled workflow parts submitted without a compiled artifact.
    /// No production input path constructs this tag today; the variant
    /// keeps the tag set exhaustive for a future parts-submission path.
    RawWorkflowParts,
    /// Prepared artifact stamped with an IR version older than
    /// [`CURRENT_IR_VERSION`].
    LegacyCompiledIr,
    /// Submission whose preparation or validation failed, so no artifact
    /// exists to classify; the typed prepare error propagates to the caller.
    Malformed,
}

/// Classifies one prepared artifact against the current IR version.
#[must_use]
pub fn classify_accepted_artifact(artifact: &AcceptedArtifact) -> PayloadTag {
    if artifact.artifact_version == CURRENT_IR_VERSION {
        PayloadTag::AcceptedArtifact
    } else {
        PayloadTag::LegacyCompiledIr
    }
}

/// Strict policy admits only accepted artifacts.
#[must_use]
pub fn is_strict_accepted_artifact_tag(tag: PayloadTag) -> bool {
    matches!(tag, PayloadTag::AcceptedArtifact)
}
