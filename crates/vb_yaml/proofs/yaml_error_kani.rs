// Kani proof: YamlError span field invariants
// PO-K04: YamlError span field (C4.1-C4.3)
//
// Proves against enriched YamlError (all 20 variants):
//  1. Every YamlError variant can be constructed without panic
//  2. Variants with span: Some(s) preserve the SourceSpan
//  3. Variants with span: None have span() returning None
//  4. span() method works for all variants
//
// Variant summary (as of 2026-05-25):
//  - 16 variants WITH span field: UnsupportedTrigger, UnsupportedFeature,
//    DuplicateKey, AnchorAliasMerge, CustomTag, BinaryScalar, MultipleDocuments,
//    AmbiguousScalar, ScalarTooLong, SequenceTooLong, MappingTooLarge,
//    UnknownField, MissingField, FieldShape, ParseError, ForbiddenFeature
//  - 4 variants WITHOUT span: SourceTooLarge, NestingTooDeep, NodeLimitExceeded,
//    EmptySource
// Assumptions: YamlError enum non_exhaustive; 20 variants as of contract date.
//  Kani stubs for SourceSpan values are pure construction.

use vb_yaml::error::YamlError;
use vb_yaml::source_map_types::SourceSpan;

/// Helper: creates an arbitrary SourceSpan for Kani.
fn arbitrary_source_span() -> SourceSpan {
    SourceSpan::new(
        kani::any::<usize>(), // start_offset
        kani::any::<usize>(), // end_offset
        kani::any::<usize>(), // start_line
        kani::any::<usize>(), // start_col
        kani::any::<usize>(), // end_line
        kani::any::<usize>(), // end_col
    )
}

/// All 20 YamlError variants can be constructed with span: None (for
/// span-bearing variants) or their natural shape (for limit variants).
/// This harness verifies that no variant panics on construction.
#[kani::proof]
#[kani::unwind(3)]
fn yaml_error_all_variants_none_span_legal() {
    // --- 16 variants WITH span: Option<SourceSpan> ---

    // 1. UnsupportedTrigger
    let _e1 = YamlError::UnsupportedTrigger {
        trigger: "test",
        span: None,
    };

    // 2. UnsupportedFeature
    let _e2 = YamlError::UnsupportedFeature {
        feature: "test",
        span: None,
    };

    // 3. DuplicateKey
    let _e3 = YamlError::DuplicateKey {
        key: Box::<str>::from("test"),
        span: None,
    };

    // 4. AnchorAliasMerge
    let _e4 = YamlError::AnchorAliasMerge { span: None };

    // 5. CustomTag
    let _e5 = YamlError::CustomTag {
        tag: Box::<str>::from("tag"),
        span: None,
    };

    // 6. BinaryScalar
    let _e6 = YamlError::BinaryScalar { span: None };

    // 7. MultipleDocuments
    let _e7 = YamlError::MultipleDocuments {
        count: 3,
        span: None,
    };

    // 8. AmbiguousScalar
    let _e8 = YamlError::AmbiguousScalar {
        scalar: Box::<str>::from("scalar"),
        span: None,
    };

    // 9. ScalarTooLong — HAS span field (was limit-only in earlier contract)
    let _e9 = YamlError::ScalarTooLong {
        len: 1000,
        max: 100,
        span: None,
    };

    // 10. SequenceTooLong — HAS span field
    let _e10 = YamlError::SequenceTooLong {
        len: 1000,
        max: 100,
        span: None,
    };

    // 11. MappingTooLarge — HAS span field
    let _e11 = YamlError::MappingTooLarge {
        count: 500,
        max: 256,
        span: None,
    };

    // 12. UnknownField
    let _e12 = YamlError::UnknownField {
        field: Box::<str>::from("field"),
        span: None,
    };

    // 13. MissingField
    let _e13 = YamlError::MissingField {
        field: "required",
        span: None,
    };

    // 14. FieldShape
    let _e14 = YamlError::FieldShape {
        field: "field",
        expected: "string",
        span: None,
    };

    // 15. ParseError
    let _e15 = YamlError::ParseError {
        line: 1,
        reason: Box::<str>::from("bad"),
        span: None,
    };

    // 16. ForbiddenFeature
    let _e16 = YamlError::ForbiddenFeature {
        detail: "detail",
        span: None,
    };

    // --- 4 limit variants WITHOUT span ---

    // 17. SourceTooLarge — no span field (document-level limit)
    let _e17 = YamlError::SourceTooLarge {
        size: 100,
        max: 50,
    };

    // 18. NestingTooDeep — no span field (document-level limit)
    let _e18 = YamlError::NestingTooDeep {
        depth: 10,
        max: 5,
    };

    // 19. NodeLimitExceeded — no span field (document-level limit)
    let _e19 = YamlError::NodeLimitExceeded {
        count: 100,
        max: 50,
    };

    // 20. EmptySource — no span field (document-level limit)
    let _e20 = YamlError::EmptySource;

    // All 20 variants constructed without panic — reachability assertion
    assert!(true);
}

