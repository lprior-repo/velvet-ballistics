use super::*;

#[test]
fn mutant_kills_inline_single_case_rejects_multiline_body() {
    // Given
    let file = candidate_file("tests/weak_table_loop_missing_case_label.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/weak_table_loop_missing_case_label.txt")
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
fn mutant_kills_inline_single_case_rejects_missing_assertion() {
    // Given
    let file = candidate_file("tests/inline_no_assertion_single_case_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/inline_no_assertion_single_case_loop.txt")
            .to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/inline_no_assertion_single_case_loop.rs",
            location(2, 1),
            LoopPatternKind::TableLoop,
            0,
            LabelEvidence::Absent,
        )])
    );
}

#[test]
fn mutant_kills_baseline_current_set_equality() {
    // Given
    let inventory = Inventory::with_baseline_and_current(
        vec![fid("tests/weak.rs:7:5:TableLoop")],
        vec![fid("tests/weak.rs:7:5:TableLoop")],
    );

    // When
    let result = validate_inventory(inventory);

    // Then
    assert_eq!(result, Ok(ValidatedInventory::summary(0, 0, 0, 0, vec![])));
}
