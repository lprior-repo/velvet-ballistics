use super::*;

#[test]
fn mutant_kills_assertion_only_applies_to_open_loop() {
    // Given
    let file = candidate_file("tests/non_assertion_for_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/non_assertion_for_loop.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/non_assertion_for_loop.rs",
            location(4, 5),
            LoopPatternKind::TableLoop,
            0,
            LabelEvidence::Absent,
        )])
    );
}

#[test]
fn mutant_kills_assertion_requires_open_matching_loop() {
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

#[test]
fn mutant_kills_non_assertion_lines_not_counted() {
    // Given
    let file = candidate_file("tests/non_assertion_for_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/non_assertion_for_loop.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/non_assertion_for_loop.rs",
            location(4, 5),
            LoopPatternKind::TableLoop,
            0,
            LabelEvidence::Absent,
        )])
    );
}

#[test]
fn mutant_kills_assertion_scope_line_matching() {
    // Given
    let file = candidate_file("tests/safe_case_labeled_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/safe_case_labeled_loop.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/safe_case_labeled_loop.rs",
            location(5, 5),
            LoopPatternKind::TableLoop,
            1,
            LabelEvidence::BehaviorAndCases {
                behavior: behavior("parser rejects invalid ids"),
                cases: cases(&["empty", "whitespace"])
            },
        )])
    );
}

#[test]
fn mutant_kills_multiple_assertions_count_exactly() {
    // Given
    let file = candidate_file("tests/two_assertions_for_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/two_assertions_for_loop.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/two_assertions_for_loop.rs",
            location(4, 5),
            LoopPatternKind::TableLoop,
            2,
            LabelEvidence::Absent,
        )])
    );
}

#[test]
fn mutant_kills_single_assertion_count_nonzero() {
    // Given
    let file = candidate_file("tests/inline_single_case_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/inline_single_case_loop.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/inline_single_case_loop.rs",
            location(2, 1),
            LoopPatternKind::TableLoop,
            1,
            LabelEvidence::AcceptedExceptionEvidence {
                reason: ExceptionReason::new("bounded smoke loop"),
                scope: ExceptionScope::new("single deterministic inline case")
            },
        )])
    );
}

#[test]
fn mutant_kills_label_evidence_requires_behavior_and_case_tokens() {
    // Given
    let behavior_file = candidate_file("tests/behavior_without_case_for_loop.rs");
    let behavior_text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/behavior_without_case_for_loop.txt")
            .to_owned(),
    );
    let case_file = candidate_file("tests/case_without_behavior_for_loop.rs");
    let case_text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/case_without_behavior_for_loop.txt")
            .to_owned(),
    );

    // When
    let behavior_result = scan_test_file(behavior_file, behavior_text);
    let case_result = scan_test_file(case_file, case_text);

    // Then
    assert_eq!(
        behavior_result,
        Ok(vec![LoopPattern::new(
            "tests/behavior_without_case_for_loop.rs",
            location(4, 5),
            LoopPatternKind::TableLoop,
            1,
            LabelEvidence::Absent,
        )])
    );
    assert_eq!(
        case_result,
        Ok(vec![LoopPattern::new(
            "tests/case_without_behavior_for_loop.rs",
            location(4, 5),
            LoopPatternKind::TableLoop,
            1,
            LabelEvidence::Absent,
        )])
    );
}

#[test]
fn mutant_kills_inline_single_case_exception_detection() {
    // Given
    let file = candidate_file("tests/inline_single_case_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/inline_single_case_loop.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/inline_single_case_loop.rs",
            location(2, 1),
            LoopPatternKind::TableLoop,
            1,
            LabelEvidence::AcceptedExceptionEvidence {
                reason: ExceptionReason::new("bounded smoke loop"),
                scope: ExceptionScope::new("single deterministic inline case")
            },
        )])
    );
}

#[test]
fn mutant_kills_inline_single_case_requires_one_assertion_and_inline_shape() {
    // Given
    let file = candidate_file("tests/inline_multi_case_loop.rs");
    let text = SourceText::Text(
        include_str!("../../fixtures/vb_5xs4_sources/inline_multi_case_loop.txt").to_owned(),
    );

    // When
    let result = scan_test_file(file, text);

    // Then
    assert_eq!(
        result,
        Ok(vec![LoopPattern::new(
            "tests/inline_multi_case_loop.rs",
            location(2, 1),
            LoopPatternKind::TableLoop,
            1,
            LabelEvidence::Absent,
        )])
    );
}
