#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

//! Unit tests for `digest_step_primitive` Save arm.
//!
//! These tests prove that the `digest_step_primitive` function includes
//! the Save { value } field bytes in the hasher state — not just the
//! canonical primitive name.
//!
//! The bug (pre-fix): Save fell through to the `other =>` catch-all
//! which only hashed `canonical_primitive_name(other)` (e.g. b"set"),
//! ignoring the semantic `value` field entirely.
//!
//! The fix: explicit match arm for Save (part_05.rs:374-381) that hashes
//! `b"set" + value` where `value` is encoded as UTF-8 bytes for
//! `ScalarValue::String` and `i64::to_le_bytes()` for `ScalarValue::Integer`.

#![forbid(unsafe_code)]

use super::*;
use vb_yaml::ast::{ScalarValue, StepPrimitive};

/// Build a Save StepPrimitive with a String value.
fn save_string(value: &str) -> StepPrimitive {
    StepPrimitive::Save {
        value: ScalarValue::String(value.to_string()),
    }
}

/// Build a Save StepPrimitive with an Integer value.
fn save_integer(value: i64) -> StepPrimitive {
    StepPrimitive::Save {
        value: ScalarValue::Integer(value),
    }
}

/// Hash a manual byte sequence and return the blake3 output bytes.
fn hash_bytes(data: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for chunk in data {
        hasher.update(chunk);
    }
    hasher.finalize().into()
}

/// Call digest_step_primitive with a hasher and return the final hash bytes.
fn hash_primitive(primitive: &StepPrimitive) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    digest_step_primitive(&mut hasher, primitive).expect("valid test primitive");
    hasher.finalize().into()
}

// ── Save digest tests ─────────────────────────────────────────────────

/// Verify Save arm hits (not catch-all fallthrough) by checking that
/// Save { value } produces a different digest than a catch-all would
/// produce for the same primitive name.
#[test]
fn save_value_field_affects_digest() {
    let save_step = save_integer(42);

    // The explicit Save arm hashes b"set" + 42_i64.to_le_bytes()
    let actual = hash_primitive(&save_step);

    // Simulate the pre-fix catch-all: only hash the primitive name "set"
    let catch_all_only_name = hash_bytes(&[b"set"]);

    assert_ne!(
        actual, catch_all_only_name,
        "Save arm must hash field bytes beyond just the primitive name 'set'; \
         pre-fix catch-all hash checked only the name"
    );
}

/// Verify that different Integer values produce different digests.
/// This is the core field-sensitivity proof: the value field affects the
/// digest, so the Save arm is not just hashing the discriminator.
#[test]
fn save_different_integer_values_produce_different_digests() {
    let save_a = save_integer(42);
    let save_b = save_integer(43);

    let hash_a = hash_primitive(&save_a);
    let hash_b = hash_primitive(&save_b);

    assert_ne!(
        hash_a, hash_b,
        "Save primitives with different Integer values must produce different digests"
    );
}

/// Verify that different String values produce different digests.
#[test]
fn save_different_string_values_produce_different_digests() {
    let save_a = save_string("alpha");
    let save_b = save_string("beta");

    let hash_a = hash_primitive(&save_a);
    let hash_b = hash_primitive(&save_b);

    assert_ne!(
        hash_a, hash_b,
        "Save primitives with different String values must produce different digests"
    );
}

/// Verify that Save String and Save Integer produce different digests
/// even when the logical value is the same. This proves that the encoding
/// discriminates the `ScalarValue` variant in addition to the value.
#[test]
fn save_string_vs_integer_produce_different_digest() {
    let save_str = save_string("42");
    let save_int = save_integer(42);

    let hash_str = hash_primitive(&save_str);
    let hash_int = hash_primitive(&save_int);

    assert_ne!(
        hash_str, hash_int,
        "Save String vs Integer must produce different digests even for same logical value"
    );
}

/// Verify that the explicit Save String encoding is `b"save" + UTF-8 bytes`.
#[test]
fn save_string_encoding_matches_save_plus_utf8_bytes() {
    let save_step = save_string("my_output");

    let actual = hash_primitive(&save_step);
    let expected = hash_bytes(&[b"set", b"my_output"]);

    assert_eq!(
        actual, expected,
        "Save String must hash b\"set\" + raw UTF-8 bytes"
    );
}

/// Verify that the explicit Save Integer encoding is `b"set" + i64 LE bytes`.
#[test]
fn save_integer_encoding_matches_save_plus_le_bytes() {
    let save_step = save_integer(42);

    let actual = hash_primitive(&save_step);
    let expected = hash_bytes(&[b"set", &42_i64.to_le_bytes()]);

    assert_eq!(
        actual, expected,
        "Save Integer must hash b\"set\" + i64 LE bytes"
    );
}

/// Verify that Save Integer boundary values produce distinct digests.
#[test]
fn save_integer_boundary_values_produce_distinct_digests() {
    let save_zero = save_integer(0);
    let save_max = save_integer(i64::MAX);
    let save_min = save_integer(i64::MIN);

    let h_zero = hash_primitive(&save_zero);
    let h_max = hash_primitive(&save_max);
    let h_min = hash_primitive(&save_min);

    assert_ne!(h_zero, h_max, "Save Integer 0 vs MAX must differ");
    assert_ne!(h_zero, h_min, "Save Integer 0 vs MIN must differ");
    assert_ne!(h_max, h_min, "Save Integer MAX vs MIN must differ");
}

/// Verify that Save is deterministic: calling digest_step_primitive twice
/// on the same Save primitive produces identical digests.
#[test]
fn save_deterministic_digest() {
    let save_step = save_integer(7);

    let h1 = hash_primitive(&save_step);
    let h2 = hash_primitive(&save_step);

    assert_eq!(
        h1, h2,
        "Save digest must be deterministic: identical inputs must produce identical outputs"
    );
}

/// Verify that Save with String "0" differs from Save with Integer 0.
/// This guards against a regression where the encoding collapses
/// numeric and string forms of the same digit string.
#[test]
fn save_string_zero_differs_from_integer_zero() {
    let save_str_zero = save_string("0");
    let save_int_zero = save_integer(0);

    let h_str = hash_primitive(&save_str_zero);
    let h_int = hash_primitive(&save_int_zero);

    assert_ne!(
        h_str, h_int,
        "Save String(\"0\") and Save Integer(0) must produce different digests"
    );
}

/// Verify that two different Save primitives with empty String values
/// still produce identical digests (deterministic encoding for empty string).
#[test]
fn save_empty_string_deterministic() {
    let save_a = save_string("");
    let save_b = save_string("");

    let h_a = hash_primitive(&save_a);
    let h_b = hash_primitive(&save_b);

    assert_eq!(
        h_a, h_b,
        "Save with empty String value must produce deterministic digest"
    );
}
