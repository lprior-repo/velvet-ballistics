#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]

use crate::errors::CoreError;
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint, join_taint};
use crate::value_store::ValueStore;

// -- FiniteF64 rejection tests --

#[test]
fn finite_f64_rejects_nan_returns_non_finite_number() {
    let result = FiniteF64::new(f64::NAN);

    assert_eq!(result, Err(CoreError::NonFiniteNumber));
}

#[test]
fn finite_f64_rejects_positive_infinity_returns_non_finite_number() {
    let result = FiniteF64::new(f64::INFINITY);

    assert_eq!(result, Err(CoreError::NonFiniteNumber));
}

#[test]
fn finite_f64_rejects_negative_infinity_returns_non_finite_number() {
    let result = FiniteF64::new(f64::NEG_INFINITY);

    assert_eq!(result, Err(CoreError::NonFiniteNumber));
}

#[test]
fn finite_f64_accepts_zero() {
    let result = FiniteF64::new(0.0);

    assert_eq!(result.map(|f| f.get()), Ok(0.0));
}

#[test]
fn finite_f64_accepts_negative_one() {
    let result = FiniteF64::new(-1.0);

    assert_eq!(result.map(|f| f.get()), Ok(-1.0));
}

#[test]
fn finite_f64_accepts_max_finite() {
    let result = FiniteF64::new(f64::MAX);

    assert_eq!(result.map(|f| f.get()), Ok(f64::MAX));
}

#[test]
fn finite_f64_get_returns_inner_value() -> Result<(), String> {
    let value = FiniteF64::new(3.25).map_err(|error| error.to_string())?;

    if value.get() != 3.25 {
        return Err(String::from("unexpected finite f64 inner value"));
    }
    Ok(())
}

// -- SlotValue type_name tests --

#[test]
fn slot_value_type_name_returns_correct_names() -> Result<(), String> {
    if SlotValue::Null.type_name() != "null" {
        return Err(String::from("unexpected null type name"));
    }
    if SlotValue::Bool(true).type_name() != "boolean" {
        return Err(String::from("unexpected true type name"));
    }
    if SlotValue::Bool(false).type_name() != "boolean" {
        return Err(String::from("unexpected false type name"));
    }
    if SlotValue::I64(0).type_name() != "number" {
        return Err(String::from("unexpected i64 type name"));
    }
    let f64_val = FiniteF64::new(1.0).map_err(|error| error.to_string())?;
    if SlotValue::F64(f64_val).type_name() != "number" {
        return Err(String::from("unexpected f64 type name"));
    }
    if SlotValue::Symbol(SymbolId::new(1)).type_name() != "symbol" {
        return Err(String::from("unexpected symbol type name"));
    }
    if SlotValue::List(ListId::new(1)).type_name() != "list" {
        return Err(String::from("unexpected list type name"));
    }
    if SlotValue::Object(ObjectId::new(1)).type_name() != "object" {
        return Err(String::from("unexpected object type name"));
    }
    if SlotValue::Blob(BlobId::new(1)).type_name() != "blob" {
        return Err(String::from("unexpected blob type name"));
    }
    Ok(())
}

// -- SlotValue is_true tests --

#[test]
fn slot_value_is_true_only_for_bool_true() {
    assert!(SlotValue::Bool(true).is_true());
}

#[test]
fn slot_value_is_true_returns_false_for_bool_false() {
    assert!(!SlotValue::Bool(false).is_true());
}

#[test]
fn slot_value_is_true_returns_false_for_null() {
    assert!(!SlotValue::Null.is_true());
}

// -- ConstValue to_slot_value mapping tests --

#[test]
fn const_value_to_slot_value_maps_null_correctly() {
    let result = ConstValue::Null.to_slot_value();

    assert_eq!(result, Ok(SlotValue::Null));
}

#[test]
fn const_value_to_slot_value_maps_bool_true_correctly() {
    let result = ConstValue::Bool(true).to_slot_value();

    assert_eq!(result, Ok(SlotValue::Bool(true)));
}

