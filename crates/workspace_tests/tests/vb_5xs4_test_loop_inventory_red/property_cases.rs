use super::*;

proptest! {
    #[test]
    fn proptest_out_of_scope_roots_return_exact_input_root_error(path in prop::sample::select(vec!["../outside", "/tmp", "vendor/x.rs", "target/generated.rs"])) {
        // Given
        let root = workspace_root("/tmp/vb-5xs4-proptest-root");
        let scope = InventoryScope::Roots(vec![path.to_owned()]);

        // When
        let result = discover_rust_test_files(root, scope);

        // Then
        prop_assert_eq!(result, Err(InventoryError::InputRootOutOfScope { path: path.to_owned() }));
    }

    #[test]
    fn proptest_unlabeled_and_ambiguous_evidence_never_becomes_safe(evidence in prop::sample::select(vec![LabelEvidence::Absent, LabelEvidence::DuplicateCaseLabel { label: case_label("invalid"), behavior: Some(behavior("parser rejects invalid ids")), case_count: 2 }, LabelEvidence::CaseOnly { case: case_label("empty") }, LabelEvidence::BehaviorOnly { behavior: behavior("parser rejects invalid ids") }])) {
        // Given
        let pattern = LoopPattern::new("tests/generated.rs", location(7, 5), LoopPatternKind::TableLoop, 1, evidence);

        // When
        let result = classify_loop_pattern(pattern, LabelingPolicy::RequireBehaviorAndCaseIdentity);

        // Then
        prop_assert_ne!(result, Ok(LoopRisk::SafeLabelingProven { finding_id: fid("tests/generated.rs:7:5:TableLoop"), behavior_evidence: behavior("parser rejects invalid ids"), case_evidence: cases(&["empty"]) }));
    }

    #[test]
    fn proptest_non_risky_findings_cannot_suppress_unassigned_risky_finding(extra_safe_count in 0usize..16usize) {
        // Given
        let inventory = Inventory::with_non_risky_count_and_one_unassigned_risky(extra_safe_count, "tests/weak.rs:7:5:TableLoop");

        // When
        let result = validate_inventory(inventory);

        // Then
        prop_assert_eq!(result, Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/weak.rs:7:5:TableLoop".to_owned() }));
    }
}

// Kani proof obligations for State 3 remain in `.beads/vb-5xs4/test-plan.md`.
// They are intentionally not compiled in this integration-test target because
// the workspace has no configured `cfg(kani)` check-cfg contract.