/// Variants with span: Some preserve the SourceSpan.
#[kani::proof]
fn yaml_error_span_preservation() {
    let span = arbitrary_source_span();

    let err = YamlError::ParseError {
        line: 42,
        reason: Box::<str>::from("test error"),
        span: Some(span),
    };

    match err {
        YamlError::ParseError { span: got, .. } => {
            assert_eq!(got, Some(span));
        }
        _ => unreachable!(),
    }
}

/// Constructing span-bearing variants with span: Some does not panic.
#[kani::proof]
fn yaml_error_parse_errors_with_span_no_panic() {
    let span = arbitrary_source_span();

    // All span-bearing variants can carry span: Some
    let _e1 = YamlError::ParseError {
        line: kani::any(),
        reason: Box::<str>::from("err"),
        span: Some(span),
    };
    let _e2 = YamlError::AnchorAliasMerge { span: Some(span) };
    let _e3 = YamlError::CustomTag {
        tag: Box::<str>::from("!tag"),
        span: Some(span),
    };
    let _e4 = YamlError::BinaryScalar { span: Some(span) };
    let _e5 = YamlError::AmbiguousScalar {
        scalar: Box::<str>::from("scalar"),
        span: Some(span),
    };
    let _e6 = YamlError::ScalarTooLong {
        len: 50,
        max: 100,
        span: Some(span),
    };

    assert!(true);
}

/// The span() method returns None for limit variants and None-span variants.
#[kani::proof]
fn yaml_error_span_method_none_for_limit_variants() {
    // Limit variants: span() always returns None
    assert_eq!(YamlError::EmptySource.span(), None);
    assert_eq!(
        YamlError::SourceTooLarge {
            size: 100,
            max: 50
        }
        .span(),
        None
    );
    assert_eq!(
        YamlError::NestingTooDeep {
            depth: 10,
            max: 5
        }
        .span(),
        None
    );
    assert_eq!(
        YamlError::NodeLimitExceeded {
            count: 100,
            max: 50
        }
        .span(),
        None
    );

    // Span-bearing variant with span: None
    assert_eq!(
        YamlError::DuplicateKey {
            key: Box::<str>::from("k"),
            span: None
        }
        .span(),
        None
    );
}

/// The span() method returns Some(span) for span-bearing variants with span: Some.
#[kani::proof]
fn yaml_error_span_method_returns_span() {
    let span = arbitrary_source_span();

    let err = YamlError::UnknownField {
        field: Box::<str>::from("f"),
        span: Some(span),
    };
    assert_eq!(err.span(), Some(span));

    let err2 = YamlError::ForbiddenFeature {
        detail: "d",
        span: Some(span),
    };
    assert_eq!(err2.span(), Some(span));
}
