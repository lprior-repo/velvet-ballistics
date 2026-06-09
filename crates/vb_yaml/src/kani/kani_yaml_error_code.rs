#![forbid(unsafe_code)]
//! PO-006: Kani harness verifying that every `YamlError` variant,
//! when constructed with arbitrary field values, returns a
//! `SymbolicCode` that is registered in `vb_core::CODE_REGISTRY`
//! (i.e., `SymbolicCode::from_static` returns `Some`).
//!
//! GOD RULE 1 compliance:
//! - Variant selector is `kani::any::<u8>()`, bounded via `kani::assume`.
//! - Every field is `kani::any::<T>()` for its concrete type.
//! - No hardcoded `YamlError` literals; no fixed `variants: [YamlError; N]`
//!   array of dummy values.
//! - Uses the production `YamlError` and `HasSymbolicCode` impl from
//!   `vb_yaml::error`, not a parallel mini-model.
//!
//! Bound: 21 YamlError variants (unwind=21).
//!
//! NOTE: This harness uses `String` + `.leak()` to produce owned
//! `&'static str` references (see kani_all_variants_registered.rs
//! comment R-003). The leak is bounded by `kani::assume` on string
//! length so Kani tractability is preserved.

use crate::YamlError;
use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};

/// Total number of `YamlError` variants in the production enum
/// (`crates/vb_yaml/src/error.rs`). 21 as of vb-ymlkn01:
/// UnsupportedTrigger, UnsupportedFeature, DuplicateKey, AnchorAliasMerge,
/// CustomTag, BinaryScalar, MultipleDocuments, AmbiguousScalar,
/// SourceTooLarge, NestingTooDeep, NodeLimitExceeded, ScalarTooLong,
/// SequenceTooLong, MappingTooLarge, UnknownField, EmptySource,
/// MissingField, FieldShape, ParseError, ForbiddenFeature, LegacyPrimitive.
const YAML_ERROR_VARIANT_COUNT: u8 = 21;

/// Upper bound on the length of an arbitrary generated string field.
/// Kani generates bytes up to this length; kept small for tractability.
const MAX_KANI_STRING_LEN: usize = 32;

/// Construct an arbitrary `YamlError` whose variant is selected by
/// `variant` (a symbolic byte in `[0, YAML_ERROR_VARIANT_COUNT)`)
/// and whose fields are all `kani::any::<T>()` values for the
/// field's concrete type.
///
/// Every field is symbolic — no constants, no dummy strings, no
/// hardcoded counters. This is the GOD RULE 1 fix.
fn arbitrary_yaml_error(variant: u8) -> YamlError {
    match variant {
        0 => YamlError::DuplicateKey {
            key: kani::any::<String>().into_boxed_str(),
        },
        1 => YamlError::ForbiddenFeature {
            detail: static_str_from_any(),
        },
        2 => YamlError::AnchorAliasMerge,
        3 => YamlError::CustomTag {
            tag: kani::any::<String>().into_boxed_str(),
        },
        4 => YamlError::BinaryScalar,
        5 => YamlError::MultipleDocuments {
            count: kani::any::<usize>(),
        },
        6 => YamlError::AmbiguousScalar {
            scalar: kani::any::<String>().into_boxed_str(),
        },
        7 => YamlError::SourceTooLarge {
            size: kani::any::<usize>(),
            max: kani::any::<usize>(),
        },
        8 => YamlError::NestingTooDeep {
            depth: kani::any::<u16>(),
            max: kani::any::<u16>(),
        },
        9 => YamlError::NodeLimitExceeded {
            count: kani::any::<u32>(),
            max: kani::any::<u32>(),
        },
        10 => YamlError::ScalarTooLong {
            len: kani::any::<usize>(),
            max: kani::any::<usize>(),
        },
        11 => YamlError::SequenceTooLong {
            len: kani::any::<usize>(),
            max: kani::any::<usize>(),
        },
        12 => YamlError::MappingTooLarge {
            count: kani::any::<usize>(),
            max: kani::any::<usize>(),
        },
        13 => YamlError::UnknownField {
            field: kani::any::<String>().into_boxed_str(),
        },
        14 => YamlError::EmptySource,
        15 => YamlError::MissingField {
            field: static_str_from_any(),
        },
        16 => YamlError::FieldShape {
            field: static_str_from_any(),
            expected: static_str_from_any(),
        },
        17 => YamlError::ParseError {
            line: kani::any::<usize>(),
            reason: kani::any::<String>().into_boxed_str(),
        },
        18 => YamlError::UnsupportedFeature {
            feature: static_str_from_any(),
        },
        19 => YamlError::UnsupportedTrigger {
            trigger: static_str_from_any(),
        },
        20 => YamlError::LegacyPrimitive {
            primitive: static_str_from_any(),
            canonical: static_str_from_any(),
        },
        // Compile-time exhaustiveness: if `variant >= YAML_ERROR_VARIANT_COUNT`
        // the precondition is violated (kani::assume restricts to [0, 21)).
        _ => YamlError::EmptySource,
    }
}

/// Generate a bounded arbitrary string and leak it to a `&'static str`.
///
/// The string is bounded by `kani::assume(len <= MAX_KANI_STRING_LEN)`
/// so Kani does not blow up exploring huge `String` values. The leak
/// produces a valid `&'static str` reference for the lifetime of
/// the harness (R-003 from kani_all_variants_registered.rs).
fn static_str_from_any() -> &'static str {
    let s: String = kani::any::<String>();
    kani::assume(s.len() <= MAX_KANI_STRING_LEN);
    Box::leak(s.into_boxed_str())
}

/// PO-006: For every YamlError variant, with arbitrary field values,
/// `HasSymbolicCode::symbolic_code` returns a `SymbolicCode` that is
/// registered in `vb_core::CODE_REGISTRY` and is not the
/// `INTERNAL_INVARIANT` sentinel.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(21)]
fn kani_yaml_error_code_registered() {
    let variant: u8 = kani::any();
    // Restrict the symbolic byte to a valid variant index.
    kani::assume(variant < YAML_ERROR_VARIANT_COUNT);

    let error = arbitrary_yaml_error(variant);
    let code = error.symbolic_code();

    // The returned code must be a registered SymbolicCode: from_static
    // returns Some only for names that appear in vb_core::CODE_REGISTRY.
    let registered = SymbolicCode::from_static(code.as_str());
    assert!(
        registered.is_some(),
        "YamlError variant {}: symbolic_code '{}' is not registered in CODE_REGISTRY",
        variant,
        code.as_str()
    );

    // The code must never be the INTERNAL_INVARIANT sentinel; that
    // would mean a variant fell through to the unreachable fallback in
    // the production `HasSymbolicCode` impl.
    assert_ne!(
        code,
        SymbolicCode::INTERNAL_INVARIANT,
        "YamlError variant {}: symbolic_code must not be INTERNAL_INVARIANT",
        variant
    );

    // The code name must be non-empty.
    assert!(
        !code.as_str().is_empty(),
        "YamlError variant {}: symbolic_code must not be empty",
        variant
    );
}
