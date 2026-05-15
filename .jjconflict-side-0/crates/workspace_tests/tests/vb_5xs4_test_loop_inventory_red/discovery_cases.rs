use super::*;

#[test]
fn discover_returns_tests_and_crates_paths_when_scope_is_first_party_tests() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");
    let scope = InventoryScope::FirstPartyRustTests;

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Ok(vec![
            candidate_file("crates/core/tests/loop_cases.rs"),
            candidate_file("tests/subscope/real.rs"),
            candidate_file("tests/weak_loop.rs"),
        ])
    );
}

#[test]
fn discover_returns_input_root_out_of_scope_when_scope_contains_parent_escape() {
    // Given
    let root = workspace_root("/tmp/vb-5xs4-scope-root");
    let scope = InventoryScope::Roots(vec!["../outside".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InputRootOutOfScope {
            path: "../outside".to_owned(),
        })
    );
}

#[test]
fn discover_returns_input_root_out_of_scope_when_scope_contains_absolute_tmp() {
    // Given
    let root = workspace_root("/tmp/vb-5xs4-scope-root");
    let scope = InventoryScope::Roots(vec!["/tmp".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InputRootOutOfScope {
            path: "/tmp".to_owned(),
        })
    );
}

#[test]
fn discover_excludes_vendor_and_target_paths_when_not_whitelisted() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/vendor_generated_out_of_scope");
    let scope = InventoryScope::FirstPartyRustTests;

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(result, Ok(vec![candidate_file("crates/a/tests/real.rs")]));
}

#[test]
fn discover_returns_workspace_unreadable_when_root_marker_is_unreadable() {
    // Given
    let root = workspace_root("/tmp/vb-5xs4-missing-root");
    let scope = InventoryScope::FirstPartyRustTests;

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::WorkspaceUnreadable {
            root: "/tmp/vb-5xs4-missing-root".to_owned(),
        })
    );
}

#[test]
fn scan_returns_table_loop_pattern_when_for_loop_contains_unlabeled_assertion() {
    // Given
    let file = candidate_file("tests/weak_table_loop_missing_case_label.rs");
    let text = SourceText::Text(
        include_str!("../../../../fixtures/vb_5xs4_sources/weak_table_loop_missing_case_label.txt")
            .to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/weak_table_loop_missing_case_label.rs",
            location(6, 5),
            LoopPatternKind::TableLoop,
            1,
            LabelEvidence::Absent,
        )])
    );
}

#[test]
fn scan_returns_iterator_loop_pattern_when_for_each_contains_assertion() {
    // Given
    let file = candidate_file("tests/weak_iterator_for_each_missing_behavior.rs");
    let text = SourceText::Text(
        include_str!(
            "../../../../fixtures/vb_5xs4_sources/weak_iterator_for_each_missing_behavior.txt"
        )
        .to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/weak_iterator_for_each_missing_behavior.rs",
            location(3, 31),
            LoopPatternKind::IteratorTableLoop,
            1,
            LabelEvidence::Absent,
        )])
    );
}

#[test]
fn scan_returns_two_patterns_when_source_contains_nested_loops() {
    // Given
    let file = candidate_file("tests/nested_loops.rs");
    let text = SourceText::Text(
        include_str!("../../../../fixtures/vb_5xs4_sources/nested_loops.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![
            LoopPattern::new(
                "tests/nested_loops.rs",
                location(5, 5),
                LoopPatternKind::NestedOuterLoop,
                0,
                LabelEvidence::Absent
            ),
            LoopPattern::new(
                "tests/nested_loops.rs",
                location(6, 9),
                LoopPatternKind::NestedInnerLoop,
                1,
                LabelEvidence::Absent
            ),
        ])
    );
}

#[test]
fn scan_returns_invalid_utf8_when_source_text_is_invalid() {
    // Given
    let file = candidate_file("tests/invalid_utf8.rs");
    let text = SourceText::InvalidUtf8 { byte_offset: 3 };

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InvalidUtf8 {
            path: "tests/invalid_utf8.rs".to_owned(),
            byte_offset: 3,
        })
    );
}

#[test]
fn scan_returns_parse_failed_when_source_is_unrecoverable_rust() {
    // Given
    let file = candidate_file("tests/malformed_rust_unrecoverable.rs");
    let text = SourceText::Text(
        include_str!("../../../../fixtures/vb_5xs4_sources/malformed_rust_unrecoverable.txt")
            .to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::ParseFailed {
            path: "tests/malformed_rust_unrecoverable.rs".to_owned(),
            location: location(3, 1),
        })
    );
}

#[test]
fn scan_returns_file_read_failed_when_source_text_records_read_failure() {
    // Given
    let file = candidate_file("tests/unreadable_candidate.rs");
    let text = SourceText::ReadFailed {
        operation: "read_to_string".to_owned(),
    };

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::FileReadFailed {
            path: "tests/unreadable_candidate.rs".to_owned(),
            operation: "read_to_string".to_owned(),
        })
    );
}

#[test]
fn scan_returns_unsupported_generated_source_when_source_location_is_untraceable() {
    // Given
    let file = candidate_file("tests/untraceable_generated_loop.rs");
    let text = SourceText::Text(
        include_str!("../../../../fixtures/vb_5xs4_sources/untraceable_generated_loop.txt")
            .to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::UnsupportedGeneratedSource {
            path_or_macro: "untraceable_generated_loop".to_owned(),
            reason: "no_stable_first_party_location".to_owned(),
        })
    );
}

#[test]
fn classify_returns_repair_required_when_table_loop_has_no_case_label() {
    // Given
    let pattern = weak_table_pattern();
    let policy = LabelingPolicy::RequireBehaviorAndCaseIdentity;

    // When
    let result = classify_loop_pattern(pattern, policy);

    // Then
    assert_eq!(
        result,
        Ok(LoopRisk::Risky {
            finding_id: fid("tests/weak.rs:7:5:TableLoop"),
            reason: RiskReason::MissingCaseIdentity,
            required_action: DispositionKind::RepairRequired,
        })
    );
}
