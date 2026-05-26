// Proptest: YamlError event-stream span preservation
// PO-P03: YamlError event-stream errors (C4.2)
//
// Properties:
//  1. Parse-level errors constructed with span: Some(s) preserve the SourceSpan
//  2. span() method returns the correct source span
//  3. All span-bearing variants can be constructed with span: None
// Strategy: randomized u32 span values (converted to usize for SourceSpan)
//
// NOTE: All 16 span-bearing YamlError variants preserve their span field.
// The 4 limit variants (SourceTooLarge, NestingTooDeep, NodeLimitExceeded,
// EmptySource) have no span field.

use proptest::prelude::*;
use vb_yaml::YamlError;
use vb_yaml::source_map::SourceSpan;

/// Strategy for generating arbitrary SourceSpan values with bounded offsets.
fn source_span_strategy() -> impl Strategy<Value = SourceSpan> {
    // Use u32-sized bounds for practical testing (real YAML sources won't exceed 4 GiB)
    (
        0u32..=100u32,
        0u32..=100u32,
        1u32..=100u32,
        1u32..=100u32,
        1u32..=100u32,
        1u32..=100u32,
    )
        .prop_map(
            |(start_offset, end_offset, start_line, start_col, end_line, end_col)| {
                SourceSpan::new(
                    start_offset as usize,
                    end_offset as usize,
                    start_line as usize,
                    start_col as usize,
                    end_line as usize,
                    end_col as usize,
                )
            },
        )
}

/// Strategy for generating line numbers (usize, bounded).
fn line_strategy() -> impl Strategy<Value = usize> {
    (1u32..=1000u32).prop_map(|v| v as usize)
}

proptest! {
    #[test]
    fn parse_error_preserves_span(span in source_span_strategy(), line in line_strategy()) {
        let err = YamlError::ParseError {
            line,
            reason: Box::<str>::from("test parse error"),
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn anchor_alias_merge_preserves_span(span in source_span_strategy()) {
        let err = YamlError::AnchorAliasMerge { span: Some(span) };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn custom_tag_preserves_span(span in source_span_strategy()) {
        let err = YamlError::CustomTag {
            tag: Box::<str>::from("!test"),
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn binary_scalar_preserves_span(span in source_span_strategy()) {
        let err = YamlError::BinaryScalar { span: Some(span) };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn ambiguous_scalar_preserves_span(span in source_span_strategy()) {
        let err = YamlError::AmbiguousScalar {
            scalar: Box::<str>::from("scalar"),
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn duplicate_key_preserves_span(span in source_span_strategy()) {
        let err = YamlError::DuplicateKey {
            key: Box::<str>::from("duplicate key"),
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn unknown_field_preserves_span(span in source_span_strategy()) {
        let err = YamlError::UnknownField {
            field: Box::<str>::from("unknown"),
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn unsupported_trigger_preserves_span(span in source_span_strategy()) {
        let err = YamlError::UnsupportedTrigger {
            trigger: "test",
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn unsupported_feature_preserves_span(span in source_span_strategy()) {
        let err = YamlError::UnsupportedFeature {
            feature: "test",
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn scalar_too_long_preserves_span(span in source_span_strategy()) {
        // ScalarTooLong HAS a span field (was limit-only in earlier contract)
        let err = YamlError::ScalarTooLong {
            len: 100,
            max: 50,
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn sequence_too_long_preserves_span(span in source_span_strategy()) {
        let err = YamlError::SequenceTooLong {
            len: 100,
            max: 50,
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn mapping_too_large_preserves_span(span in source_span_strategy()) {
        let err = YamlError::MappingTooLarge {
            count: 100,
            max: 50,
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn forbidden_feature_preserves_span(span in source_span_strategy()) {
        let err = YamlError::ForbiddenFeature {
            detail: "test",
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn multiple_documents_preserves_span(span in source_span_strategy()) {
        let err = YamlError::MultipleDocuments {
            count: 2,
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn missing_field_preserves_span(span in source_span_strategy()) {
        let err = YamlError::MissingField {
            field: "test",
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

    #[test]
    fn field_shape_preserves_span(span in source_span_strategy()) {
        let err = YamlError::FieldShape {
            field: "test",
            expected: "string",
            span: Some(span),
        };
        prop_assert_eq!(err.span(), Some(span));
    }

}

/// All 20 YamlError variants can be constructed with span: None and
/// their .span() method correctly returns None.
/// Moved outside proptest! block because zero-parameter tests
/// are not supported by the proptest 1.11.0 macro fn arms.
#[test]
fn span_none_is_legal_for_all_variants() {
    // TF-VB-002 REPAIRED: Replace vacuous assert!(true) with exact .span()
    // assertions proving backward-compatibility (C4.3).
    // All 16 span-bearing variants constructed with span: None
    let e1 = YamlError::UnsupportedTrigger {
        trigger: "t",
        span: None,
    };
    let e2 = YamlError::UnsupportedFeature {
        feature: "f",
        span: None,
    };
    let e3 = YamlError::DuplicateKey {
        key: Box::<str>::from("k"),
        span: None,
    };
    let e4 = YamlError::AnchorAliasMerge { span: None };
    let e5 = YamlError::CustomTag {
        tag: Box::<str>::from("!t"),
        span: None,
    };
    let e6 = YamlError::BinaryScalar { span: None };
    let e7 = YamlError::MultipleDocuments {
        count: 2,
        span: None,
    };
    let e8 = YamlError::AmbiguousScalar {
        scalar: Box::<str>::from("s"),
        span: None,
    };
    let e9 = YamlError::ScalarTooLong {
        len: 100,
        max: 50,
        span: None,
    };
    let e10 = YamlError::SequenceTooLong {
        len: 100,
        max: 50,
        span: None,
    };
    let e11 = YamlError::MappingTooLarge {
        count: 100,
        max: 50,
        span: None,
    };
    let e12 = YamlError::UnknownField {
        field: Box::<str>::from("f"),
        span: None,
    };
    let e13 = YamlError::MissingField {
        field: "f",
        span: None,
    };
    let e14 = YamlError::FieldShape {
        field: "f",
        expected: "e",
        span: None,
    };
    let e15 = YamlError::ParseError {
        line: 1,
        reason: Box::<str>::from("r"),
        span: None,
    };
    let e16 = YamlError::ForbiddenFeature {
        detail: "d",
        span: None,
    };

    // Limit variants without span field (their span() always returns None)
    let e17 = YamlError::SourceTooLarge { size: 100, max: 50 };
    let e18 = YamlError::NestingTooDeep { depth: 10, max: 5 };
    let e19 = YamlError::NodeLimitExceeded {
        count: 100,
        max: 50,
    };
    let e20 = YamlError::EmptySource;

    // All 20 variants constructed; now assert span() returns None for each
    let errors: [&YamlError; 20] = [
        &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15, &e16,
        &e17, &e18, &e19, &e20,
    ];
    for error in errors {
        assert_eq!(
            error.span(),
            None,
            "variant {error} constructed with span: None must return None from span()"
        );
    }
}
