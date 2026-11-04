//! Exact historical schema-3 provenance, after current validation.
//! This rebuilds comparison evidence only; retained bytes are never rewritten.

use super::*;

const HISTORICAL_VERIFIERS: [[u8; 32]; 2] = [
    // 260ad7c4084a8d218e04f54e6d3ec0d708cf9324, six original source bodies.
    [
        0x63, 0x99, 0xd3, 0x46, 0x35, 0xe7, 0xcf, 0x7c, 0xe6, 0x1d, 0xaa, 0x44, 0x10, 0xce, 0x8b,
        0x0c, 0x7a, 0x68, 0xb7, 0xc8, 0x9e, 0x87, 0x01, 0x9b, 0xdf, 0x88, 0x6e, 0x8f, 0xa2, 0x9e,
        0x18, 0x48,
    ],
    // cdc29c78a67e5c49ccf85f367ddb374d3d58a5b7, same order, no separators.
    [
        0x91, 0x54, 0x4c, 0x6c, 0xa0, 0xb1, 0x3b, 0xd8, 0x5f, 0x62, 0xe6, 0x2c, 0xc0, 0x3a, 0xdb,
        0xe5, 0xc6, 0xeb, 0x10, 0x34, 0x49, 0xcd, 0xec, 0xb2, 0x32, 0xe7, 0xe0, 0x8c, 0x60, 0xcd,
        0x52, 0x3e,
    ],
];

/// The caller must first revalidate IR, contracts, policy and registered packs.
pub(super) fn expected_proof(
    actual: &AcceptedArtifact,
    expected: &AcceptedArtifact,
) -> Result<VerificationProof, JournalError> {
    let held = base(&actual.verification.evidence)?;
    let current = base(&expected.verification.evidence)?;
    if held.schema_version != EVIDENCE_VERSION || current.schema_version != EVIDENCE_VERSION {
        return Err(JournalError::ArtifactMalformed);
    }
    let historical = held.verifier_digest != current.verifier_digest;
    if historical && !known_verifier(held.verifier_digest) {
        return Err(JournalError::ArtifactMalformed);
    }
    let mut proof = expected.verification.clone();
    if historical {
        let rebuilt = base_mut(&mut proof.evidence)?;
        rebuilt.verifier_digest = held.verifier_digest;
        rebuilt.ledger = obligation_ledger(
            rebuilt.policy_digest,
            held.verifier_digest,
            effect_closure(&expected.action_contracts, held.verifier_digest)?,
        )?;
    }
    // The envelope checksum was checked by the public verification entrypoint.
    // Rebind only the local expected proof's self-digest for the exact comparison.
    proof.digest = actual.verification.digest;
    Ok(proof)
}

fn known_verifier(verifier: WorkflowDigest) -> bool {
    HISTORICAL_VERIFIERS
        .iter()
        .any(|bytes| verifier == WorkflowDigest::from_bytes(*bytes))
}

fn base(evidence: &ArtifactEvidence) -> Result<&ArtifactVerificationEvidence, JournalError> {
    match evidence {
        ArtifactEvidence::Verified(evidence) => Ok(evidence),
        ArtifactEvidence::Packed(evidence) => Ok(&evidence.base),
        ArtifactEvidence::Unverified => Err(JournalError::ArtifactMalformed),
    }
}

fn base_mut(
    evidence: &mut ArtifactEvidence,
) -> Result<&mut ArtifactVerificationEvidence, JournalError> {
    match evidence {
        ArtifactEvidence::Verified(evidence) => Ok(evidence),
        ArtifactEvidence::Packed(evidence) => Ok(&mut evidence.base),
        ArtifactEvidence::Unverified => Err(JournalError::ArtifactMalformed),
    }
}
