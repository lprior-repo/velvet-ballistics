#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Digest and observation-slot helpers.
//!
//! Pure, deterministic helpers for building [`DigestObservation`]s
//! and [`SlotObservation`]s. All functions in this module are
//! allocation-free with the exception of [`capability_set_digest`],
//! which degrades to the [`ALLOCATION_FAILED_SENTINEL`] sentinel on
//! allocation failure so two divergent runs never produce
//! false-positive divergence from a transient OOM.

use blake3::Hasher;
use vb_core::{CapabilitySet, SlotIdx, WorkflowDigest};

use super::subject::{DigestObservation, DigestSubject, SEMANTIC_OBSERVATION_SCHEMA_VERSION};

/// Fixed-width BLAKE3 domain-separation prefix bound to the observation schema.
///
/// Versioned so a schema bump invalidates hashes even when the
/// underlying canonical encoding accidentally collides with an older
/// version.
pub(crate) const OBSERVATION_DOMAIN_PREFIX: &[u8; 35] = b"vb-storage.semantic-observation.v2\x00";

/// Sentinel digest returned when a serde-based encoding fails (allocation
/// exhausted). Always identical for the same subject so two divergent runs
/// both degrade to the same value rather than producing false-positive
/// divergence.
pub(crate) const ALLOCATION_FAILED_SENTINEL: [u8; 32] = [
    0xA1, 0x1F, 0xC4, 0x09, 0xBA, 0x5E, 0x2C, 0xD3, 0x88, 0x7B, 0x4E, 0x6F, 0xA0, 0xCD, 0x15, 0x82,
    0x33, 0x57, 0xE9, 0x4B, 0x91, 0x26, 0x7D, 0xC8, 0x60, 0xA3, 0xF1, 0x08, 0x5E, 0xBC, 0x74, 0x2A,
];

/// Build a `DigestObservation` over arbitrary bytes.
///
/// Uses `blake3` directly so the encoding is deterministic and
/// collision-free for the byte sequences we feed in.
#[must_use]
pub(crate) fn serialized_digest(subject: DigestSubject, bytes: &[u8]) -> DigestObservation {
    let mut hasher = Hasher::new();
    hasher.update(OBSERVATION_DOMAIN_PREFIX);
    hasher.update(&[subject.tag()]);
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    DigestObservation {
        subject,
        bytes: out,
    }
}

/// Build a `DigestObservation` directly from a `WorkflowDigest`.
#[must_use]
pub(crate) fn workflow_digest_observation(
    subject: DigestSubject,
    digest: WorkflowDigest,
) -> DigestObservation {
    DigestObservation {
        subject,
        bytes: digest.as_bytes(),
    }
}

/// Compute a stable string digest (BLAKE3 of the UTF-8 bytes).
#[must_use]
pub(crate) fn str_digest(s: &str) -> DigestObservation {
    serialized_digest(DigestSubject::CancellationReason, s.as_bytes())
}

/// Canonical BLAKE3 encoding of a slot-write event.
///
/// Returns the value digest (when present) and the extra-envelope digest
/// (when present). `attempt` is bound into the prefix so slot writes at
/// different attempts on the same slot do not collide.
#[must_use]
pub(crate) fn slot_observation(
    slot: SlotIdx,
    attempt: u16,
    value: Option<&[u8]>,
    extra: Option<&[u8]>,
) -> super::ask::SlotObservation {
    let value_digest = value.map(|bytes| serialized_digest(DigestSubject::Slot, bytes));
    let extra_digest = extra.map(|bytes| serialized_digest(DigestSubject::Slot, bytes));
    super::ask::SlotObservation {
        slot,
        attempt,
        value_digest,
        extra_digest,
    }
}