#[test]
fn const_value_to_slot_value_maps_i64_correctly() {
    let result = ConstValue::I64(42).to_slot_value();

    assert_eq!(result, Ok(SlotValue::I64(42)));
}

#[test]
fn const_value_to_slot_value_maps_f64_correctly() -> Result<(), String> {
    let finite = FiniteF64::new(2.5).map_err(|error| error.to_string())?;
    let result = ConstValue::F64(finite).to_slot_value();

    if result != Ok(SlotValue::F64(finite)) {
        return Err(String::from("unexpected f64 slot value"));
    }
    Ok(())
}

#[test]
fn const_value_to_slot_value_maps_symbol_correctly() {
    let result = ConstValue::Symbol(SymbolId::new(7)).to_slot_value();

    assert_eq!(result, Ok(SlotValue::Symbol(SymbolId::new(7))));
}

// =========================================================================
// Adversarial BDD tests — FiniteF64 edge cases
// =========================================================================

#[test]
fn finite_f64_negative_zero_is_accepted_and_preserves_sign_bit() {
    // -0.0 is finite and must be accepted; the sign bit must survive.
    let result = FiniteF64::new(-0.0_f64);
    assert_eq!(result.as_ref().map(|f| f.get()), Ok(-0.0_f64));
    // Confirm it is distinct from +0.0 at the bit-pattern level.
    assert_eq!(result.map(|f| f.get().to_bits()), Ok((-0.0_f64).to_bits()));
}

#[test]
fn finite_f64_positive_zero_is_accepted() {
    let result = FiniteF64::new(0.0_f64);
    assert_eq!(result.map(|f| f.get()), Ok(0.0_f64));
}

#[test]
fn finite_f64_rejects_canonical_nan_quiet() {
    let result = FiniteF64::new(f64::NAN);
    assert_eq!(result, Err(CoreError::NonFiniteNumber));
}

#[test]
fn finite_f64_rejects_signaling_nan() {
    // Signaling NaN: exponent all-ones, MSB of mantissa clear, non-zero mantissa.
    let signaling_nan = f64::from_bits(0x7FF0_0000_0000_0001_u64);
    assert!(signaling_nan.is_nan(), "must be NaN");
    assert_eq!(
        FiniteF64::new(signaling_nan),
        Err(CoreError::NonFiniteNumber)
    );
}

#[test]
fn finite_f64_rejects_negative_signaling_nan() {
    let neg_signaling_nan = f64::from_bits(0xFFF0_0000_0000_0001_u64);
    assert!(neg_signaling_nan.is_nan(), "must be NaN");
    assert_eq!(
        FiniteF64::new(neg_signaling_nan),
        Err(CoreError::NonFiniteNumber)
    );
}

#[test]
fn finite_f64_rejects_nan_payload_variants() {
    // Try several NaN bit patterns to ensure no bypass.
    let payloads: [u64; 4] = [
        0x7FF8_0000_0000_0000,
        0x7FFC_0000_0000_0000,
        0xFFF8_0000_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
    ];
    for payload in payloads {
        let nan_val = f64::from_bits(payload);
        assert!(nan_val.is_nan(), "payload {payload:#018X} must be NaN");
        assert_eq!(
            FiniteF64::new(nan_val),
            Err(CoreError::NonFiniteNumber),
            "NaN payload {payload:#018X} must be rejected"
        );
    }
}

#[test]
fn finite_f64_accepts_smallest_positive_subnormal() {
    let subnormal = f64::from_bits(1_u64); // smallest positive subnormal
    assert!(subnormal.is_subnormal(), "must be subnormal");
    let result = FiniteF64::new(subnormal);
    assert_eq!(result.map(|f| f.get()), Ok(subnormal));
}

#[test]
fn finite_f64_accepts_largest_subnormal() {
    let largest_subnormal = f64::from_bits(0x000F_FFFF_FFFF_FFFF_u64);
    assert!(largest_subnormal.is_subnormal(), "must be subnormal");
    assert!(largest_subnormal.is_finite(), "subnormals are finite");
    let result = FiniteF64::new(largest_subnormal);
    assert_eq!(result.map(|f| f.get()), Ok(largest_subnormal));
}

