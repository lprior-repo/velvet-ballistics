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
pub(super) fn assert_action_completed_envelope(
    events: &[JournalEvent],
    run: RunId,
    ticket: vb_core::ActionTicket,
    expected_output: SlotIdx,
    expected_value: SlotValue,
    artifact: &vb_storage::AcceptedArtifact,
    action: ActionId,
) -> TestResult {
    let (
        seq,
        completed_run,
        completed_ticket,
        output,
        outcome,
        value,
        encoded_len,
        taint,
        value_digest,
        action_abi_digest,
    ) = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::ActionCompletedEnvelope {
                run,
                seq,
                ticket,
                output,
                outcome,
                value,
                encoded_len,
                taint,
                value_digest,
                action_abi_digest,
            } => Some((
                *seq,
                *run,
                *ticket,
                *output,
                *outcome,
                value,
                *encoded_len,
                *taint,
                value_digest,
                *action_abi_digest,
            )),
            _ => None,
        })
        .ok_or("cold reconciliation did not append completion envelope")?;
    let expected_bytes = postcard::to_allocvec(&expected_value)?;
    let expected_len = u32::try_from(expected_bytes.len())?;
    let expected_digest = *blake3::hash(&expected_bytes).as_bytes();
    let abi_contract = artifact
        .action_contracts
        .iter()
        .find(|contract| contract.id == action)
        .ok_or("accepted artifact missing registered action contract")?;
    let abi_bytes = postcard::to_allocvec(abi_contract)?;
    let expected_abi = WorkflowDigest::from_bytes(blake3::hash(&abi_bytes).into());
    assert_eq!(completed_run, run);
    assert!(seq.get() > 0);
    assert_eq!(completed_ticket, ticket);
    assert_eq!(output, expected_output);
    assert_eq!(outcome, vb_storage::DurableActionOutcome::Ready);
    assert_eq!(value, &expected_bytes);
    assert_eq!(encoded_len, expected_len);
    assert_eq!(taint, vb_core::Taint::Clean);
    assert_eq!(value_digest, &expected_digest);
    assert_eq!(action_abi_digest, expected_abi);
    Ok(())
}
