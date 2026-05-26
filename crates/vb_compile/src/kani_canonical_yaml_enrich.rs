// Kani proof: canonical_yaml_error construction and YamlError category/span
// PO-K05: Canonical YAML span preservation (C5.1-C5.3)
//
// Implementation status:
//  - canonical_yaml_error() creates CompileError::CanonicalYaml { category, message }
//  - yaml_error_category() covers all 20 YamlError variants
//  - YamlError::span() returns Option<SourceSpan>
//  - The span bridge (clamp_u32, span_from_source_span) is implemented (PO-K07)
//
// What is VERIFIED:
//  1. canonical_yaml_error() never panics for common YamlError variants
//  2. yaml_error_category() yields correct categories for all 20 variants
//  3. YamlError::span() returns correct Option<SourceSpan> for variants with/without span
//  4. CompileError::CanonicalYaml is structurally stable
//
// What is PENDING IMPLEMENTATION:
//  - CompileError::CanonicalYaml does not carry a SourceMark field;
//    span propagation from YamlError into CanonicalYaml is not yet done.
//  - When a mark field is added, the span bridge (PO-K07) can feed it.

#![forbid(unsafe_code)]

use crate::CompileError;
use crate::mod_compile_validation::{canonical_yaml_error, yaml_error_category};
use vb_yaml::YamlError;

// ---------------------------------------------------------------------------
// Panic-freedom: canonical_yaml_error never panics
// ---------------------------------------------------------------------------

/// canonical_yaml_error produces CompileError::CanonicalYaml without panicking.
#[kani::proof]
fn canonical_yaml_error_no_panic() {
    // Test common variants with span: None
    let err = YamlError::DuplicateKey {
        key: Box::<str>::from("test"),
        span: None,
    };
    let compile_err = canonical_yaml_error(err);
    match compile_err {
        CompileError::CanonicalYaml {
            category, message, ..
        } => {
            assert!(!category.is_empty());
            assert!(!message.is_empty());
        }
        _ => assert!(false, "canonical_yaml_error must produce CanonicalYaml"),
    }

    // Test limit variant (no span)
    let err2 = YamlError::EmptySource;
    let compile_err2 = canonical_yaml_error(err2);
    match compile_err2 {
        CompileError::CanonicalYaml {
            category, message, ..
        } => {
            assert!(!category.is_empty());
            assert!(!message.is_empty());
        }
        _ => assert!(false, "canonical_yaml_error must produce CanonicalYaml"),
    }

    // Test ParseError (carries optional span)
    let err3 = YamlError::ParseError {
        line: 42,
        reason: Box::<str>::from("bad yaml"),
        span: None,
    };
    let compile_err3 = canonical_yaml_error(err3);
    match compile_err3 {
        CompileError::CanonicalYaml {
            category, message, ..
        } => {
            assert!(!category.is_empty());
            assert!(!message.is_empty());
        }
        _ => assert!(false, "canonical_yaml_error must produce CanonicalYaml"),
    }

    // Test UnsupportedTrigger (carries optional span)
    let err4 = YamlError::UnsupportedTrigger {
        trigger: "x",
        span: None,
    };
    let compile_err4 = canonical_yaml_error(err4);
    match compile_err4 {
        CompileError::CanonicalYaml { .. } => {}
        _ => assert!(false, "canonical_yaml_error must produce CanonicalYaml"),
    }
}

// ---------------------------------------------------------------------------
// Category classification: all 20 variants (split into 5 groups to keep
// per-harness heap allocations within Kani's verification limits).
// ---------------------------------------------------------------------------

/// yaml_error_category returns static string references, so pointer comparison
/// is equivalent to content equality and avoids Kani memcmp limitations.
#[kani::proof]
fn yaml_error_category_forbidden_feature_a() {
    let expected = "forbidden_feature";
    assert!(
        yaml_error_category(&YamlError::UnsupportedTrigger {
            trigger: "t",
            span: None
        })
        .as_ptr()
            == expected.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::UnsupportedFeature {
            feature: "f",
            span: None
        })
        .as_ptr()
            == expected.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::AnchorAliasMerge { span: None }).as_ptr()
            == expected.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::CustomTag {
            tag: Box::<str>::from("t"),
            span: None
        })
        .as_ptr()
            == expected.as_ptr()
    );
}