#[test]
fn finite_f64_accepts_smallest_negative_subnormal() {
    let neg_subnormal = f64::from_bits(0x8000_0000_0000_0001_u64);
    assert!(neg_subnormal.is_subnormal(), "must be negative subnormal");
    assert!(neg_subnormal.is_finite());
    let result = FiniteF64::new(neg_subnormal);
    assert_eq!(result.map(|f| f.get()), Ok(neg_subnormal));
}

#[test]
fn finite_f64_accepts_min_positive_normal() {
    let min_normal = f64::MIN_POSITIVE; // 2.2250738585072014e-308
    assert!(!min_normal.is_subnormal());
    assert!(min_normal.is_finite());
    let result = FiniteF64::new(min_normal);
    assert_eq!(result.map(|f| f.get()), Ok(min_normal));
}

#[test]
fn finite_f64_accepts_f64_min() {
    // f64::MIN is the most negative finite value
    let result = FiniteF64::new(f64::MIN);
    assert_eq!(result.map(|f| f.get()), Ok(f64::MIN));
}

#[test]
fn finite_f64_accepts_f64_max() {
    let result = FiniteF64::new(f64::MAX);
    assert_eq!(result.map(|f| f.get()), Ok(f64::MAX));
}

#[test]
fn finite_f64_rejects_positive_infinity() {
    assert_eq!(
        FiniteF64::new(f64::INFINITY),
        Err(CoreError::NonFiniteNumber)
    );
}

#[test]
fn finite_f64_rejects_negative_infinity() {
    assert_eq!(
        FiniteF64::new(f64::NEG_INFINITY),
        Err(CoreError::NonFiniteNumber)
    );
}

// =========================================================================
// Adversarial BDD tests — SlotValue type confusion and edge cases
// =========================================================================

#[test]
fn slot_value_i64_max_roundtrips() {
    let val = SlotValue::I64(i64::MAX);
    assert_eq!(val.type_name(), "number");
    assert!(!val.is_true());
}

#[test]
fn slot_value_i64_min_roundtrips() {
    let val = SlotValue::I64(i64::MIN);
    assert_eq!(val.type_name(), "number");
    assert!(!val.is_true());
}

#[test]
fn slot_value_i64_zero_roundtrips() {
    let val = SlotValue::I64(0);
    assert_eq!(val.type_name(), "number");
    assert!(!val.is_true());
}

#[test]
fn slot_value_i64_negative_one_roundtrips() {
    let val = SlotValue::I64(-1);
    assert_eq!(val.type_name(), "number");
}

#[test]
fn slot_value_null_is_not_true() {
    assert!(!SlotValue::Null.is_true());
    assert_eq!(SlotValue::Null.type_name(), "null");
}

#[test]
fn slot_value_bool_false_is_not_true() {
    assert!(!SlotValue::Bool(false).is_true());
}

#[test]
fn slot_value_symbol_zero_is_valid() {
    let val = SlotValue::Symbol(SymbolId::new(0));
    assert_eq!(val.type_name(), "symbol");
    assert!(!val.is_true());
}

#[test]
fn slot_value_symbol_max_u32_is_valid() {
    let val = SlotValue::Symbol(SymbolId::new(u32::MAX));
    assert_eq!(val.type_name(), "symbol");
}

#[test]
fn slot_value_list_max_u32_is_valid() {
    let val = SlotValue::List(ListId::new(u32::MAX));
    assert_eq!(val.type_name(), "list");
}

#[test]
fn slot_value_object_max_u32_is_valid() {
    let val = SlotValue::Object(ObjectId::new(u32::MAX));
    assert_eq!(val.type_name(), "object");
}

#[test]
fn slot_value_blob_max_u64_is_valid() {
    let val = SlotValue::Blob(BlobId::new(u64::MAX));
    assert_eq!(val.type_name(), "blob");
}

