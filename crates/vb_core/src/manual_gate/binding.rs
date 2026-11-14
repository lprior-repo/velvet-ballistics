use super::GateDigest;
use serde::{Deserialize, Serialize};

/// Durable run binding carried by a bound gate and its approval receipt.
///
/// Added alongside the unchanged V1 `ManualGate` layout: existing gates and
/// records carry no binding and remain decodable exactly as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateBinding {
    /// Accepted compiled artifact digest.
    pub artifact_digest: GateDigest,
    /// Policy/resource-contract digest in force at admission.
    pub policy_digest: GateDigest,
    /// Canonical typed-input digest.
    pub input_digest: GateDigest,
    /// Run attempt the gate belongs to; zero is the invalid placeholder.
    pub attempt: u16,
}

impl GateBinding {
    /// Returns true when every digest is a real binding and the attempt is
    /// non-zero.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.is_zero_digest(self.artifact_digest)
            && !self.is_zero_digest(self.policy_digest)
            && !self.is_zero_digest(self.input_digest)
            && self.attempt != 0
    }

    fn is_zero_digest(&self, digest: GateDigest) -> bool {
        digest.as_bytes() == [0; 32]
    }
}
