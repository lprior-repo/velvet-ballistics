use super::*;
use vb_core::{ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx};

pub(super) fn action_workflow(actions: &[ActionId]) -> Result<CompiledWorkflow, String> {
    let mut parts = minimal_workflow()?.to_parts();
    let first = parts.nodes.first().ok_or("missing fixture node")?.clone();
    let mut nodes = vec![first];
    for action in actions {
        let id = u16::try_from(nodes.len()).map_err(|e| e.to_string())?;
        let next = id.checked_add(1).ok_or("fixture step overflow")?;
        let after = next.checked_add(1).ok_or("fixture step overflow")?;
        nodes.push(CompiledNode {
            id: StepIdx::new(id),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(next)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: *action,
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(next),
            output: None,
            next: Some(StepIdx::new(after)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(u16::try_from(nodes.len()).map_err(|e| e.to_string())?),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    parts.nodes = nodes.into_boxed_slice();
    parts.slot_count = 2;
    parts.digest = zero_workflow_digest();
    parts.digest = vb_core::WorkflowDigest::from_bytes(
        blake3::hash(&postcard::to_allocvec(&parts).map_err(|e| e.to_string())?).into(),
    );
    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

pub(super) fn assert_checked(
    artifact: &AcceptedArtifact,
    policy: vb_core::RuntimePolicy,
) -> Result<(), String> {
    verify_accepted_artifact_for_policy(artifact, policy).map_err(|e| e.to_string())?;
    assert_eq!(artifact.verification.gate_count, 0);
    assert_eq!(artifact.verification.durable, false);
    assert_eq!(artifact.verification.replayable_claimed, false);
    assert_eq!(artifact.verification.taint_safe_claimed, false);
    Ok(())
}

pub(super) fn assert_declarations(
    artifact: &AcceptedArtifact,
    actions: &[ActionId],
) -> Result<(), String> {
    use vb_core::artifact::{ArtifactEvidence, AssumptionId, ObligationClosure, ObligationKind};
    assert_eq!(
        artifact
            .action_contracts
            .iter()
            .map(|contract| contract.id)
            .collect::<Vec<_>>(),
        actions
    );
    assert_eq!(artifact.verification.idempotency_keyed.as_ref(), &[]);
    assert_eq!(artifact.verification.idempotency_attested.as_ref(), &[]);
    let ArtifactEvidence::Verified(evidence) = &artifact.verification.evidence else {
        return Err("missing checked evidence".into());
    };
    let contract_digest = vb_core::WorkflowDigest::from_bytes(
        blake3::hash(
            &postcard::to_allocvec(&artifact.action_contracts).map_err(|e| e.to_string())?,
        )
        .into(),
    );
    assert_eq!(evidence.action_contract_digest, contract_digest);
    let effect = evidence
        .ledger
        .iter()
        .find(|entry| entry.obligation == ObligationKind::EffectSafety)
        .ok_or("missing effect entry")?;
    assert_eq!(
        effect.closure,
        ObligationClosure::ExternalAssumption {
            assumption_id: AssumptionId::ExecutorHonorsActionContracts,
            evidence_digest: contract_digest
        }
    );
    Ok(())
}