#[test]
fn slot_value_f64_with_negative_zero_is_valid() {
    let result = FiniteF64::new(-0.0_f64);
    assert_eq!(result.as_ref().map(|f| f.get()), Ok(-0.0_f64));
    let finite = result.expect("setup: negative zero must be finite");
    let val = SlotValue::F64(finite);
    assert_eq!(val.type_name(), "number");
    assert!(!val.is_true());
}

// =========================================================================
// Adversarial BDD tests — Taint propagation and ordering
// =========================================================================

#[test]
fn taint_clean_is_zero_discriminant() {
    assert_eq!(taint_discriminant(Taint::Clean), 0);
}

#[test]
fn taint_secret_is_two_discriminant() {
    assert_eq!(taint_discriminant(Taint::Secret), 2);
}

#[test]
fn taint_derived_from_secret_is_one_discriminant() {
    assert_eq!(taint_discriminant(Taint::DerivedFromSecret), 1);
}

fn taint_discriminant(taint: Taint) -> u8 {
    match taint {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    }
}

#[test]
fn taint_variants_are_distinct() {
    let variants = [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b, "same index must be equal");
            } else {
                assert_ne!(a, b, "different indices must be distinct");
            }
        }
    }
}

#[test]
fn taint_copy_semantics_preserve_equality() {
    let a = Taint::Secret;
    let b = a; // copy
    assert_eq!(a, b, "copy must preserve equality");
}

#[test]
fn taint_postcard_roundtrips_all_variants() {
    let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
    for variant in variants {
        let bytes = postcard::to_allocvec(&variant);
        assert!(
            bytes.is_ok(),
            "postcard serialization should succeed for {variant:?}"
        );
        let Ok(bytes) = bytes else { return };
        let recovered: Result<Taint, _> = postcard::from_bytes(&bytes);
        assert!(
            recovered.is_ok(),
            "postcard deserialization should succeed for {variant:?}"
        );
        let Ok(recovered) = recovered else { return };
        assert_eq!(variant, recovered, "roundtrip must preserve {variant:?}");
    }
}

#[test]
fn taint_lattice_join_is_commutative() {
    let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
    for a in variants {
        for b in variants {
            assert_eq!(
                join_taint(a, b),
                join_taint(b, a),
                "join_taint must be commutative: join({a:?}, {b:?})"
            );
        }
    }
}

#[test]
fn taint_lattice_join_is_associative() {
    let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
    for a in variants {
        for b in variants {
            for c in variants {
                assert_eq!(
                    join_taint(join_taint(a, b), c),
                    join_taint(a, join_taint(b, c)),
                    "join_taint must be associative: ({a:?}, {b:?}, {c:?})"
                );
            }
        }
    }
}

#[test]
fn taint_lattice_secret_is_top_element() {
    let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
    for v in variants {
        assert_eq!(
            join_taint(v, Taint::Secret),
            Taint::Secret,
            "Secret must absorb all: join({v:?}, Secret)"
        );
    }
}

#[test]
fn taint_lattice_clean_is_bottom_element() {
    let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
    for v in variants {
        assert_eq!(
            join_taint(v, Taint::Clean),
            v,
            "Clean must be identity: join({v:?}, Clean)"
        );
    }
}

// =========================================================================
// Adversarial BDD tests — ConstValue edge cases
// =========================================================================

#[test]
fn const_value_to_slot_value_i64_max() {
    let result = ConstValue::I64(i64::MAX).to_slot_value();
    assert_eq!(result, Ok(SlotValue::I64(i64::MAX)));
}

#[test]
fn const_value_to_slot_value_i64_min() {
    let result = ConstValue::I64(i64::MIN).to_slot_value();
    assert_eq!(result, Ok(SlotValue::I64(i64::MIN)));
}

#[test]
fn const_value_to_slot_value_bool_false() {
    let result = ConstValue::Bool(false).to_slot_value();
    assert_eq!(result, Ok(SlotValue::Bool(false)));
}

