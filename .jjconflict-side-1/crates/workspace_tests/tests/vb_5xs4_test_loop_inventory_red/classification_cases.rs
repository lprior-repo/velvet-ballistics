use super::*;

#[test]
fn assign_returns_safe_labeling_proven_when_behavior_and_case_evidence_are_complete() {
    // Given
    let risk = LoopRisk::SafeLabelingProven {
        finding_id: fid("tests/safe_case_labeled_loop.rs:5:5:TableLoop"),
        behavior_evidence: behavior("parser rejects invalid ids"),
        case_evidence: cases(&["empty"]),
    };
    let evidence = AssignmentEvidence::SafeLabelEvidence {
        behavior: behavior("parser rejects invalid ids"),
        cases: cases(&["empty"]),
    };

    // When
    let result = assign_disposition(risk, evidence);

    // Then
    assert_eq!(
        result,
        Ok(Disposition::SafeLabelingProven {
            behavior_evidence: behavior("parser rejects invalid ids"),
            case_evidence: cases(&["empty"])
        })
    );
}

#[test]
fn assign_returns_ambiguous_case_label_when_safe_proof_case_evidence_is_missing() {
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
fn validate_returns_validated_inventory_when_every_risky_finding_has_one_disposition() {
    // Given
    let inventory = Inventory::from_findings(vec![Finding::risky(
        "tests/weak.rs:7:5:TableLoop",
        RiskReason::MissingCaseIdentity,
        Some(Disposition::RepairRequired(RepairMetadata::new(
            "Lewis",
            "vb-repair-1",
        ))),
    )]);

    // When
    let result = validate_inventory(inventory);

    // Then
    assert_eq!(
        result,
        Ok(ValidatedInventory::summary(
            1,
            1,
            0,
            0,
            vec![fid("tests/weak.rs:7:5:TableLoop")]
        ))
    );
}

#[test]
fn validate_returns_unassigned_risky_pattern_when_risky_finding_has_no_disposition() {
    // Given
    let inventory = weak_inventory_without_disposition();

    // When
    let result = validate_inventory(inventory);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::UnassignedRiskyPattern {
            finding_id: "tests/weak.rs:7:5:TableLoop".to_owned()
        })
    );
}

#[test]
fn validate_returns_conflicting_disposition_when_repair_and_exception_are_both_present() {
    // Given
    let inventory = Inventory::from_findings(vec![Finding::risky_with_dispositions(
        "tests/weak.rs:7:5:TableLoop",
        RiskReason::MissingCaseIdentity,
        vec![
            Disposition::RepairRequired(RepairMetadata::new("Lewis", "vb-repair-1")),
            Disposition::AcceptedException(exception(
                "bounded smoke loop",
                "single deterministic fixture",
                "Lewis",
                "mutation refresh",
            )),
        ],
    )]);

    // When
    let result = validate_inventory(inventory);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::ConflictingDisposition {
            finding_id: "tests/weak.rs:7:5:TableLoop".to_owned(),
            dispositions: vec![
                DispositionKind::RepairRequired,
                DispositionKind::AcceptedException
            ]
        })
    );
}

#[test]
fn validate_returns_destructive_change_detected_when_baseline_finding_disappears() {
    // Given
    let inventory = Inventory::with_baseline_and_current(
        vec![fid("tests/deletion_baseline.rs:7:5:TableLoop")],
        vec![],
    );

    // When
    let result = validate_inventory(inventory);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::DestructiveChangeDetected {
            path: "tests/deletion_baseline.rs".to_owned(),
            previous_finding: "tests/deletion_baseline.rs:7:5:TableLoop".to_owned()
        })
    );
}

#[test]
fn validate_returns_unassigned_risky_pattern_when_non_risky_finding_is_also_present() {
    // Given
    let inventory = Inventory::from_findings(vec![
        Finding::non_risky("tests/mixed_risky_and_non_risky.rs:4:5:SafeLabeledLoop"),
        Finding::risky(
            "tests/mixed_risky_and_non_risky.rs:12:5:TableLoop",
            RiskReason::MissingCaseIdentity,
            None,
        ),
    ]);

    // When
    let result = validate_inventory(inventory);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::UnassignedRiskyPattern {
            finding_id: "tests/mixed_risky_and_non_risky.rs:12:5:TableLoop".to_owned()
        })
    );
}

#[test]
fn report_contains_exact_risky_finding_fields_when_repair_required_is_present() {
    // Given
    let inventory = inventory_with_findings(vec![ValidatedFinding::repair_required(
        FindingSummary::new(
            "tests/weak.rs",
            "7:5",
            LoopPatternKind::TableLoop,
            RiskReason::MissingCaseIdentity,
        ),
        RepairMetadata::new("Lewis", "vb-repair-1"),
    )]);

    // When
    let result = render_inventory_report(inventory);

    // Then
    assert_eq!(
        result,
        Ok(InventoryReport::from_findings(
            0,
            vec![ReportFinding::repair_required(
                FindingSummary::new(
                    "tests/weak.rs",
                    "7:5",
                    LoopPatternKind::TableLoop,
                    RiskReason::MissingCaseIdentity,
                ),
                RepairMetadata::new("Lewis", "vb-repair-1")
            )],
            MutationEvidence::NotProvided,
            None
        ))
    );
}

#[test]
fn report_contains_exact_exception_metadata_when_accepted_exception_is_present() {
    // Given
    let inventory = inventory_with_findings(vec![ValidatedFinding::accepted_exception(
        "tests/accepted_exception_loop.rs",
        "7:5",
        exception(
            "bounded smoke loop",
            "single deterministic fixture",
            "Lewis",
            "mutation refresh",
        ),
    )]);

    // When
    let result = render_inventory_report(inventory);

    // Then
    assert_eq!(
        result,
        Ok(InventoryReport::from_findings(
            0,
            vec![ReportFinding::accepted_exception(
                "tests/accepted_exception_loop.rs",
                "7:5",
                exception(
                    "bounded smoke loop",
                    "single deterministic fixture",
                    "Lewis",
                    "mutation refresh",
                )
            )],
            MutationEvidence::NotProvided,
            None
        ))
    );
}

#[test]
fn report_contains_exact_safe_label_evidence_when_safe_labeling_is_present() {
    // Given
    let inventory = inventory_with_findings(vec![safe_labeling_finding(
        "tests/safe_case_labeled_loop.rs",
        "5:5",
        "parser rejects invalid ids",
        vec!["empty".to_owned(), "whitespace".to_owned()],
    )]);

    // When
    let result = render_inventory_report(inventory);

    // Then
    assert_eq!(
        result,
        Ok(InventoryReport::from_findings(
            0,
            vec![ReportFinding::safe_labeling(
                "tests/safe_case_labeled_loop.rs",
                "5:5",
                "parser rejects invalid ids",
                vec!["empty".to_owned(), "whitespace".to_owned()]
            )],
            MutationEvidence::NotProvided,
            None
        ))
    );
}
