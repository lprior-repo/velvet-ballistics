use super::*;

pub(crate) fn assert_nonapproval_refusals(
    catalog: &ExecutorCatalog,
    artifact: &vb_storage::AcceptedArtifact,
    register: &vb_storage::executor::RegisterGrant,
    params: &vb_core::ScopedGrantParams,
    approval: &BoundApprovalReceipt,
) -> TestResult {
    for decision in [
        GateDecision::Deny,
        GateDecision::Delegate,
        GateDecision::RequestMoreEvidence,
    ] {
        let mut refused = approval.clone();
        refused.receipt.decision = decision;
        refused.digest = vb_runtime::manual_gate::compute_bound_receipt_digest(&refused)?;
        let params = vb_core::ScopedGrantParams {
            approval_receipt: refused.digest,
            ..params.clone()
        };
        let result =
            catalog.issue_scoped_grant_for_bound_gate(artifact, register, params, &refused);
        if !matches!(
            result,
            Err(ExecutorError::Grant(
                vb_core::GrantError::GateReceiptMismatch
            ))
        ) {
            return Err(
                format!("expected non-approval {decision:?} refusal; got {result:?}").into(),
            );
        }
    }
    Ok(())
}