#[test]
fn const_value_to_slot_value_symbol_zero() {
    let result = ConstValue::Symbol(SymbolId::new(0)).to_slot_value();
    assert_eq!(result, Ok(SlotValue::Symbol(SymbolId::new(0))));
}

#[test]
fn const_value_to_slot_value_symbol_max() {
    let result = ConstValue::Symbol(SymbolId::new(u32::MAX)).to_slot_value();
    assert_eq!(result, Ok(SlotValue::Symbol(SymbolId::new(u32::MAX))));
}

#[test]
fn slot_value_equality_is_reflexive_for_null() {
    assert_eq!(SlotValue::Null, SlotValue::Null);
}

#[test]
fn slot_value_equality_distinguishes_null_from_bool_false() {
    assert_ne!(SlotValue::Null, SlotValue::Bool(false));
}

#[test]
fn slot_value_equality_distinguishes_i64_zero_from_f64_zero() {
    // SlotValue::I64(0) and SlotValue::F64(FiniteF64(0.0)) are different variants.
    let result = FiniteF64::new(0.0);
    assert_eq!(result.as_ref().map(|f| f.get()), Ok(0.0));
    let finite = result.expect("setup: zero must be finite");
    assert_ne!(SlotValue::I64(0), SlotValue::F64(finite));
}

#[test]
fn slot_value_equality_distinguishes_symbol_from_list() {
    assert_ne!(
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::List(ListId::new(0))
    );
}

// =========================================================================
// Additional edge-case tests — construction, equality, Debug, Display
// =========================================================================

#[test]
fn slot_value_null_debug_format() {
    let val = SlotValue::Null;
    assert!(
        format!("{val:?}").contains("Null"),
        "Debug for Null must contain 'Null'"
    );
}

#[test]
fn slot_value_bool_debug_format() {
    let val = SlotValue::Bool(true);
    let debug = format!("{val:?}");
    assert!(debug.contains("Bool"), "Debug for Bool must contain 'Bool'");
}

#[test]
fn slot_value_i64_debug_format() {
    let val = SlotValue::I64(-99);
    let debug = format!("{val:?}");
    assert!(debug.contains("I64"), "Debug for I64 must contain 'I64'");
}

#[test]
fn slot_value_null_display_is_null() {
    assert_eq!(format!("{}", SlotValue::Null), "null");
}

#[test]
fn slot_value_bool_display_true() {
    assert_eq!(format!("{}", SlotValue::Bool(true)), "true");
}

#[test]
fn slot_value_bool_display_false() {
    assert_eq!(format!("{}", SlotValue::Bool(false)), "false");
}

#[test]
fn slot_value_i64_display() {
    assert_eq!(format!("{}", SlotValue::I64(42)), "42");
}

#[test]
fn slot_value_i64_negative_display() {
    assert_eq!(format!("{}", SlotValue::I64(-1)), "-1");
}

#[test]
fn slot_value_symbol_display() {
    let val = SlotValue::Symbol(SymbolId::new(7));
    assert_eq!(format!("{val}"), "symbol:7");
}

#[test]
fn slot_value_list_display() {
    let val = SlotValue::List(ListId::new(3));
    assert_eq!(format!("{val}"), "list:3");
}

#[test]
fn slot_value_object_display() {
    let val = SlotValue::Object(ObjectId::new(5));
    assert_eq!(format!("{val}"), "object:5");
}

#[test]
fn slot_value_blob_display() {
    let val = SlotValue::Blob(BlobId::new(9));
    assert_eq!(format!("{val}"), "blob:9");
}

#[test]
fn slot_value_i64_equality_same() {
    assert_eq!(SlotValue::I64(0), SlotValue::I64(0));
    assert_eq!(SlotValue::I64(-1), SlotValue::I64(-1));
    assert_eq!(SlotValue::I64(i64::MAX), SlotValue::I64(i64::MAX));
}

#[test]
fn slot_value_i64_inequality_different() {
    assert_ne!(SlotValue::I64(0), SlotValue::I64(1));
    assert_ne!(SlotValue::I64(-1), SlotValue::I64(1));
}

