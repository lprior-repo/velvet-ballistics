use super::*;

#[test]
fn clean_document_has_no_findings() {
    let text = "EvalExpr joins taints. Finished(SlotValue, Taint).";
    assert_eq!(scan(text), Vec::<String>::new());
}

#[test]
fn node_stale_wording_is_detected() {
    let text = "EvalExpr Always Clean";
    assert_eq!(
        scan(text),
        vec![
            "line 1: EvalExpr stale taint wording: Always Clean",
            "missing finish signal taint wording"
        ]
    );
}

#[test]
fn multiple_phrases_on_one_line() {
    let text = "EvalExpr Always Clean and always Clean";
    assert_eq!(
        scan(text),
        vec![
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 1: EvalExpr stale taint wording: always Clean",
            "missing finish signal taint wording"
        ]
    );
}

#[test]
fn vocabulary_conflict_detected() {
    let text = "Clean < Secret < DerivedFromSecret";
    assert_eq!(
        scan(text),
        vec!["wrong lattice order", "missing finish signal taint wording"]
    );
}

#[test]
fn control_flow_conflict_case_insensitive() {
    let text = "EVAL tracks secret branch-condition taint";
    assert_eq!(
        scan(text),
        vec![
            "control-flow taint conflation: tracks secret branch-condition taint",
            "missing finish signal taint wording"
        ]
    );
}

#[test]
fn stale_finish_without_comma() {
    let text = "Finished(SlotValue).";
    assert_eq!(
        scan(text),
        vec![
            "stale finish signal missing taint",
            "missing finish signal taint wording"
        ]
    );
}

#[test]
fn fresh_finish_with_comma_is_ok() {
    let text = "Finished(SlotValue, Taint).";
    assert_eq!(scan(text), Vec::<String>::new());
}

#[test]
fn mixed_stale_and_fresh_finish() {
    let text = "Finished(SlotValue, Taint) and also Finished(SlotValue).";
    assert_eq!(scan(text), vec!["stale finish signal missing taint"]);
}

#[test]
fn finish_rejection_contradiction() {
    let text = "When Finish arrives with Secret, reject it.";
    assert_eq!(
        scan(text),
        vec![
            "finish rejection contradiction",
            "missing finish signal taint wording"
        ]
    );
}

#[test]
fn finish_allowed_phrases_not_contradiction() {
    let text = "When Finish arrives with Secret, no rejection.";
    assert_eq!(scan(text), vec!["missing finish signal taint wording"]);
}

#[test]
fn missing_required_finish_wording() {
    let text = "EvalExpr joins taints.";
    assert_eq!(scan(text), vec!["missing finish signal taint wording"]);
}

#[test]
fn writer_contradiction_detected() {
    let text = "EvalExpr uses write_slot but not write_slot_with_taint.";
    assert_eq!(
        scan(text),
        vec![
            "line 1: EvalExpr stale taint wording: write_slot",
            "missing finish signal taint wording"
        ]
    );
}

#[test]
fn finding_order_is_correct() {
    let text = "EvalExpr Always Clean. Clean < Secret < DerivedFromSecret. tracks branch-condition taint. Finished(SlotValue).";
    assert_eq!(
        scan(text),
        vec![
            "line 1: EvalExpr stale taint wording: Always Clean",
            "wrong lattice order",
            "control-flow taint conflation: tracks branch-condition taint",
            "stale finish signal missing taint",
            "missing finish signal taint wording"
        ]
    );
}

#[test]
fn writer_finding_follows_phases_for_same_node() {
    let text = "EvalExpr Always Clean uses write_slot but not write_slot_with_taint.";
    assert_eq!(
        scan(text),
        vec![
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 1: EvalExpr stale taint wording: write_slot",
            "missing finish signal taint wording"
        ]
    );
}
