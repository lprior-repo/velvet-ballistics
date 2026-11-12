use crate::support::*;

#[test]
fn scan_for_stale_clean_only_text_returns_empty_report_when_resolved_nodes_use_joined_taint() {
    // Given
    let doc = snapshot_with(
        "EvalExpr output taint is join_taint over loaded slot taints.\n\
         BuildObject output taint is the join of field slot taints.\n\
         BuildList output taint is the join of item slot taints.\n\
         Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );
    let expected = ContradictionReport {
        stale_clean_only: Vec::new(),
        no_join_claims: Vec::new(),
        write_slot_only_claims: Vec::new(),
        scanned_nodes: vec![
            ResolvedNode::EvalExpr,
            ResolvedNode::BuildObject,
            ResolvedNode::BuildList,
            ResolvedNode::Finish,
        ],
    };

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(result, Ok(expected));
}

#[test]
fn scan_for_stale_clean_only_text_reports_eval_expr_always_clean() {
    // Given
    let doc = snapshot_with("| EvalExpr | Always Clean |");

    // When
    let result = scan_for_stale_clean_only_text(doc);

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
fn scan_for_stale_clean_only_text_reports_eval_expr_no_operand_join() {
    // Given
    let doc = snapshot_with("EvalExpr performs No taint join of expression operands");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "No taint join".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_reports_build_object_always_clean() {
    // Given
    let doc = snapshot_with("| BuildObject | Always Clean |");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildObject,
            phrase: "Always Clean".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_reports_build_object_no_field_join() {
    // Given
    let doc = snapshot_with("BuildObject has no join of field taints");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildObject,
            phrase: "no join of field taints".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_reports_build_list_always_clean() {
    // Given
    let doc = snapshot_with("| BuildList | Always Clean |");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildList,
            phrase: "Always Clean".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_reports_build_list_no_item_join() {
    // Given
    let doc = snapshot_with("BuildList has no join of item taints");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildList,
            phrase: "no join of item taints".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_reports_write_slot_only_semantics_for_resolved_nodes() {
    // Given
    let doc = snapshot_with("EvalExpr writes with write_slot and not write_slot_with_taint");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "write_slot".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_reports_lowercase_always_clean_for_eval_expr() {
    // Given
    let doc = snapshot_with("EvalExpr always Clean after expression evaluation");

    // When
    let result = scan_for_stale_clean_only_text(doc);

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
fn scan_for_stale_clean_only_text_reports_eval_expr_write_slot_only() {
    // Given
    let doc = snapshot_with("EvalExpr uses write_slot and not write_slot_with_taint");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "write_slot".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_reports_build_object_write_slot_only() {
    // Given
    let doc = snapshot_with("BuildObject uses write_slot and not write_slot_with_taint");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildObject,
            phrase: "write_slot".to_owned(),
        })
    );
}

#[test]
fn scan_for_stale_clean_only_text_reports_build_list_write_slot_only() {
    // Given
    let doc = snapshot_with("BuildList uses write_slot and not write_slot_with_taint");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildList,
            phrase: "write_slot".to_owned(),
        })
    );
}
