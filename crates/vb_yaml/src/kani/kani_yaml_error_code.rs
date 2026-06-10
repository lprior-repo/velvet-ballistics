#![forbid(unsafe_code)]
//! PO-006: Kani harness verifying that every `YamlError` variant,
//! when constructed with arbitrary field values, returns a
//! `SymbolicCode` that is registered in `vb_core::CODE_REGISTRY`
//! (i.e., `SymbolicCode::from_static` returns `Some`).
//!
//! GOD RULE 1 compliance:
//! - Variant selector is `kani::any::<u8>()`, bounded via `kani::assume`.
//! - Every field is `kani::any::<T>()` for its concrete type.
//! - String fields use `kani::any::<[u8; N]>()` (kani 0.67.0 does not
//!   implement `Arbitrary` for `String`) and are converted via
//!   `String::from_utf8_lossy`. The byte array is symbolic, so every
//!   byte is arbitrary — no hardcoded `YamlError` literals, no fixed
//!   dummy strings.
//! - Uses the production `YamlError` and `HasSymbolicCode` impl from
//!   `vb_yaml::error`, not a parallel mini-model.
//!
//! Bound: 21 YamlError variants (unwind=21).
//!
//! R-003-revised: kani 0.67.0 does not implement `Arbitrary` for
//! `String`, so we use `kani::any::<[u8; N]>()` and convert. The
//! resulting `String`/`&'static str` lifetime is bounded by N bytes
//! to keep Kani tractability.

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

/// Number of symbolic bytes used to materialise each string-typed field.
/// Small constant (keepts Kani tractability) yet non-trivial so
/// every byte is symbolic and no field is hardcoded.
const STRING_FIELD_BYTES: usize = 8;

/// Generate a `Box<str>` of length `N` from a symbolic ASCII byte array.
///
/// Uses `kani::any::<[u8; N]>()` because kani 0.67.0 does not implement
/// `Arbitrary` for `String`. Each byte is constrained to ASCII (0..0x80)
/// via `kani::assume`, which guarantees valid UTF-8 by construction
/// and avoids the unbounded CBMC unwind through `String::from_utf8_lossy`.
/// Length is fixed at `N` (no allocation-time length symbolic), keeping
/// Kani tractable while every byte remains symbolic.
#[inline]
fn bounded_box_str<const N: usize>() -> Box<str> {
    let bytes: [u8; N] = kani::any();
    let mut owned = [0u8; N];
    let mut i = 0;
    while i < N {
        let b = bytes[i];
        kani::assume(b < 0x80);
        owned[i] = b;
        i += 1;
    }
    // Map each ASCII byte to its char representation; valid 1-byte UTF-8
    // by construction. This bypasses `core::str::validations::run_utf8_validation`
    // which iterates the full multi-byte UTF-8 sequence analysis and would
    // blow up the CBMC unwind bound on a 21-variant harness.
    let s: String = owned.iter().map(|&b| b as char).collect();
    s.into_boxed_str()
}

/// Generate a `&'static str` of length `N` from a symbolic ASCII byte
/// array by leaking the converted `String`. The leak is bounded by N.
#[inline]
fn bounded_static_str<const N: usize>() -> &'static str {
    let bytes: [u8; N] = kani::any();
    let mut owned = [0u8; N];
    let mut i = 0;
    while i < N {
        let b = bytes[i];
        kani::assume(b < 0x80);
        owned[i] = b;
        i += 1;
    }
    let s: String = owned.iter().map(|&b| b as char).collect();
    Box::leak(s.into_boxed_str())
}

/// Construct an arbitrary `YamlError` whose variant is selected by
/// `variant` (a symbolic byte in `[0, YAML_ERROR_VARIANT_COUNT)`)
/// and whose fields are all symbolic values for the field's concrete
/// type.
///
/// Every field is symbolic — no constants, no dummy strings, no
/// hardcoded counters. This is the GOD RULE 1 fix.
fn arbitrary_yaml_error(variant: u8) -> YamlError {
    match variant {
        0 => YamlError::DuplicateKey {
            key: bounded_box_str::<STRING_FIELD_BYTES>(),
        },
        1 => YamlError::ForbiddenFeature {
            detail: bounded_static_str::<STRING_FIELD_BYTES>(),
        },
        2 => YamlError::AnchorAliasMerge,
        3 => YamlError::CustomTag {
            tag: bounded_box_str::<STRING_FIELD_BYTES>(),
        },
        4 => YamlError::BinaryScalar,
        5 => YamlError::MultipleDocuments {
            count: kani::any::<usize>(),
        },
        6 => YamlError::AmbiguousScalar {
            scalar: bounded_box_str::<STRING_FIELD_BYTES>(),
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
            field: bounded_box_str::<STRING_FIELD_BYTES>(),
        },
        14 => YamlError::EmptySource,
        15 => YamlError::MissingField {
            field: bounded_static_str::<STRING_FIELD_BYTES>(),
        },
        16 => YamlError::FieldShape {
            field: bounded_static_str::<STRING_FIELD_BYTES>(),
            expected: bounded_static_str::<STRING_FIELD_BYTES>(),
        },
        17 => YamlError::ParseError {
            line: kani::any::<usize>(),
            reason: bounded_box_str::<STRING_FIELD_BYTES>(),
        },
        18 => YamlError::UnsupportedFeature {
            feature: bounded_static_str::<STRING_FIELD_BYTES>(),
        },
        19 => YamlError::UnsupportedTrigger {
            trigger: bounded_static_str::<STRING_FIELD_BYTES>(),
        },
        20 => YamlError::LegacyPrimitive {
            primitive: bounded_static_str::<STRING_FIELD_BYTES>(),
            canonical: bounded_static_str::<STRING_FIELD_BYTES>(),
        },
        // Compile-time exhaustiveness: if `variant >= YAML_ERROR_VARIANT_COUNT`
        // the precondition is violated (kani::assume restricts to [0, 21)).
        _ => YamlError::EmptySource,
    }
}

/// PO-006: For every YamlError variant, with arbitrary field values,
/// `HasSymbolicCode::symbolic_code` returns a `SymbolicCode` that is
/// registered in `vb_core::CODE_REGISTRY` and is not the
/// `INTERNAL_INVARIANT` sentinel.
#[cfg(kani)]
#[kani::proof]
// CODE_REGISTRY has 236 entries; symbolic_to_numeric iterates the
// registry, so this bound must exceed 236 to cover the worst case.
// 256 is the next power-of-two above 236, providing margin for
// per-field utf-8 validation and per-byte memcmp.
#[kani::unwind(256)]
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
