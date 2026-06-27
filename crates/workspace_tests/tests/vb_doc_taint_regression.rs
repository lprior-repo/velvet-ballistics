use xtask::doc_reconcile::reconcile::check_doc_taint_consistency;
use xtask::doc_reconcile::{DocReconcileError, ResolvedNode};

#[test]
fn check_doc_taint_consistency_rejects_eval_expr_always_clean_in_pipe_table() {
    let result = check_doc_taint_consistency(
        "| EvalExpr | Always Clean \u{2014} no taint join of expression operands. |",
    );
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "Always Clean".to_owned(),
        })
    );
}

#[test]
fn check_doc_taint_consistency_rejects_build_object_always_clean_no_field_join() {
    let result = check_doc_taint_consistency(
        "| BuildObject | Always Clean \u{2014} no join of field taints |",
    );
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildObject,
            phrase: "Always Clean".to_owned(),
        })
    );
}

#[test]
fn check_doc_taint_consistency_rejects_build_list_always_clean_no_item_join() {
    let result =
        check_doc_taint_consistency("| BuildList | Always Clean \u{2014} no join of item taints |");
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildList,
            phrase: "Always Clean".to_owned(),
        })
    );
}

#[test]
fn check_doc_taint_consistency_rejects_eval_expr_write_slot_only_claim() {
    let result =
        check_doc_taint_consistency("EvalExpr writes with write_slot (not write_slot_with_taint)");
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "write_slot".to_owned(),
        })
    );
}
