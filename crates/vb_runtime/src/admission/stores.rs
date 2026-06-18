#![forbid(unsafe_code)]
//! Artifact store implementations for runtime admission.

use std::sync::Arc;
use vb_core::ids::WorkflowDigest;

use super::errors::ArtifactEnvelopeError;
use super::types::REQUIRED_GATE_COUNT;

/// Trait for checking whether a compiled artifact exists in storage.
///
/// Implemented by storage backends that can verify artifact presence.
/// The shard uses this to enforce admission policy.
pub trait ArtifactStore: Send + Sync {
    /// Returns `true` if a compiled artifact with the given digest exists.
    fn compiled_ir_exists(&self, digest: WorkflowDigest) -> bool;
}

/// Shared artifact store trait object.
pub type SharedArtifactStore = Arc<dyn ArtifactStore>;

/// Shared accepted artifact store for full validation at admission gate.
pub type SharedAcceptedArtifactStore = Arc<dyn AcceptedArtifactStore>;

/// Loads and validates accepted artifacts from storage.
///
/// This trait enables the runtime admission gate to perform full artifact
/// validation — not just existence — before admitting a run.
pub trait AcceptedArtifactStore: Send + Sync {
    /// Loads and validates an accepted artifact by digest.
    ///
    /// Returns the validated artifact on success, or an error if the artifact
    /// is missing or fails semantic validation (gate count, proof flags).
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>;
}

/// Artifact store that always reports artifacts as present.
/// Used in tests and when policy is Relaxed.
#[derive(Debug, Default)]
pub struct AlwaysPresentArtifactStore;

/// Artifact store for non-durable strict admission where no accepted artifact
/// source exists.
#[derive(Debug, Default)]
pub struct MissingAcceptedArtifactStore;

impl AlwaysPresentArtifactStore {
    /// Creates a new shared always-present store (legacy artifact-only view).
    #[must_use]
    pub fn shared_artifact() -> SharedArtifactStore {
        Arc::new(Self)
    }

    /// Creates a new shared always-present store as an accepted artifact store.
    #[must_use]
    pub fn shared() -> SharedAcceptedArtifactStore {
        Arc::new(Self)
    }
}

impl ArtifactStore for AlwaysPresentArtifactStore {
    fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
        true
    }
}

impl AcceptedArtifactStore for AlwaysPresentArtifactStore {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(always_present_accepted_artifact(artifact_digest))
    }
}

fn always_present_accepted_artifact(
    artifact_digest: WorkflowDigest,
) -> vb_storage::admission::AcceptedArtifact {
    vb_storage::admission::AcceptedArtifact {
        digest: artifact_digest,
        source_digest: artifact_digest,
        policy_digest: artifact_digest,
        ir: Vec::new(),
        verification: always_present_verification_proof(artifact_digest),
        accepted_at_seq: vb_storage::types::EventSeq::new(0),
        required_capabilities: Box::new([]),
    }
}

fn always_present_verification_proof(
    artifact_digest: WorkflowDigest,
) -> vb_storage::admission::VerificationProof {
    vb_storage::admission::VerificationProof {
        digest: artifact_digest,
        gate_count: REQUIRED_GATE_COUNT,
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: Vec::new(),
    }
}

impl MissingAcceptedArtifactStore {
    /// Creates a new shared missing-artifact store.
    #[must_use]
    pub fn shared() -> SharedAcceptedArtifactStore {
        Arc::new(Self)
    }
}

impl AcceptedArtifactStore for MissingAcceptedArtifactStore {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Err(ArtifactEnvelopeError::ArtifactNotFound {
            digest: artifact_digest,
        })
    }
}

/// Artifact store backed by FjallJournal.
pub struct StorageArtifactStore {
    #[cfg(not(kani))]
    journal: Arc<vb_storage::FjallJournal>,
    #[cfg(kani)]
    compiled_ir_exists_value: bool,
}

impl StorageArtifactStore {
    /// Creates a new storage-backed artifact store.
    #[must_use]
    #[cfg(not(kani))]
    pub fn new(journal: Arc<vb_storage::FjallJournal>) -> Self {
        Self { journal }
    }

    /// Creates a Kani model of the storage-backed artifact store.
    ///
    /// Kani cannot tractably model Fjall's filesystem-backed internals in the
    /// runtime proof lane. The model preserves the wrapper contract exercised by
    /// `compiled_ir_exists`: a storage query is reduced to a bounded boolean
    /// result while production builds continue to use the real journal field.
    #[must_use]
    #[cfg(kani)]
    pub fn new(_journal: Arc<vb_storage::FjallJournal>) -> Self {
        Self {
            compiled_ir_exists_value: false,
        }
    }

    /// Creates a Kani-only artifact-existence model without constructing Fjall.
    #[must_use]
    #[cfg(kani)]
    pub fn kani_model(compiled_ir_exists_value: bool) -> Self {
        Self {
            compiled_ir_exists_value,
        }
    }

    /// Creates a new shared storage-backed artifact store (legacy artifact-only view).
    #[must_use]
    pub fn shared_artifact(journal: Arc<vb_storage::FjallJournal>) -> SharedArtifactStore {
        Arc::new(Self::new(journal))
    }

    /// Creates a new shared storage-backed accepted artifact store.
    #[must_use]
    pub fn shared(journal: Arc<vb_storage::FjallJournal>) -> SharedAcceptedArtifactStore {
        Arc::new(Self::new(journal))
    }
}

impl ArtifactStore for StorageArtifactStore {
    #[cfg(not(kani))]
    fn compiled_ir_exists(&self, digest: WorkflowDigest) -> bool {
        matches!(self.journal.compiled_ir(digest), Ok(Some(_)))
    }

    #[cfg(kani)]
    fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
        self.compiled_ir_exists_value
    }
}

impl AcceptedArtifactStore for StorageArtifactStore {
    #[cfg(not(kani))]
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        // Load the compiled IR record from the journal.
        let record = self
            .journal
            .compiled_ir(artifact_digest)
            .map_err(|_jb_err| ArtifactEnvelopeError::ArtifactNotFound {
                digest: artifact_digest,
            })?
            .ok_or(ArtifactEnvelopeError::ArtifactNotFound {
                digest: artifact_digest,
            })?;

        // Decode the postcard payload as AcceptedArtifact.
        let artifact: vb_storage::admission::AcceptedArtifact = postcard::from_bytes(&record.ir)
            .map_err(|_decode_err| ArtifactEnvelopeError::PostcardDecodeFailed)?;

        if artifact.digest != artifact_digest {
            return Err(ArtifactEnvelopeError::ArtifactDigestMismatch {
                requested: artifact_digest,
                found: artifact.digest,
            });
        }
        super::validation::validate_accepted_artifact_envelope(&artifact)?;

        Ok(artifact)
    }

    #[cfg(kani)]
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Err(ArtifactEnvelopeError::ArtifactNotFound {
            digest: artifact_digest,
        })
    }
}
