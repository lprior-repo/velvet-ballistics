use super::errors::GateError;
use vb_core::ids::RunId;
use vb_core::manual_gate::{ApprovalReceipt, ApproverId, GateDecision, GateDigest, ManualGate};

/// Canonical decision byte for receipt digest encoding.
fn decision_byte(decision: GateDecision) -> u8 {
    match decision {
        GateDecision::Approve => 0,
        GateDecision::Deny => 1,
        GateDecision::RequestMoreEvidence => 2,
        GateDecision::Delegate => 3,
        GateDecision::BreakGlass => 4,
    }
}

/// Computes the receipt digest binding gate, run, decision, approver,
/// evidence digest, prompt digest, and decision time, with domain
/// separation. The same inputs always produce the same digest; any
/// field change produces a different digest.
pub fn compute_receipt_digest(
    gate: u64,
    run: RunId,
    decision: GateDecision,
    approver: &ApproverId,
    evidence_bundle_digest: GateDigest,
    prompt_digest: GateDigest,
    decided_at_ms: u64,
) -> Result<GateDigest, GateError> {
    let approver_len =
        u64::try_from(approver.value().len()).map_err(|_| GateError::ReceiptDigestEncoding)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"vb-gate-receipt-v1");
    hasher.update(&gate.to_le_bytes());
    hasher.update(&run.get().to_le_bytes());
    hasher.update(&[decision_byte(decision)]);
    hasher.update(&approver_len.to_le_bytes());
    hasher.update(approver.value().as_bytes());
    hasher.update(&evidence_bundle_digest.as_bytes());
    hasher.update(&prompt_digest.as_bytes());
    hasher.update(&decided_at_ms.to_le_bytes());
    Ok(GateDigest::from_bytes(*hasher.finalize().as_bytes()))
}

/// Verifies a receipt against its gate: the receipt's evidence and prompt
/// digests must match the gate, and its digest must equal the digest
/// recomputed from the receipt's own bound fields.
pub fn verify_receipt(receipt: &ApprovalReceipt, gate: &ManualGate) -> Result<bool, GateError> {
    if receipt.gate != gate.id
        || receipt.run != gate.run
        || receipt.evidence_bundle_digest != gate.evidence_bundle_digest
        || receipt.prompt_digest != gate.prompt_digest
    {
        return Ok(false);
    }
    let recomputed = compute_receipt_digest(
        receipt.gate.get(),
        receipt.run,
        receipt.decision,
        &receipt.approver,
        receipt.evidence_bundle_digest,
        receipt.prompt_digest,
        receipt.decided_at_ms,
    )?;
    Ok(recomputed == receipt.digest)
}
