use crate::support::*;

#[test]
fn plan_taint_doc_reconciliation_adds_eval_expr_edit_for_eval_expr_stale_phrase() {
    // Given
    let doc = snapshot_with(
        "EvalExpr Always Clean. Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.edits),
        Ok(vec![PatchEdit::EvalExprJoin])
    );
}

#[test]
fn plan_taint_doc_reconciliation_adds_build_object_edit_for_build_object_stale_phrase() {
    // Given
    let doc = snapshot_with(
        "BuildObject Always Clean. Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.edits),
        Ok(vec![PatchEdit::BuildObjectJoin])
    );
}

#[test]
fn plan_taint_doc_reconciliation_adds_build_list_edit_for_build_list_stale_phrase() {
    // Given
    let doc = snapshot_with(
        "BuildList Always Clean. Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.edits),
        Ok(vec![PatchEdit::BuildListJoin])
    );
}

#[test]
fn plan_taint_doc_reconciliation_adds_finish_edit_when_finish_taint_sentence_is_absent() {
    // Given
    let doc = snapshot_with("EvalExpr output taint is join_taint over loaded slot taints.");

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.edits),
        Ok(vec![PatchEdit::FinishCarriesTaint])
    );
}

#[test]
fn scan_for_stale_clean_only_text_rejects_finish_signal_without_taint() {
    // Given
    let doc = snapshot_with("Finish emits EngineSignal::Finished(SlotValue).");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::Finish,
            phrase: "Finished(SlotValue)".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_rejects_finish_secret_rejection_contradiction() {
    // Given
    let doc = snapshot_with(
        "Finish compile-time validation rejects Secret finish results, but runtime preserves taint.",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::Finish,
            phrase: "rejects finish taint".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_rejects_valid_finish_signal_plus_secret_rejection() {
    // Given
    let doc = snapshot_with(
        "Finish emits EngineSignal::Finished(SlotValue, Taint). Compile-time rejects Secret finish results.",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::Finish,
            phrase: "rejects finish taint".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_rejects_derived_finish_rejection() {
    // Given
    let doc = snapshot_with("Finish rejects DerivedFromSecret result taint.");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::Finish,
            phrase: "rejects finish taint".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_accepts_finish_no_rejection_wording() {
    // Given
    let doc = snapshot_with(
        "Finish emits EngineSignal::Finished(SlotValue, Taint). No rejection of Secret or DerivedFromSecret results.",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result.map(|report| report.scanned_nodes),
        Ok(vec![
            ResolvedNode::EvalExpr,
            ResolvedNode::BuildObject,
            ResolvedNode::BuildList,
            ResolvedNode::Finish,
        ])
    );
}

#[test]
fn plan_taint_doc_reconciliation_counts_single_stale_phrase() {
    // Given
    let doc = snapshot_with(
        "EvalExpr Always Clean. Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(result.map(|plan| plan.contradiction_count), Ok(1));
}

#[test]
fn plan_taint_doc_reconciliation_records_eval_expr_stale_phrase() {
    // Given
    let doc = snapshot_with(
        "EvalExpr Always Clean. Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.stale_text_removed),
        Ok(vec![StalePhrase::EvalExprAlwaysClean])
    );
}

#[test]
fn plan_taint_doc_reconciliation_records_build_object_no_join_stale_phrase() {
    // Given
    let doc = snapshot_with(
        "BuildObject has no join of field taints. Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.stale_text_removed),
        Ok(vec![StalePhrase::BuildObjectNoFieldJoin])
    );
}

#[test]
fn plan_taint_doc_reconciliation_records_build_list_no_join_stale_phrase() {
    // Given
    let doc = snapshot_with(
        "BuildList has no join of item taints. Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.stale_text_removed),
        Ok(vec![StalePhrase::BuildListNoItemJoin])
    );
}

#[test]
fn plan_taint_doc_reconciliation_reports_needs_reconciliation_for_stale_phrase() {
    // Given
    let doc = snapshot_with(
        "EvalExpr Always Clean. Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.status),
        Ok(PatchPlanStatus::NeedsReconciliation)
    );
}

#[test]
fn plan_taint_doc_reconciliation_reports_already_consistent_for_joined_nodes_and_finish() {
    // Given
    let doc = snapshot_with(
        "EvalExpr output taint is join_taint over loaded slot taints.\n\
         BuildObject output taint is the join of field slot taints.\n\
         BuildList output taint is the join of item slot taints.\n\
         Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(
        result.map(|plan| plan.status),
        Ok(PatchPlanStatus::AlreadyConsistent)
    );
}

#[test]
fn plan_taint_doc_reconciliation_leaves_non_goals_empty_when_control_flow_non_goal_absent() {
    // Given
    let doc = snapshot_with("Finish emits EngineSignal::Finished(SlotValue, Taint).");

    // When
    let result = plan_taint_doc_reconciliation(doc, strict_policy());

    // Then
    assert_eq!(result.map(|plan| plan.preserved_non_goals), Ok(Vec::new()));
}