/// Pointer comparison for forbidden_feature + duplicate_key.
#[kani::proof]
fn yaml_error_category_forbidden_feature_b() {
    let ff = "forbidden_feature";
    let dk = "duplicate_key";
    assert!(yaml_error_category(&YamlError::BinaryScalar { span: None }).as_ptr() == ff.as_ptr());
    assert!(
        yaml_error_category(&YamlError::AmbiguousScalar {
            scalar: Box::<str>::from("s"),
            span: None
        })
        .as_ptr()
            == ff.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::ForbiddenFeature {
            detail: "d",
            span: None
        })
        .as_ptr()
            == ff.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::DuplicateKey {
            key: Box::<str>::from("k"),
            span: None
        })
        .as_ptr()
            == dk.as_ptr()
    );
}

/// Pointer comparison for document_count + limit_exceeded.
#[kani::proof]
fn yaml_error_category_limit_group_a() {
    let dc = "document_count";
    let le = "limit_exceeded";
    assert!(
        yaml_error_category(&YamlError::MultipleDocuments {
            count: 2,
            span: None
        })
        .as_ptr()
            == dc.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::SourceTooLarge { size: 100, max: 50 }).as_ptr()
            == le.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::NestingTooDeep { depth: 10, max: 5 }).as_ptr()
            == le.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::NodeLimitExceeded {
            count: 100,
            max: 50
        })
        .as_ptr()
            == le.as_ptr()
    );
}

/// Pointer comparison for limit_exceeded + empty_source.
#[kani::proof]
fn yaml_error_category_limit_group_b() {
    let le = "limit_exceeded";
    let es = "empty_source";
    assert!(
        yaml_error_category(&YamlError::ScalarTooLong {
            len: 100,
            max: 50,
            span: None
        })
        .as_ptr()
            == le.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::SequenceTooLong {
            len: 100,
            max: 50,
            span: None
        })
        .as_ptr()
            == le.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::MappingTooLarge {
            count: 100,
            max: 50,
            span: None
        })
        .as_ptr()
            == le.as_ptr()
    );
    assert!(yaml_error_category(&YamlError::EmptySource).as_ptr() == es.as_ptr());
}

/// Pointer comparison for unknown_field, missing_field, field_shape, parse_error.
#[kani::proof]
fn yaml_error_category_misc() {
    let uf = "unknown_field";
    let mf = "missing_field";
    let fs = "field_shape";
    let pe = "parse_error";
    assert!(
        yaml_error_category(&YamlError::UnknownField {
            field: Box::<str>::from("f"),
            span: None
        })
        .as_ptr()
            == uf.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::MissingField {
            field: "f",
            span: None
        })
        .as_ptr()
            == mf.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::FieldShape {
            field: "f",
            expected: "e",
            span: None
        })
        .as_ptr()
            == fs.as_ptr()
    );
    assert!(
        yaml_error_category(&YamlError::ParseError {
            line: 1,
            reason: Box::<str>::from("r"),
            span: None
        })
        .as_ptr()
            == pe.as_ptr()
    );
}

// ---------------------------------------------------------------------------
// YamlError::span() returns correct option for each variant
// ---------------------------------------------------------------------------

/// YamlError::span() returns None for span-less variants (limit errors).
#[kani::proof]
fn yaml_error_span_is_none_for_limit_variants() {
    assert!(
        YamlError::SourceTooLarge { size: 1, max: 10 }
            .span()
            .is_none()
    );
    assert!(
        YamlError::NestingTooDeep { depth: 1, max: 5 }
            .span()
            .is_none()
    );
    assert!(
        YamlError::NodeLimitExceeded { count: 1, max: 10 }
            .span()
            .is_none()
    );
    assert!(YamlError::EmptySource.span().is_none());
}

/// YamlError::span() returns Some for span-carrying variants.
#[kani::proof]
fn yaml_error_span_is_some_for_span_variants() {
    use vb_yaml::source_map::SourceSpan;
    let ss = SourceSpan::new(0, 10, 1, 1, 1, 10);

    assert!(
        YamlError::DuplicateKey {
            key: Box::<str>::from("k"),
            span: Some(ss)
        }
        .span()
        .is_some()
    );

    assert!(
        YamlError::ParseError {
            line: 1,
            reason: Box::<str>::from("r"),
            span: Some(ss)
        }
        .span()
        .is_some()
    );
}
