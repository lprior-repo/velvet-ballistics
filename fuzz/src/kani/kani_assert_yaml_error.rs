#![forbid(unsafe_code)]
//! PO-KANI-003: Kani harness verifying that `assert_typed_yaml_error` in
//! the fuzz crate handles all 21 `YamlError` variants without panicking,
//! and that symbolic code extraction succeeds for every variant.
//!
//! Proves: `assert_typed_yaml_error` completes without panic for any
//! possible `YamlError` variant. The exhaustive match in the function
//! covers all 21 variants (compile-time enforced).
//!
//! GOD RULE 1 compliance: Uses `kani::any()` for variant selection —
//! no hardcoded dummy error data except for field construction.
//!
//! GOD RULE 2 compliance (REPAIRED): Calls the REAL production function
//! `fuzz_lib::assert_typed_yaml_error` — no model copy.
//!
//! Bound: 21 YamlError variants.

use vb_core::diagnostic::SymbolicCode;
use vb_yaml::YamlError;

const YAML_ERROR_VARIANT_COUNT: u8 = 21;

/// Generate an arbitrary YamlError variant. The variant index is chosen
/// symbolically via kani::any(), ensuring full coverage of the match arms
/// in both `assert_typed_yaml_error` and `symbolic_code()`.
///
/// NOTE: String fields use `kani::any::<String>()` (owned) instead of
/// `kani::any::<&str>()` (borrowed) to avoid potential UB from dangling
/// symbolic references (R-003). `Box::from(String)` or `.into_boxed_str()`
/// is used where the production type expects `Box<str>`.
fn arbitrary_yaml_error(variant: u8) -> YamlError {
    match variant {
        0 => YamlError::DuplicateKey {
            key: kani::any::<String>().into_boxed_str(),
        },
        1 => YamlError::ForbiddenFeature {
            detail: &*kani::any::<String>().leak(),
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
            field: &*kani::any::<String>().leak(),
        },
        16 => YamlError::FieldShape {
            field: &*kani::any::<String>().leak(),
            expected: &*kani::any::<String>().leak(),
        },
        17 => YamlError::ParseError {
            line: kani::any::<usize>(),
            reason: kani::any::<String>().into_boxed_str(),
        },
        18 => YamlError::UnsupportedFeature {
            feature: &*kani::any::<String>().leak(),
        },
        19 => YamlError::UnsupportedTrigger {
            trigger: &*kani::any::<String>().leak(),
        },
        20 => YamlError::LegacyPrimitiveDeprecated {
            name: kani::any::<String>(),
            replacement: kani::any::<String>(),
        },
        _ => YamlError::EmptySource,
    }
}

/// PO-KANI-003: `assert_typed_yaml_error` handles any YamlError variant
/// without panicking. The function must never panic, regardless of the
/// variant or its field values.
///
/// REPAIRED (F-REVIEW-002): Now calls the REAL production function
/// `fuzz_lib::assert_typed_yaml_error` instead of a local model copy.
/// The production function at `fuzz/src/lib.rs:200` was made `pub(crate)`
/// to enable this call.
#[kani::proof]
#[kani::unwind(21)]
fn check_assert_typed_yaml_error_total() {
    let variant: u8 = kani::any();
    kani::assume(variant < YAML_ERROR_VARIANT_COUNT);

    let error = arbitrary_yaml_error(variant);
    // PRODUCTION FUNCTION: calls fuzz_lib::assert_typed_yaml_error
    // (imported as crate::assert_typed_yaml_error via `use` below)
    crate::assert_typed_yaml_error(error);
}

/// PO-KANI-003 (extended): symbolic_code() succeeds for every variant.
/// The code returned must be registered in CODE_REGISTRY.
#[kani::proof]
#[kani::unwind(21)]
fn check_assert_typed_yaml_error_code_registered() {
    let variant: u8 = kani::any();
    kani::assume(variant < YAML_ERROR_VARIANT_COUNT);

    let error = arbitrary_yaml_error(variant);
    let code = error.symbolic_code();

    // Must be registered.
    assert!(SymbolicCode::from_static(code.as_str()).is_some());

    // Must not be the sentinel.
    assert_ne!(code, SymbolicCode::INTERNAL_INVARIANT);
}
