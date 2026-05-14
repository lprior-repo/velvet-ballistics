use super::*;

#[test]
fn mutant_kills_root_rejection_before_discovery() {
    // Given
    let root = workspace_root("/tmp/vb-5xs4-mutant-missing-root");
    let scope = InventoryScope::Roots(vec!["../outside/sentinel_unreadable.rs".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InputRootOutOfScope {
            path: "../outside/sentinel_unreadable.rs".to_owned()
        })
    );
}

#[test]
fn mutant_kills_allowed_tests_and_crates_roots() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");

    // When
    let tests_result = discover_rust_test_files(
        root.clone(),
        InventoryScope::Roots(vec!["tests".to_owned()]),
    );
    let crates_result = discover_rust_test_files(
        root,
        InventoryScope::Roots(vec!["crates/core/tests".to_owned()]),
    );

    // Then
    assert_eq!(
        tests_result,
        Ok(vec![
            candidate_file("tests/subscope/real.rs"),
            candidate_file("tests/weak_loop.rs")
        ])
    );
    assert_eq!(
        crates_result,
        Ok(vec![candidate_file("crates/core/tests/loop_cases.rs")])
    );
}

#[test]
fn mutant_kills_allowed_root_requires_prefix_and_not_excluded() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");
    let scope = InventoryScope::Roots(vec!["src/lib.rs".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InputRootOutOfScope {
            path: "src/lib.rs".to_owned()
        })
    );
}

#[test]
fn mutant_kills_tests_or_crates_root_disjunction() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");
    let scope = InventoryScope::Roots(vec!["crates/core/tests".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Ok(vec![candidate_file("crates/core/tests/loop_cases.rs")])
    );
}

#[test]
fn mutant_kills_nested_tests_root_disjunction() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");
    let scope = InventoryScope::Roots(vec!["tests/subscope".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(result, Ok(vec![candidate_file("tests/subscope/real.rs")]));
}

#[test]
fn mutant_kills_vendor_root_exclusion() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");
    let scope = InventoryScope::Roots(vec!["tests/vendor".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InputRootOutOfScope {
            path: "tests/vendor".to_owned()
        })
    );
}

#[test]
fn mutant_kills_target_root_exclusion() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");
    let scope = InventoryScope::Roots(vec!["tests/target".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InputRootOutOfScope {
            path: "tests/target".to_owned()
        })
    );
}

#[test]
fn mutant_kills_generated_root_exclusion() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");
    let scope = InventoryScope::Roots(vec!["tests/generated".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InputRootOutOfScope {
            path: "tests/generated".to_owned()
        })
    );
}

#[test]
fn mutant_kills_external_root_exclusion() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");
    let scope = InventoryScope::Roots(vec!["tests/external".to_owned()]);

    // When
    let result = discover_rust_test_files(root, scope);

    // Then
    assert_eq!(
        result,
        Err(InventoryError::InputRootOutOfScope {
            path: "tests/external".to_owned()
        })
    );
}

#[test]
fn mutant_kills_excluded_directory_pruning() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");

    // When
    let result = discover_rust_test_files(root, InventoryScope::FirstPartyRustTests);

    // Then
    assert_eq!(
        result,
        Ok(vec![
            candidate_file("crates/core/tests/loop_cases.rs"),
            candidate_file("tests/subscope/real.rs"),
            candidate_file("tests/weak_loop.rs")
        ])
    );
}

#[test]
fn mutant_kills_first_party_test_rs_requires_scope_and_extension() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");

    // When
    let result = discover_rust_test_files(root, InventoryScope::FirstPartyRustTests);

    // Then
    assert_eq!(
        result,
        Ok(vec![
            candidate_file("crates/core/tests/loop_cases.rs"),
            candidate_file("tests/subscope/real.rs"),
            candidate_file("tests/weak_loop.rs")
        ])
    );
}

#[test]
fn mutant_kills_first_party_test_rs_rejects_non_rust_files_inside_tests_scope() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");

    // When
    let result = discover_rust_test_files(root, InventoryScope::FirstPartyRustTests);

    // Then
    assert_eq!(
        result,
        Ok(vec![
            candidate_file("crates/core/tests/loop_cases.rs"),
            candidate_file("tests/subscope/real.rs"),
            candidate_file("tests/weak_loop.rs")
        ])
    );
}

#[test]
fn mutant_kills_first_party_test_rs_rejects_crate_source_outside_tests_directory() {
    // Given
    let root = workspace_root("tests/fixtures/vb_5xs4/discovery_workspace");

    // When
    let result = discover_rust_test_files(root, InventoryScope::FirstPartyRustTests);

    // Then
    assert_eq!(
        result,
        Ok(vec![
            candidate_file("crates/core/tests/loop_cases.rs"),
            candidate_file("tests/subscope/real.rs"),
            candidate_file("tests/weak_loop.rs")
        ])
    );
}

#[test]
fn mutant_kills_nested_scope_closure() {
    // Given
    let file = candidate_file("tests/assertion_after_closed_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/assertion_after_closed_loop.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/assertion_after_closed_loop.rs",
            location(4, 5),
            LoopPatternKind::TableLoop,
            0,
            LabelEvidence::Absent,
        )])
    );
}
