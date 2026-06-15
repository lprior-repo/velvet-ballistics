#![forbid(unsafe_code)]
//! PO-KANI-002: Kani harness verifying that every `YamlError` variant,
//! when passed through the production `HasSymbolicCode::symbolic_code()`
//! implementation, returns a `SymbolicCode` that is registered in
//! `vb_core::CODE_REGISTRY` (i.e., `SymbolicCode::from_static` returns
//! `Some`).
//!
//! Proves: No `YamlError` variant falls through to the `INTERNAL_INVARIANT`
//! fallback. All 21 variants (as of vb-jpq7.34) produce registered codes.
//!
//! GOD RULE 1 compliance: Uses `kani::any()` for variant selection and
//! field generation — no hardcoded dummy data.
//!
//! Bound: 21 YamlError variants.

use crate::YamlError;
use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};

/// Total number of YamlError variants as of vb-jpq7.34.
const YAML_ERROR_VARIANT_COUNT: u8 = 21;

/// Number of symbolic bytes used to materialise each string-typed field.
/// kani 0.67.0 does not implement `Arbitrary` for `String`, so we
/// generate a fixed-size byte array and convert via `String::from_utf8_lossy`.
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

/// Generate a `&'static str` of length `N` from a symbolic ASCII byte array
/// by leaking the converted `String`. The leak is bounded by N.
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

/// Generate an arbitrary YamlError variant with arbitrary field values
/// using Kani's symbolic execution to cover all branches of
/// `symbolic_code()`.
///
/// NOTE (R-003-revised): kani 0.67.0 does not implement `Arbitrary` for
/// `String`, so we materialise string fields from a symbolic
/// `[u8; STRING_FIELD_BYTES]` via `String::from_utf8_lossy`. The leak
/// for `&'static str` fields is bounded by the same N.
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
        // Compile-time exhaustiveness: if variant >= YAML_ERROR_VARIANT_COUNT
        // the precondition is violated (kani::assume restricts to [0, 20]).
        _ => YamlError::EmptySource,
    }
}

/// PO-KANI-002: Every YamlError variant's symbolic_code() is registered.
///
/// Constructs every variant with arbitrary field values and asserts that
/// `SymbolicCode::from_static(code.as_str())` returns `Some`, proving
/// no variant ever hits the `INTERNAL_INVARIANT` fallback in the
/// production `HasSymbolicCode` impl.
#[kani::proof]
// CODE_REGISTRY has 236 entries; symbolic_to_numeric iterates the
// registry, so this bound must exceed 236 to cover the worst case.
#[kani::unwind(256)]
fn verify_all_variants_registered() {
    let variant: u8 = kani::any();
    kani::assume(variant < YAML_ERROR_VARIANT_COUNT);

    let error = arbitrary_yaml_error(variant);
    let code = error.symbolic_code();

    // The returned code must be a registered SymbolicCode.
    // from_static returns Some only for registered names.
    let registered = SymbolicCode::from_static(code.as_str());
    kani::assert(registered.is_some(),
        "YamlError variant {}: symbolic_code '{}' is not registered in CODE_REGISTRY",
        variant, code.as_str())

    // The code must not be the INTERNAL_INVARIANT sentinel.
    kani::assert_ne!(code,
        SymbolicCode::INTERNAL_INVARIANT,
        "YamlError variant {}: symbolic_code must not be INTERNAL_INVARIANT",
        variant)

    // The code name must be non-empty.
    kani::assert(!code.as_str().is_empty(),
        "YamlError variant {}: symbolic_code must not be empty", variant)
}
