use crate::evidence::{EvidenceIndex, required_evidence};
use crate::{ClaimKind, DocReconcileError, EvidenceBoundedReport, MasterDocSnapshot};

pub(super) fn validate(
    doc: MasterDocSnapshot,
    evidence: EvidenceIndex,
) -> Result<EvidenceBoundedReport, DocReconcileError> {
    if let Some((sentence, claim_kind)) = first_unsupported_claim(&doc.text, &evidence) {
        return Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence,
            claim_kind,
            required: required_evidence(),
        });
    }
    let (cited_claims, pending_claims) = evidence.support_counts_for(&doc.text);
    Ok(EvidenceBoundedReport {
        unsupported_claims: Vec::new(),
        cited_claims,
        pending_claims,
        forbidden_claims: Vec::new(),
    })
}

fn first_unsupported_claim(text: &str, evidence: &EvidenceIndex) -> Option<(String, ClaimKind)> {
    let known = [
        ("tests prove joined taint", ClaimKind::TestEvidence),
        (
            "Lean proves implementation parity",
            ClaimKind::FormalEvidence,
        ),
        ("DRIFT-1 is release ready", ClaimKind::ReleaseReadiness),
        (
            "DRIFT-1 generated Rust and IR parity is verified",
            ClaimKind::GeneratedParity,
        ),
        (
            "full generated Rust and IR parity is verified",
            ClaimKind::GeneratedParity,
        ),
    ];
    known.iter().find_map(|(sentence, claim_kind)| {
        if text.contains(sentence) && !evidence.supports_sentence(sentence) {
            Some(((*sentence).to_owned(), *claim_kind))
        } else {
            None
        }
    })
}
