use crate::support::*;

#[test]
fn check_doc_taint_consistency_reports_eval_expr_always_clean() {
    // Given
    let text = "EvalExpr Always Clean";

    // When
    let result = vb_doc::reconcile::check_doc_taint_consistency(text);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "Always Clean".to_owned(),
        })
    );
}

#[test]
fn check_doc_taint_consistency_returns_empty_report_for_joined_text() {
    // Given
    let text = "EvalExpr output taint is join_taint over loaded slot taints.";

    // When
    let result = vb_doc::reconcile::check_doc_taint_consistency(text);

    // Then
    assert_eq!(
        result,
        Ok(ContradictionReport {
            stale_clean_only: Vec::new(),
            no_join_claims: Vec::new(),
            write_slot_only_claims: Vec::new(),
            scanned_nodes: vec![
                ResolvedNode::EvalExpr,
                ResolvedNode::BuildObject,
                ResolvedNode::BuildList,
                ResolvedNode::Finish,
            ],
        })
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
