#![forbid(unsafe_code)]
//! Untrusted action-pack declarations; execution authority lives at the storage boundary.

use crate::{ActionId, WorkflowDigest};
use serde::{Deserialize, Serialize};

/// Independent idempotency classification from master77.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdempotencyClass {
    /// No external effect.
    PureDeterministic,
    /// A key is declared without an enforcement guarantee.
    KeyRequiredDeclared,
    /// Registered executor actually enforces duplicate keys.
    KeyEnforcedByExecutor,
    /// Provider guarantee, unsupported by the local register.
    ProviderEnforced,
    /// Observation, without a general guarantee.
    ObservedIdempotent,
    /// No classification established.
    Unknown,
}

/// Recovery is independent of idempotency; writes default to blocked.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryClass {
    /// Pure action.
    NoRecoveryNeeded,
    /// Retry with the identical logical key.
    RetrySameKey,
    /// Read an existing durable receipt without a production effect.
    ReconcileReceipt,
    /// Requires a separately implemented compensation protocol.
    Compensate,
    /// Requires a separately implemented operator gate.
    ManualGate,
    /// No automatic recovery authority.
    #[default]
    Blocked,
}

/// Registered implementation selector; strings never select runtime code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutorIdentity {
    /// Desired-state I64 register with atomic Fjall target/receipt commit.
    FjallRegisterV1,
}

macro_rules! digest_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name(WorkflowDigest);
        impl $name {
            /// Wrap untrusted digest bytes; this does not establish authority.
            pub const fn from_digest(digest: WorkflowDigest) -> Self {
                Self(digest)
            }
            /// Return the role-specific digest bytes.
            pub const fn digest(self) -> WorkflowDigest {
                self.0
            }
        }
    };
}
digest_type!(
    ExecutorVersionDigest,
    "Fingerprint of registered implementation sources."
);
digest_type!(
    ManifestDigest,
    "Digest of the exact action ABI declaration."
);
digest_type!(
    ReceiptSchemaDigest,
    "Fingerprint of the receipt wire schema."
);
digest_type!(
    ActionPackDigest,
    "Digest of an exact action-pack declaration."
);
digest_type!(InputDigest, "Digest of canonical executor input bytes.");
digest_type!(ReceiptDigest, "Digest of a complete durable receipt.");
digest_type!(
    ConformanceReportDigest,
    "Digest of executed observations, not authority by itself."
);

/// Exact numeric target, with no wildcard or prefix expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityScopeSpec {
    /// Local register namespace.
    pub namespace: u64,
    /// Exact key within the namespace.
    pub key: u64,
}

/// Synchronous adapter never retries automatically after an uncertain failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    /// Deadline checked before starting I/O; late acknowledgement requires reconciliation.
    /// Fjall fsync is synchronous and does not promise a hard syscall time bound.
    pub timeout_ms: u64,
}

/// Raw serialized metadata cannot construct checked executor conformance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPack {
    /// Numeric action dispatch index.
    pub action: ActionId,
    /// Exact action contract identity.
    pub manifest: ManifestDigest,
    /// Built-in executor selector.
    pub executor: ExecutorIdentity,
    /// Exact implementation version.
    pub executor_version: ExecutorVersionDigest,
    /// Exact authorized resource.
    pub scope: CapabilityScopeSpec,
    /// Exact receipt wire schema.
    pub receipt_schema: ReceiptSchemaDigest,
    /// Explicit timeout behavior.
    pub timeout: TimeoutPolicy,
    /// Declared idempotency axis.
    pub idempotency: IdempotencyClass,
    /// Declared recovery axis.
    pub recovery: RecoveryClass,
    /// Audit identity of actual duplicate/input/resource/reopen observations.
    pub duplicate_ticket_test: ConformanceReportDigest,
}

/// New wire variant preserves the layout of historical Verified evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackedArtifactEvidence {
    /// Independently reconstructed base checks.
    pub base: crate::artifact::ArtifactVerificationEvidence,
    /// Bounded, ordered external action declarations. Requires catalog context.
    pub packs: Box<[ActionPack]>,
}
