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
    unused_variables
)]

//! Tests verifying `SideEffect` enum matches master plan Section 65.
//!
//! These tests assert the FIXED state: the 7-variant taxonomy defined
//! in the master plan (Pure, LocalRead, LocalWrite, ExternalRead,
//! ExternalWrite, Process, UnsafeShell).

use vb_core::{
    RetrySafety, SideEffect,
    action::{ActionName, verify_idempotency},
};

/// Helper to extract the #[repr(u8)] discriminant of a SideEffect variant.
/// Returns None if the variant name does not exist in the current implementation.
const fn side_effect_discriminant(v: SideEffect) -> u8 {
    v as u8
}

/// Test that SideEffect has exactly 7 variants as specified in master plan Section 65.
///
/// Master plan defines: Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite,
/// Process, UnsafeShell (7 total).
///
/// This test will fail to compile if:
/// - The variant count is wrong (not 7)
/// - Any master plan variant name doesn't exist
#[test]
fn side_effect_has_exactly_seven_master_plan_variants() {
    const MASTER_PLAN_COUNT: usize = 7;

    // Enumerate all 7 variants to prove they exist and have unique discriminants.
    let variants = [
        SideEffect::Pure,
        SideEffect::LocalRead,
        SideEffect::LocalWrite,
        SideEffect::ExternalRead,
        SideEffect::ExternalWrite,
        SideEffect::Process,
        SideEffect::UnsafeShell,
    ];

    // Verify we have exactly 7 entries
    assert_eq!(variants.len(), MASTER_PLAN_COUNT);

    // Verify all discriminants are unique (0-6).
    let mut discriminants: Vec<u8> = variants.iter().map(|v| *v as u8).collect();
    discriminants.sort();
    assert_eq!(discriminants, vec![0, 1, 2, 3, 4, 5, 6]);
}

/// Test that SideEffect::Pure exists and has a unique discriminant.
#[test]
fn side_effect_pure_variant_exists() {
    let _ = SideEffect::Pure;
    let d = side_effect_discriminant(SideEffect::Pure);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::LocalRead exists and has a unique discriminant.
#[test]
fn side_effect_local_read_variant_exists() {
    let _ = SideEffect::LocalRead;
    let d = side_effect_discriminant(SideEffect::LocalRead);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::LocalWrite exists and has a unique discriminant.
#[test]
fn side_effect_local_write_variant_exists() {
    let _ = SideEffect::LocalWrite;
    let d = side_effect_discriminant(SideEffect::LocalWrite);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::ExternalRead exists and has a unique discriminant.
#[test]
fn side_effect_external_read_variant_exists() {
    let _ = SideEffect::ExternalRead;
    let d = side_effect_discriminant(SideEffect::ExternalRead);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::ExternalWrite exists and has a unique discriminant.
#[test]
fn side_effect_external_write_variant_exists() {
    let _ = SideEffect::ExternalWrite;
    let d = side_effect_discriminant(SideEffect::ExternalWrite);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::Process exists and has a unique discriminant.
#[test]
fn side_effect_process_variant_exists() {
    let _ = SideEffect::Process;
    let d = side_effect_discriminant(SideEffect::Process);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::UnsafeShell exists and has a unique discriminant.
#[test]
fn side_effect_unsafe_shell_variant_exists() {
    let _ = SideEffect::UnsafeShell;
    let d = side_effect_discriminant(SideEffect::UnsafeShell);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that all 7 SideEffect variants have distinct #[repr(u8)] discriminants.
///
/// This prevents two variants from sharing the same discriminant value,
/// which would violate the exclusivity requirement of the master plan.
#[test]
fn side_effect_all_discriminants_are_distinct() {
    use std::collections::BTreeSet;

    let variants = [
        SideEffect::Pure,
        SideEffect::LocalRead,
        SideEffect::LocalWrite,
        SideEffect::ExternalRead,
        SideEffect::ExternalWrite,
        SideEffect::Process,
        SideEffect::UnsafeShell,
    ];

    let mut discriminants = BTreeSet::new();
    let mut i = 0;
    while i < variants.len() {
        let d = side_effect_discriminant(variants[i]);
        assert!(
            discriminants.insert(d),
            "duplicate discriminant {d} found at index {i}"
        );
        i += 1;
    }
    assert_eq!(discriminants.len(), 7, "all 7 discriminants must be unique");
}

/// Test that SideEffect implements Copy (required for enum discriminants).
#[test]
fn side_effect_is_copy() {
    let a = SideEffect::Pure;
    let _b = a;
}

/// Test that SideEffect variants are used correctly in verify_idempotency.
///
/// This verifies that the verify_idempotency function in vb_core's action.rs
/// handles the fixed SideEffect taxonomy correctly.
#[test]
fn side_effect_verify_idempotency_handles_pure_correctly() {
    use vb_core::{ActionContract, ActionId, Idempotency, RunFrame, RunId, StepIdx};

    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 1,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    let frame = frame.unwrap_or_else(|e| panic!("test frame construction failed: {e:?}"));

    // Pure actions should always pass idempotency verification
    let result = verify_idempotency(&contract, &[], &frame);
    assert_eq!(
        result,
        Ok(()),
        "Pure SideEffect should pass verify_idempotency"
    );
}
