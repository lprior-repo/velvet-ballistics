use crate::manual_gate::GateDigest;

/// Immutable origin established by a trusted issuance constructor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScopedGrantAuthority {
    /// Issued by the trusted host path independently of a manual gate.
    TrustedNonGate,
    /// Issued for one concrete bound approval receipt.
    BoundGate { receipt_digest: GateDigest },
    /// Deserialized bytes do not establish a trusted issuance path.
    #[default]
    Untrusted,
}
