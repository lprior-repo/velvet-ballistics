use crate::support::*;

#[test]
fn plan_taint_doc_reconciliation_returns_patch_plan_when_doc_has_stale_nodes_and_evidence_is_bounded()
 {
    // Given
    let doc = snapshot_with(
        "DRIFT-1 joined taint resolved.\n\
         | EvalExpr | Always Clean — No taint join of expression operands |\n\
         | BuildObject | Always Clean — no join of field taints |\n\
         | BuildList | Always Clean — no join of item taints |\n\
         Finish currently omits EngineSignal::Finished(SlotValue, Taint).\n\
         v1 does not track control-flow taint.",
    );
    let expected = DocPatchPlan {
        target: PatchTarget::MasterDoc(master_doc_path()),
        edits: vec![
            PatchEdit::EvalExprJoin,
            PatchEdit::BuildObjectJoin,
            PatchEdit::BuildListJoin,
            PatchEdit::FinishCarriesTaint,
        ],
        stale_text_removed: vec![
            StalePhrase::EvalExprAlwaysClean,
            StalePhrase::EvalExprNoOperandJoin,
            StalePhrase::BuildObjectAlwaysClean,
            StalePhrase::BuildObjectNoFieldJoin,
            StalePhrase::BuildListAlwaysClean,
            StalePhrase::BuildListNoItemJoin,
        ],
        evidence_actions: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        preserved_non_goals: vec![PreservedNonGoal::ControlFlowTaintV1NonGoal],
        forbidden_actions: Vec::new(),
        contradiction_count: 6,
        status: PatchPlanStatus::NeedsReconciliation,
    };

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(result, Ok(expected));
}

#[test]
fn plan_taint_doc_reconciliation_returns_noop_plan_when_doc_is_already_consistent() {
    // Given
    let doc = snapshot_with(
        "EvalExpr output taint is join_taint over loaded slot taints.\n\
         BuildObject output taint is the join of field slot taints.\n\
         BuildList output taint is the join of item slot taints.\n\
         Finish emits EngineSignal::Finished(SlotValue, Taint).\n\
         v1 does not track control-flow taint.",
    );
    let expected = DocPatchPlan {
        target: PatchTarget::MasterDoc(master_doc_path()),
        edits: Vec::new(),
        stale_text_removed: Vec::new(),
        evidence_actions: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        preserved_non_goals: vec![PreservedNonGoal::ControlFlowTaintV1NonGoal],
        forbidden_actions: Vec::new(),
        contradiction_count: 0,
        status: PatchPlanStatus::AlreadyConsistent,
    };

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(result, Ok(expected));
}

#[test]
fn plan_taint_doc_reconciliation_preserves_control_flow_non_goal_when_data_flow_is_reconciled() {
    // Given
    let doc = snapshot_with(
        "EvalExpr Always Clean. BuildObject Always Clean. BuildList Always Clean.\n\
         Explicit v1 non-goal: v1 does not track control-flow taint.",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.preserved_non_goals),
        Ok(vec![PreservedNonGoal::ControlFlowTaintV1NonGoal])
    );
}

#[test]
fn plan_taint_doc_reconciliation_returns_wrong_workspace_when_target_is_outside_vb_l2d7() {
    // Given
    let doc = outside_snapshot_with("EvalExpr output taint is join_taint over loaded slot taints.");

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::WrongWorkspace {
            path: outside_doc_path(),
        })
    );
}

#[test]
fn plan_taint_doc_reconciliation_accepts_policy_defined_workspace_root() {
    // Given
    let workspace_root = std::env::temp_dir().join("vb-policy-root");
    let doc_path = workspace_root.join("velvet-ballistics-MASTER.md");
    let doc = MasterDocSnapshot::for_workspace_text(
        doc_path.clone(),
        "EvalExpr output taint is join_taint over loaded slot taints.\n\
         BuildObject output taint is the join of field slot taints.\n\
         BuildList output taint is the join of item slot taints.\n\
         Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, EvidencePolicy::strict_bounded(workspace_root));

    // Then
    assert_eq!(
        result.map(|plan| plan.target),
        Ok(PatchTarget::MasterDoc(doc_path))
    );
}

#[test]
fn plan_taint_doc_reconciliation_returns_unsupported_evidence_claim_when_policy_forbids_uncited_claim()
 {
    // Given
    let sentence = "DRIFT-1 generated Rust and IR parity is verified";
    let doc = snapshot_with(sentence);

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: sentence.to_owned(),
            claim_kind: ClaimKind::GeneratedParity,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn plan_taint_doc_reconciliation_returns_control_flow_conflation_when_patch_would_claim_v1_branch_taint()
 {
    // Given
    let sentence = "v1 joined taint tracks secret branch-condition taint";
    let doc = snapshot_with(sentence);

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::ControlFlowTaintConflation {
            sentence: sentence.to_owned(),
        })
    );
}
