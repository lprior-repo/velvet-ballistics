#![forbid(unsafe_code)]
//! V2 bound-receipt digest and verification.

use super::errors::GateError;
use super::receipt::verify_receipt;
use vb_core::manual_gate::{
    BoundApprovalReceipt, BoundManualGate, GateDecision, GateDigest, OperatorTrust, RiskClass,
    SeparationOfDuties,
};

/// Computes the V2 digest over every accountable bound-receipt field except
/// the digest itself. Domain separation is `vb-gate-receipt-v2`.
pub fn compute_bound_receipt_digest(
    receipt: &BoundApprovalReceipt,
) -> Result<GateDigest, GateError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"vb-gate-receipt-v2");
    hasher.update(&receipt.receipt.gate.get().to_le_bytes());
    hasher.update(&receipt.receipt.run.get().to_le_bytes());
    hasher.update(&receipt.action.get().to_le_bytes());
    hasher.update(&[decision_byte(receipt.receipt.decision)]);
    hasher.update(&[risk_byte(receipt.risk)]);
    hasher.update(&[separation_byte(receipt.separation)]);
    update_text(&mut hasher, receipt.receipt.approver.value())?;
    update_text(&mut hasher, receipt.approver_role.name())?;
    update_text(&mut hasher, receipt.requester.value())?;
    hasher.update(&receipt.receipt.evidence_bundle_digest.as_bytes());
    hasher.update(&receipt.receipt.prompt_digest.as_bytes());
    hasher.update(&receipt.binding.artifact_digest.as_bytes());
    hasher.update(&receipt.binding.policy_digest.as_bytes());
    hasher.update(&receipt.binding.input_digest.as_bytes());
    hasher.update(&receipt.binding.attempt.to_le_bytes());
    hasher.update(&[trust_byte(receipt.operator_trust)]);
    hasher.update(&receipt.receipt.decided_at_ms.to_le_bytes());
    Ok(GateDigest::from_bytes(*hasher.finalize().as_bytes()))
}

fn update_text(hasher: &mut blake3::Hasher, text: &str) -> Result<(), GateError> {
    let len = u64::try_from(text.len()).map_err(|_| GateError::ReceiptDigestEncoding)?;
    hasher.update(&len.to_le_bytes());
    hasher.update(text.as_bytes());
    Ok(())
}

const fn decision_byte(decision: GateDecision) -> u8 {
    match decision {
        GateDecision::Approve => 0,
        GateDecision::Deny => 1,
        GateDecision::RequestMoreEvidence => 2,
        GateDecision::Delegate => 3,
        GateDecision::BreakGlass => 4,
    }
}

const fn risk_byte(risk: RiskClass) -> u8 {
    match risk {
        RiskClass::Low => 0,
        RiskClass::Medium => 1,
        RiskClass::High => 2,
    }
}

const fn separation_byte(separation: SeparationOfDuties) -> u8 {
    match separation {
        SeparationOfDuties::Disabled => 0,
        SeparationOfDuties::DistinctFromRequester => 1,
    }
}

const fn trust_byte(trust: OperatorTrust) -> u8 {
    match trust {
        OperatorTrust::LocalSocketClaim => 0,
        OperatorTrust::InProcessCaller => 1,
    }
}

/// Verifies a bound receipt against its bound gate: every bound field must
/// match the gate and the V2 digest must recompute. The embedded V1 receipt is
/// also verified so an old-digest tamper cannot hide under the new wrapper.
pub fn verify_bound_receipt(
    receipt: &BoundApprovalReceipt,
    gate: &BoundManualGate,
) -> Result<bool, GateError> {
    if !gate.is_structurally_valid()
        || receipt.receipt.gate != gate.gate.id
        || receipt.receipt.run != gate.gate.run
        || receipt.action != gate.gate.action
        || !receipt.approver_role.is_valid()
        || !receipt.requester.is_valid()
        || !receipt.receipt.approver.is_valid()
        || !matches!(
            receipt.receipt.decision,
            GateDecision::Approve | GateDecision::BreakGlass
        )
        || (receipt.receipt.decision != GateDecision::BreakGlass
            && receipt.approver_role != gate.gate.required_role)
        || receipt.risk != gate.gate.risk
        || receipt.separation != gate.gate.separation_of_duties
        || receipt.binding != gate.binding
        || receipt.receipt.evidence_bundle_digest != gate.gate.evidence_bundle_digest
        || receipt.receipt.prompt_digest != gate.gate.prompt_digest
        || !verify_receipt(&receipt.receipt, &gate.gate)?
    {
        return Ok(false);
    }
    Ok(compute_bound_receipt_digest(receipt)? == receipt.digest)
}
