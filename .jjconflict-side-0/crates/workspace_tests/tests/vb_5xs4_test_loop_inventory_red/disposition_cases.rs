use super::*;

#[test]
fn report_contains_zero_findings_and_no_mutation_claim_when_inventory_is_empty() {
    // Given
    let inventory = inventory_with_findings(vec![]);

    // When
    let result = render_inventory_report(inventory);

    // Then
    assert_eq!(
        result,
        Ok(InventoryReport::from_findings(
            0,
            vec![],
            MutationEvidence::NotProvided,
            None
        ))
    );
}

#[test]
fn report_returns_policy_violation_when_runtime_policy_violation_record_is_rendered_as_success() {
    // Given
    let inventory = ValidatedInventory::with_policy_violation(
        "policy_violations_cannot_render_success",
        "report.status",
    );

    // When
    let result = render_inventory_report(inventory);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::PolicyViolation {
            rule: "policy_violations_cannot_render_success".to_owned(),
            field: "report.status".to_owned()
        })
    );
}

#[test]
fn bdd_contract_functions_return_expected_report_for_exact_fixture_workspace() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/composition_workspace");

    // When
    let files = discover_rust_test_files(root, InventoryScope::FirstPartyRustTests);

    // Then
    assert_eq!(
        files,
        Ok(vec![
            candidate_file("crates/core/tests/safe_case_labeled_loop.rs"),
            candidate_file("tests/accepted_exception_loop.rs"),
            candidate_file("tests/helper_driven_table_cases.rs"),
            candidate_file("tests/weak_iterator_for_each_missing_behavior.rs"),
            candidate_file("tests/weak_table_loop_missing_case_label.rs")
        ])
    );
}

#[test]
fn mutant_kills_symbolic_disposition_validator_deleted() {
    // Given
    let selection = DispositionSelection::Missing;

    // When
    let result = Inventory::symbolic_disposition_validate(selection);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::ConflictingDisposition {
            finding_id: String::new(),
            dispositions: vec![]
        })
    );
}

#[test]
fn mutant_kills_symbolic_disposition_equality_inversion() {
    // Given
    let selection = DispositionSelection::Single(DispositionKind::RepairRequired);

    // When
    let result = Inventory::symbolic_disposition_validate(selection);

    // Then
    assert_eq!(result, Ok(()));
}

#[test]
fn mutant_kills_symbolic_safe_label_validator_deleted() {
    // Given
    let input = SafeLabelInput::MissingBehavior { case_count: 1 };

    // When
    let result = AssignmentEvidence::symbolic_safe_label_validate(input);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::AmbiguousCaseLabel {
            label: String::new(),
            behavior: None,
            case_count: 1
        })
    );
}

#[test]
fn mutant_kills_safe_label_requires_behavior_and_case_conjunction() {
    // Given
    let input = SafeLabelInput::MissingCase {
        behavior: behavior("parser rejects invalid ids"),
    };

    // When
    let result = AssignmentEvidence::symbolic_safe_label_validate(input);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::AmbiguousCaseLabel {
            label: "parser rejects invalid ids".to_owned(),
            behavior: Some("parser rejects invalid ids".to_owned()),
            case_count: 0
        })
    );
}

#[test]
fn mutant_kills_complete_safe_label_requires_nonempty_cases() {
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
fn mutant_kills_complete_safe_label_accepts_nonempty_cases() {
    // Given
    let input = SafeLabelInput::Complete {
        behavior: behavior("parser rejects invalid ids"),
        cases: cases(&["empty"]),
    };

    // When
    let result = AssignmentEvidence::symbolic_safe_label_validate(input);

    // Then
    assert_eq!(result, Ok(()));
}

#[test]
fn mutant_kills_symbolic_safe_label_rejects_empty_case_evidence_guard() {
    // Given / When
    let result = CaseEvidence::new(Vec::new());

    // Then
    assert_eq!(result, Err(empty_case_error(None)));
}

#[test]
fn mutant_kills_disposition_contract_validation_deleted() {
    // Given
    let finding = invalid_exception_finding(invalid_exception_with_empty_reason());

    // When
    let result =
        ValidatedInventory::with_findings(vec![finding], MutationEvidence::NotProvided, None);

    // Then
    assert_eq!(result, Err(exception_policy_error("reason")));
}

#[test]
fn mutant_kills_validated_finding_validation_deleted() {
    // Given
    let finding = invalid_exception_finding(invalid_exception_with_empty_scope());

    // When
    let result =
        ValidatedInventory::with_findings(vec![finding], MutationEvidence::NotProvided, None);

    // Then
    assert_eq!(result, Err(exception_policy_error("scope")));
}

#[test]
fn mutant_kills_exception_metadata_validation_deleted() {
    // Given
    let finding = invalid_exception_finding(invalid_exception_with_empty_owner());

    // When
    let result =
        ValidatedInventory::with_findings(vec![finding], MutationEvidence::NotProvided, None);

    // Then
    assert_eq!(result, Err(exception_policy_error("owner")));
}

#[test]
fn mutant_kills_case_evidence_validation_deleted() {
    // Given / When
    let result = ValidatedFinding::safe_labeling(
        "tests/weak.rs",
        "7:5",
        "parser rejects empty ids",
        Vec::new(),
    );

    // Then
    assert_eq!(result, Err(empty_case_error(None)));
}

#[test]
fn mutant_kills_first_empty_exception_field_detection() {
    // Given
    let reason = invalid_exception_inventory_result(invalid_exception_with_empty_reason());
    let scope = invalid_exception_inventory_result(invalid_exception_with_empty_scope());
    let owner = invalid_exception_inventory_result(invalid_exception_with_empty_owner());
    let trigger = invalid_exception_inventory_result(invalid_exception_with_empty_trigger());

    // Then
    assert_eq!(reason, Err(exception_policy_error("reason")));
    assert_eq!(scope, Err(exception_policy_error("scope")));
    assert_eq!(owner, Err(exception_policy_error("owner")));
    assert_eq!(trigger, Err(exception_policy_error("review_trigger")));
}