#[test]
fn slot_value_bool_equality() {
    assert_eq!(SlotValue::Bool(true), SlotValue::Bool(true));
    assert_eq!(SlotValue::Bool(false), SlotValue::Bool(false));
    assert_ne!(SlotValue::Bool(true), SlotValue::Bool(false));
}

#[test]
fn slot_value_copy_preserves_equality() {
    let a = SlotValue::I64(42);
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn slot_value_clone_preserves_equality() {
    let a = SlotValue::Bool(true);
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn const_value_null_equality() {
    assert_eq!(ConstValue::Null, ConstValue::Null);
}

#[test]
fn const_value_bool_equality() {
    assert_eq!(ConstValue::Bool(true), ConstValue::Bool(true));
    assert_eq!(ConstValue::Bool(false), ConstValue::Bool(false));
    assert_ne!(ConstValue::Bool(true), ConstValue::Bool(false));
}

#[test]
fn const_value_i64_equality() {
    assert_eq!(ConstValue::I64(0), ConstValue::I64(0));
    assert_eq!(ConstValue::I64(i64::MAX), ConstValue::I64(i64::MAX));
    assert_eq!(ConstValue::I64(i64::MIN), ConstValue::I64(i64::MIN));
    assert_ne!(ConstValue::I64(0), ConstValue::I64(1));
}

#[test]
fn const_value_symbol_equality() {
    assert_eq!(
        ConstValue::Symbol(SymbolId::new(0)),
        ConstValue::Symbol(SymbolId::new(0))
    );
    assert_ne!(
        ConstValue::Symbol(SymbolId::new(0)),
        ConstValue::Symbol(SymbolId::new(1))
    );
}

#[test]
fn const_value_distinguishes_null_from_bool_false() {
    assert_ne!(ConstValue::Null, ConstValue::Bool(false));
}

#[test]
fn const_value_distinguishes_i64_from_symbol() {
    assert_ne!(ConstValue::I64(0), ConstValue::Symbol(SymbolId::new(0)));
}

#[test]
fn const_value_to_slot_value_null_preserves_equality() {
    assert_eq!(ConstValue::Null.to_slot_value(), Ok(SlotValue::Null));
}

#[test]
fn finite_f64_display_matches_inner() -> Result<(), String> {
    let val = FiniteF64::new(3.14).map_err(|e| e.to_string())?;
    let display = format!("{val}");
    assert!(
        display.contains("3.14"),
        "display must contain the value, got: {display}"
    );
    Ok(())
}

#[test]
fn taint_debug_format_variants() {
    let clean_debug = format!("{:?}", Taint::Clean);
    assert!(clean_debug.contains("Clean"));
    let secret_debug = format!("{:?}", Taint::Secret);
    assert!(secret_debug.contains("Secret"));
    let derived_debug = format!("{:?}", Taint::DerivedFromSecret);
    assert!(derived_debug.contains("DerivedFromSecret"));
}

#[test]
fn taint_equality_reflexive() {
    assert_eq!(Taint::Clean, Taint::Clean);
    assert_eq!(Taint::Secret, Taint::Secret);
    assert_eq!(Taint::DerivedFromSecret, Taint::DerivedFromSecret);
}

#[test]
fn taint_inequality() {
    assert_ne!(Taint::Clean, Taint::Secret);
    assert_ne!(Taint::Clean, Taint::DerivedFromSecret);
    assert_ne!(Taint::DerivedFromSecret, Taint::Secret);
}

#[test]
fn slot_value_f64_with_positive_zero() -> Result<(), String> {
    let finite = FiniteF64::new(0.0).map_err(|e| e.to_string())?;
    let val = SlotValue::F64(finite);
    assert_eq!(val.type_name(), "number");
    assert!(!val.is_true());
    Ok(())
}

#[test]
fn const_value_debug_format() {
    let debug = format!("{:?}", ConstValue::I64(42));
    assert!(
        debug.contains("I64"),
        "Debug for ConstValue::I64 must contain 'I64'"
    );
    let debug = format!("{:?}", ConstValue::Null);
    assert!(
        debug.contains("Null"),
        "Debug for ConstValue::Null must contain 'Null'"
    );
}

#[test]
fn slot_value_all_variants_distinct_type_names() {
    // Ensure each handle variant has a distinct type name
    assert_ne!(
        SlotValue::Symbol(SymbolId::new(0)).type_name(),
        SlotValue::List(ListId::new(0)).type_name()
    );
    assert_ne!(
        SlotValue::List(ListId::new(0)).type_name(),
        SlotValue::Object(ObjectId::new(0)).type_name()
    );
    assert_ne!(
        SlotValue::Object(ObjectId::new(0)).type_name(),
        SlotValue::Blob(BlobId::new(0)).type_name()
    );
}

// -- SlotValueDisplay / display_with_store tests --

#[test]
fn display_with_store_null_returns_null() {
    let store = ValueStore::new();
    assert_eq!(SlotValue::Null.display_with_store(&store), "null");
}

#[test]
fn display_with_store_bool_true_returns_true() {
    let store = ValueStore::new();
    assert_eq!(SlotValue::Bool(true).display_with_store(&store), "true");
}

#[test]
fn display_with_store_i64_returns_number() {
    let store = ValueStore::new();
    assert_eq!(SlotValue::I64(42).display_with_store(&store), "42");
}

#[test]
fn display_with_store_symbol_resolves() {
    let mut store = ValueStore::new();
    let id = store.insert_symbol("hello").expect("insert");
    assert_eq!(
        SlotValue::Symbol(id).display_with_store(&store),
        "symbol:hello"
    );
}

#[test]
fn display_with_store_symbol_out_of_bounds_falls_back() {
    let store = ValueStore::new();
    assert_eq!(
        SlotValue::Symbol(SymbolId::new(99)).display_with_store(&store),
        "symbol:99"
    );
}

#[test]
fn display_with_store_list_resolves() {
    let mut store = ValueStore::new();
    let id = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
        .expect("insert");
    assert_eq!(SlotValue::List(id).display_with_store(&store), "[1, 2]");
}

#[test]
fn display_with_store_list_out_of_bounds_falls_back() {
    let store = ValueStore::new();
    assert_eq!(
        SlotValue::List(ListId::new(99)).display_with_store(&store),
        "list:99"
    );
}

#[test]
fn display_with_store_object_resolves() {
    let mut store = ValueStore::new();
    // Insert the field key as a symbol so it resolves during display.
    let _sym_id = store.insert_symbol("field_key").expect("insert");
    let id = store
        .insert_object(
            vec![crate::value_store::ObjectField {
                key: SymbolId::new(0),
                value: SlotValue::I64(42),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .expect("insert");
    let result = SlotValue::Object(id).display_with_store(&store);
    assert_eq!(result, "{field_key: 42}");
}

#[test]
fn display_with_store_object_out_of_bounds_falls_back() {
    let store = ValueStore::new();
    assert_eq!(
        SlotValue::Object(ObjectId::new(99)).display_with_store(&store),
        "object:99"
    );
}

#[test]
fn display_with_store_blob_resolves() {
    let mut store = ValueStore::new();
    let id = store
        .insert_blob(bytes::Bytes::from_static(b"abc"))
        .expect("insert");
    assert_eq!(
        SlotValue::Blob(id).display_with_store(&store),
        "blob:<3 bytes>"
    );
}

#[test]
fn display_with_store_blob_out_of_bounds_falls_back() {
    let store = ValueStore::new();
    assert_eq!(
        SlotValue::Blob(BlobId::new(99)).display_with_store(&store),
        "blob:99"
    );
}

#[test]
fn display_with_store_nested_list() {
    let mut store = ValueStore::new();
    let inner = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .expect("insert");
    let outer = store
        .insert_list(vec![SlotValue::List(inner)].into_boxed_slice())
        .expect("insert");
    assert_eq!(SlotValue::List(outer).display_with_store(&store), "[[1]]");
}
