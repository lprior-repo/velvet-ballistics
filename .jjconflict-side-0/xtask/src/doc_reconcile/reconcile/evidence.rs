use crate::doc_reconcile::{
    ClaimKind, DocReconcileError, EvidenceBoundedReport, MasterDocSnapshot, RequiredEvidence,
};

struct EvidenceClaimPattern {
    sentence: &'static str,
    claim_kind: ClaimKind,
}

const EVIDENCE_CLAIM_PATTERNS: &[EvidenceClaimPattern] = &[
    EvidenceClaimPattern {
        sentence: "tests prove joined taint",
        claim_kind: ClaimKind::TestEvidence,
    },
    EvidenceClaimPattern {
        sentence: "Lean proves implementation parity",
        claim_kind: ClaimKind::FormalEvidence,
    },
    EvidenceClaimPattern {
        sentence: "DRIFT-1 is release ready",
        claim_kind: ClaimKind::ReleaseReadiness,
    },
    EvidenceClaimPattern {
        sentence: "full generated Rust and IR parity is verified",
        claim_kind: ClaimKind::GeneratedParity,
    },
    EvidenceClaimPattern {
        sentence: "DRIFT-1 generated Rust and IR parity is verified",
        claim_kind: ClaimKind::GeneratedParity,
    },
];

pub fn validate_evidence_bounded_wording(
    doc: MasterDocSnapshot,
    evidence: crate::doc_reconcile::evidence::EvidenceIndex,
) -> Result<EvidenceBoundedReport, DocReconcileError> {
    for pattern in EVIDENCE_CLAIM_PATTERNS {
        if doc.text.contains(pattern.sentence)
            && !evidence.has_support_for_claim(pattern.sentence, &doc.text)
        {
            return Err(DocReconcileError::UnsupportedEvidenceClaim {
                sentence: pattern.sentence.to_owned(),
                claim_kind: pattern.claim_kind,
                required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
            });
        }
    }

    Ok(EvidenceBoundedReport {
        unsupported_claims: Vec::new(),
        cited_claims: evidence.cited_count_in_text(&doc.text),
        pending_claims: evidence.pending_count_in_text(&doc.text),
        forbidden_claims: Vec::new(),
    })
}
