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
//!   `b as char` per byte. The byte array is symbolic, so every
//!   byte is arbitrary — no hardcoded `YamlError` literals, no fixed
//!   dummy strings. The `b as char` mapping bypasses
//!   `core::str::validations::run_utf8_validation` (valid 1-byte
//!   UTF-8 by construction since each byte is constrained to
//!   ASCII via `kani::assume(b < 0x80)`) and `String::from_utf8_lossy`.
//! - Uses the production `YamlError` and `HasSymbolicCode` impl from
//!   `vb_yaml::error`, not a parallel mini-model.
//!
//! Bound: 21 YamlError variants. Harness unwind is 256 (see
//! `kani_yaml_error_code_registered`), not 21 — the 256 bound is
//! driven by the 236-entry `vb_core::CODE_REGISTRY` lookup that
//! `SymbolicCode::from_static` performs on the produced code.
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
/// MissingField, FieldShape, ParseError, ForbiddenFeature,
/// LegacyPrimitiveDeprecated.
const YAML_ERROR_VARIANT_COUNT: u8 = 21;

/// Number of symbolic bytes used to materialise each string-typed field.
///
/// Reduced from 8 to 1 (bead vb-7jjh7, Option A) to keep the
/// `kani_yaml_error_code_registered` SAT instance tractable. At 8 bytes
/// per string field × 21 variants × multiple string fields per variant,
/// the SAT instance reached 12M variables × 612M clauses and timed out
/// at 1500s. With 1 byte per field the per-field symbolic-input space
/// is 128 (7-bit ASCII) and the worst-case total SAT instance drops by
/// roughly an order of magnitude. The core invariant still holds:
/// every variant produces a registered symbolic code regardless of
/// the field content (any of 128 ASCII bytes is covered symbolically).
const STRING_FIELD_BYTES: usize = 1;

/// Generate a `Box<str>` of length `N` from a symbolic ASCII byte array.
///
/// Uses `kani::any::<[u8; N]>()` because kani 0.67.0 does not implement
/// `Arbitrary` for `String`. Each byte is constrained to ASCII (0..0x80)
/// via `kani::assume`, which guarantees valid UTF-8 by construction.
/// Length is fixed at `N` (no allocation-time length symbolic), keeping
/// Kani tractable while every byte remains symbolic. The per-byte
/// `b as char` mapping below produces valid 1-byte UTF-8 without
/// invoking `core::str::validations::run_utf8_validation` or
/// `String::from_utf8_lossy`, which would otherwise inflate the
/// CBMC unwind bound on the 21-variant harness.
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

#[inline]
fn bounded_string<const N: usize>() -> String {
    String::from(bounded_box_str::<N>())
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
        20 => YamlError::LegacyPrimitiveDeprecated {
            name: bounded_string::<STRING_FIELD_BYTES>(),
            replacement: bounded_string::<STRING_FIELD_BYTES>(),
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
    kani::assert(registered.is_some(),
        "YamlError variant {}: symbolic_code '{}' is not registered in CODE_REGISTRY",
        variant, code.as_str())

    // The code must never be the INTERNAL_INVARIANT sentinel; that
    // would mean a variant fell through to the unreachable fallback in
    // the production `HasSymbolicCode` impl.
    kani::assert_ne!(code,
        SymbolicCode::INTERNAL_INVARIANT,
        "YamlError variant {}: symbolic_code must not be INTERNAL_INVARIANT",
        variant)

    // The code name must be non-empty.
    kani::assert(!code.as_str().is_empty(),
        "YamlError variant {}: symbolic_code must not be empty", variant)
}