/// Compute the deterministic BLAKE3 digest over an ordered slice of observations.
///
/// The output is the raw 32-byte BLAKE3 finalization; the schema version
/// is bound into the prefix so version collisions cannot occur.
#[must_use]
pub(crate) fn observation_digest(
    observations: &[super::signature::JournalObservation],
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(OBSERVATION_DOMAIN_PREFIX);
    // The observation count is bounded by the journal event count, which
    // is itself bounded by `MAX_BATCH_COUNT` per write. `u64` is the
    // canonical BLAKE3 length type; the slice length is at most `usize`.
    #[allow(clippy::as_conversions)]
    let count = encode_u64(observations.len() as u64);
    hasher.update(&count);
    for observation in observations {
        super::encode::encode_observation_into(observation, &mut hasher);
    }
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

/// Re-export the schema version for callers that prefer the digest
/// module's namespace. Mirrors [`super::subject::SEMANTIC_OBSERVATION_SCHEMA_VERSION`].
pub(crate) const SCHEMA_VERSION: u16 = SEMANTIC_OBSERVATION_SCHEMA_VERSION;

/// Postcard-encoded bytes for a `CapabilitySet`, or the
/// [`ALLOCATION_FAILED_SENTINEL`] on allocation failure.
///
/// Kept separate from [`capability_set_digest`] so the encoding
/// boundary is testable in isolation.
pub(crate) fn encode_capability_set_bytes(capabilities: &CapabilitySet) -> Result<Vec<u8>, ()> {
    let capacity = capabilities.len().saturating_mul(64).max(64);
    let mut buf: Vec<u8> = Vec::new();
    if buf.try_reserve(capacity).is_err() {
        return Err(());
    }
    postcard::to_allocvec(capabilities).map_err(|_| ())
}

/// Compute the BLAKE3 digest of a `CapabilitySet` for stable comparison.
///
/// Uses `postcard` to serialize the capability set. Postcard encoding
/// of a `#[derive(Serialize)]` struct is deterministic for the same
/// input, so two equivalent `CapabilitySet` values always produce the
/// same digest regardless of allocation path.
///
/// On allocation failure (extremely large grants), returns the fixed
/// [`ALLOCATION_FAILED_SENTINEL`] so both divergent runs degrade
/// identically and produce no false-positive divergence.
#[must_use]
pub(crate) fn capability_set_digest(capabilities: &CapabilitySet) -> DigestObservation {
    let mut hasher = Hasher::new();
    hasher.update(OBSERVATION_DOMAIN_PREFIX);
    hasher.update(&[DigestSubject::CapabilitySet.tag()]);
    let bytes = match encode_capability_set_bytes(capabilities) {
        Ok(bytes) => bytes,
        Err(()) => {
            return DigestObservation {
                subject: DigestSubject::CapabilitySet,
                bytes: ALLOCATION_FAILED_SENTINEL,
            };
        }
    };
    hasher.update(&bytes);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    DigestObservation {
        subject: DigestSubject::CapabilitySet,
        bytes: out,
    }
}

fn encode_u64(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

/// Test-only helper: compute the capability-set digest over a
/// pre-computed byte payload (or the sentinel when `Err(())`).
///
/// This bypasses the [`CapabilitySet`] → bytes step so the
/// sentinel-collapse contract can be tested deterministically
/// without relying on postcard's allocation-failure behavior. Only
/// compiled under `cfg(test)` so it cannot leak into production.
#[cfg(test)]
pub(crate) fn capability_set_digest_from_bytes(
    encode_result: Result<Vec<u8>, ()>,
) -> DigestObservation {
    let mut hasher = Hasher::new();
    hasher.update(OBSERVATION_DOMAIN_PREFIX);
    hasher.update(&[DigestSubject::CapabilitySet.tag()]);
    let bytes = match encode_result {
        Ok(bytes) => bytes,
        Err(()) => {
            return DigestObservation {
                subject: DigestSubject::CapabilitySet,
                bytes: ALLOCATION_FAILED_SENTINEL,
            };
        }
    };
    hasher.update(&bytes);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    DigestObservation {
        subject: DigestSubject::CapabilitySet,
        bytes: out,
    }
}
