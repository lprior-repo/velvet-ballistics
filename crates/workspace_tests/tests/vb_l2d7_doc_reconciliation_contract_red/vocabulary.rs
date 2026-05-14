use crate::support::*;

#[test]
fn validate_taint_vocabulary_consistency_returns_report_for_single_lattice() {
    // Given
    let doc = snapshot_with(
        "Clean < DerivedFromSecret < Secret.\n\
         EvalExpr uses joined data-flow taint with join_taint.\n\
         v1 does not track control-flow taint.",
    );
    let expected = TaintVocabularyReport {
        lattice: vec![
            "Clean".to_owned(),
            "DerivedFromSecret".to_owned(),
            "Secret".to_owned(),
        ],
        propagation_rule: TaintVocabularyRule::JoinedDataFlowTaint,
        conflicts: Vec::new(),
        control_flow_scope: PreservedNonGoal::ControlFlowTaintV1NonGoal,
    };

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(result, Ok(expected));
}

#[test]
fn validate_taint_vocabulary_consistency_reports_wrong_order() {
    // Given
    let sentence = "Clean < Secret < DerivedFromSecret";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::WrongOrder,
            sentence: sentence.to_owned(),
            term: None,
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_reports_unknown_taint_term() {
    // Given
    let doc = snapshot_with("Private taint dominates Secret in this resolved node.");

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::UnknownTerm,
            sentence: "Private taint dominates Secret in this resolved node.".to_owned(),
            term: Some("Private".to_owned()),
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_reports_downgrade_wording() {
    // Given
    let sentence = "Secret downgrades to Clean after BuildList";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::Downgrade,
            sentence: sentence.to_owned(),
            term: None,
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_reports_control_flow_conflation() {
    // Given
    let sentence = "joined data-flow taint means v1 tracks branch-condition taint";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::ControlFlowTaintConflation {
            sentence: sentence.to_owned(),
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_accepts_empty_text_with_default_lattice() {
    // Given
    let doc = snapshot_with("");

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result.map(|report| report.lattice),
        Ok(vec![
            "Clean".to_owned(),
            "DerivedFromSecret".to_owned(),
            "Secret".to_owned(),
        ])
    );
}

#[test]
fn validate_taint_vocabulary_consistency_reports_control_flow_conflation_for_secret_branch_phrase()
{
    // Given
    let sentence = "runtime tracks secret branch-condition taint";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::ControlFlowTaintConflation {
            sentence: sentence.to_owned(),
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_reports_unknown_private_term_exactly() {
    // Given
    let sentence = "Private value flows into BuildObject.";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::UnknownTerm,
            sentence: sentence.to_owned(),
            term: Some("Private".to_owned()),
        })
    );
}
