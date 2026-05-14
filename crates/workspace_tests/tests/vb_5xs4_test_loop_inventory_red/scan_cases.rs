use super::*;

#[test]
fn classify_returns_ambiguous_case_label_when_duplicate_case_labels_are_present() {
    // Given
    let pattern = LoopPattern::new(
        "tests/ambiguous_label_loop.rs",
        location(5, 5),
        LoopPatternKind::TableLoop,
        2,
        LabelEvidence::DuplicateCaseLabel {
            label: case_label("invalid"),
            behavior: Some(behavior("parser rejects invalid ids")),
            case_count: 2,
        },
    );
    let policy = LabelingPolicy::RequireBehaviorAndCaseIdentity;

    // When
    let result = classify_loop_pattern(pattern, policy);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::AmbiguousCaseLabel {
            label: "invalid".to_owned(),
            behavior: Some("parser rejects invalid ids".to_owned()),
            case_count: 2
        })
    );
}

#[test]
fn classify_returns_ambiguous_case_label_when_behavior_identity_is_missing() {
    // Given
    let pattern = LoopPattern::new(
        "tests/case_only.rs",
        location(5, 5),
        LoopPatternKind::TableLoop,
        1,
        LabelEvidence::CaseOnly {
            case: case_label("case=empty"),
        },
    );
    let policy = LabelingPolicy::RequireBehaviorAndCaseIdentity;

    // When
    let result = classify_loop_pattern(pattern, policy);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::AmbiguousCaseLabel {
            label: "case=empty".to_owned(),
            behavior: None,
            case_count: 1
        })
    );
}

#[test]
fn classify_returns_safe_labeling_proven_when_every_assertion_has_behavior_and_case() {
    // Given
    let pattern = LoopPattern::new(
        "tests/safe_case_labeled_loop.rs",
        location(5, 5),
        LoopPatternKind::TableLoop,
        2,
        LabelEvidence::BehaviorAndCases {
            behavior: behavior("parser rejects invalid ids"),
            cases: cases(&["empty", "whitespace"]),
        },
    );
    let policy = LabelingPolicy::RequireBehaviorAndCaseIdentity;

    // When
    let result = classify_loop_pattern(pattern, policy);

    // Then
    assert_eq!(
        result,
        Ok(LoopRisk::SafeLabelingProven {
            finding_id: fid("tests/safe_case_labeled_loop.rs:5:5:TableLoop"),
            behavior_evidence: behavior("parser rejects invalid ids"),
            case_evidence: cases(&["empty", "whitespace"])
        })
    );
}

#[test]
fn classify_rejects_safe_labeling_when_case_evidence_is_empty() {
    // Given
    let values = Vec::new();

    // When
    let result = CaseEvidence::new(values);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::AmbiguousCaseLabel {
            label: String::new(),
            behavior: None,
            case_count: 0
        })
    );
}

#[test]
fn classify_returns_same_loop_risk_when_same_pattern_and_policy_are_reused() {
    // Given
    let pattern = weak_table_pattern();
    let policy = LabelingPolicy::RequireBehaviorAndCaseIdentity;

    // When
    let first = classify_loop_pattern(pattern.clone(), policy);
    let second = classify_loop_pattern(pattern, policy);

    // Then
    assert_eq!(first, second);
    assert_eq!(
        first,
        Ok(LoopRisk::Risky {
            finding_id: fid("tests/weak.rs:7:5:TableLoop"),
            reason: RiskReason::MissingCaseIdentity,
            required_action: DispositionKind::RepairRequired
        })
    );
}

#[test]
fn assign_returns_repair_required_when_repair_bead_evidence_is_complete() {
    // Given
    let risk = LoopRisk::Risky {
        finding_id: fid("tests/weak.rs:7:5:TableLoop"),
        reason: RiskReason::MissingCaseIdentity,
        required_action: DispositionKind::RepairRequired,
    };
    let evidence = AssignmentEvidence::RepairEvidence(RepairMetadata::new("Lewis", "vb-repair-1"));

    // When
    let result = assign_disposition(risk, evidence);

    // Then
    assert_eq!(
        result,
        Ok(Disposition::RepairRequired(RepairMetadata::new(
            "Lewis",
            "vb-repair-1"
        )))
    );
}

#[test]
fn assign_returns_accepted_exception_when_exception_metadata_is_complete() {
    // Given
    let risk = LoopRisk::Risky {
        finding_id: fid("tests/accepted_exception_loop.rs:7:5:TableLoop"),
        reason: RiskReason::AcceptedExceptionRequired,
        required_action: DispositionKind::AcceptedException,
    };
    let evidence = AssignmentEvidence::ExceptionEvidence(exception(
        "bounded smoke loop",
        "single deterministic fixture",
        "Lewis",
        "mutation refresh",
    ));

    // When
    let result = assign_disposition(risk, evidence);

    // Then
    assert_eq!(
        result,
        Ok(Disposition::AcceptedException(exception(
            "bounded smoke loop",
            "single deterministic fixture",
            "Lewis",
            "mutation refresh",
        )))
    );
}

#[test]
fn assign_returns_policy_violation_when_exception_owner_is_missing() {
    // Given
    let result = ExceptionMetadata::new(
        "bounded smoke loop",
        "single deterministic fixture",
        "",
        "mutation refresh",
    );

    // Then
    assert_eq!(
        result,
        Err(InventoryError::PolicyViolation {
            rule: "accepted_exception_metadata_complete".to_owned(),
            field: "owner".to_owned()
        })
    );
}

#[test]
fn assign_returns_policy_violation_when_exception_reason_is_missing() {
    // Given
    let result = ExceptionMetadata::new(
        "",
        "single deterministic fixture",
        "Lewis",
        "mutation refresh",
    );

    // Then
    assert_eq!(
        result,
        Err(InventoryError::PolicyViolation {
            rule: "accepted_exception_metadata_complete".to_owned(),
            field: "reason".to_owned()
        })
    );
}

#[test]
fn assign_returns_policy_violation_when_exception_scope_is_missing() {
    // Given
    let result = ExceptionMetadata::new("bounded smoke loop", "", "Lewis", "mutation refresh");

    // Then
    assert_eq!(
        result,
        Err(InventoryError::PolicyViolation {
            rule: "accepted_exception_metadata_complete".to_owned(),
            field: "scope".to_owned()
        })
    );
}

#[test]
fn assign_returns_policy_violation_when_exception_review_trigger_is_missing() {
    // Given
    let result = ExceptionMetadata::new(
        "bounded smoke loop",
        "single deterministic fixture",
        "Lewis",
        "",
    );

    // Then
    assert_eq!(
        result,
        Err(InventoryError::PolicyViolation {
            rule: "accepted_exception_metadata_complete".to_owned(),
            field: "review_trigger".to_owned()
        })
    );
}
