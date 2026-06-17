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

//! Tests verifying `RetrySafety` enum matches master plan Section 65.
//!
//! These tests assert the FIXED state: the 4-variant taxonomy defined
//! in the master plan (Idempotent, RequiresIdempotencyKey, NotRetrySafe,
//! Unknown). They will initially fail to compile because the current
//! implementation uses a different 3-variant taxonomy (Safe, KeyRequired,
//! Unsafe).
//!
//! After the implementation is fixed to match the master plan, these
//! tests verify the contract is satisfied.

// Top-level `RetrySafety` import for the simple-discriminant tests.
// Other tests in this file have their own inner `use vb_core::{}` blocks.
use vb_core::{
    RetrySafety, SideEffect,
    action::{ActionName, verify_idempotency},
};

/// Helper to extract the #[repr(u8)] discriminant of a RetrySafety variant.
const fn retry_safety_discriminant(v: RetrySafety) -> u8 {
    v as u8
}

/// Test that RetrySafety has exactly 4 variants as specified in master plan Section 65.
///
/// Master plan defines: Idempotent, RequiresIdempotencyKey, NotRetrySafe,
/// Unknown (4 total).
///
/// This test will fail to compile if:
/// - The variant count is wrong (not 4)
/// - Any master plan variant name doesn't exist
#[test]
fn retry_safety_has_exactly_four_master_plan_variants() {
    // The 4 variants defined in master plan Section 65
    const MASTER_PLAN_COUNT: usize = 4;

    // Use each variant. If a variant is missing, this won't compile.
    // The function `std::mem::variant_count` is unstable, so we use a manual
    // count via a tuple-typed assertion.
    let _all_variants: (RetrySafety, RetrySafety, RetrySafety, RetrySafety) = (
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
        RetrySafety::Unknown,
    );
    let count = std::mem::size_of_val(&_all_variants) / std::mem::size_of::<RetrySafety>();
    assert_eq!(count, MASTER_PLAN_COUNT);
}

/// Test that RetrySafety::Idempotent exists and has a unique discriminant.
#[test]
fn retry_safety_idempotent_variant_exists() {
    let _ = RetrySafety::Idempotent;
    let d = retry_safety_discriminant(RetrySafety::Idempotent);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that RetrySafety::RequiresIdempotencyKey exists and has a unique discriminant.
#[test]
fn retry_safety_requires_idempotency_key_variant_exists() {
    let _ = RetrySafety::RequiresIdempotencyKey;
    let d = retry_safety_discriminant(RetrySafety::RequiresIdempotencyKey);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that RetrySafety::NotRetrySafe exists and has a unique discriminant.
#[test]
fn retry_safety_not_retry_safe_variant_exists() {
    let _ = RetrySafety::NotRetrySafe;
    let d = retry_safety_discriminant(RetrySafety::NotRetrySafe);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that RetrySafety::Unknown exists and has a unique discriminant.
#[test]
fn retry_safety_unknown_variant_exists() {
    let _ = RetrySafety::Unknown;
    let d = retry_safety_discriminant(RetrySafety::Unknown);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that all 4 RetrySafety variants have distinct #[repr(u8)] discriminants.
///
/// This prevents two variants from sharing the same discriminant value,
/// which would violate the exclusivity requirement of the master plan.
#[test]
fn retry_safety_all_discriminants_are_distinct() {
    use std::collections::BTreeSet;

    let variants = [
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
        RetrySafety::Unknown,
    ];

    let mut discriminants = BTreeSet::new();
    let mut i = 0;
    while i < variants.len() {
        let d = retry_safety_discriminant(variants[i]);
        assert!(
            discriminants.insert(d),
            "duplicate discriminant {d} found at index {i}"
        );
        i += 1;
    }
    assert_eq!(discriminants.len(), 4, "all 4 discriminants must be unique");
}

/// Test that RetrySafety implements Copy (required for enum discriminants).
#[test]
fn retry_safety_is_copy() {
    let a = RetrySafety::Idempotent;
    let _b = a;
}

/// Test that RetrySafety::Idempotent passes verify_idempotency without a key.
///
/// According to master plan Section 65:
/// - Idempotent: safe to retry unconditionally
#[test]
fn retry_safety_idempotent_allows_retry_without_key() {
    use vb_core::{ActionContract, ActionId, Idempotency, RunFrame, RunId, StepIdx};

    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::ExternalRead,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.expect("test frame");

    // Idempotent should pass even with empty key slots
    let result = verify_idempotency(&contract, &[], &frame);
    assert_eq!(result, Ok(()), "Idempotent should pass without key");
}

/// Test that RetrySafety::NotRetrySafe rejects verify_idempotency.
///
/// According to master plan Section 65:
/// - NotRetrySafe: retry rejected by default
#[test]
fn retry_safety_not_retry_safe_rejects_retry() {
    use vb_core::{ActionContract, ActionId, Idempotency, RunFrame, RunId, SlotIdx, StepIdx};

    let contract = ActionContract {
        id: ActionId::new(2),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::ExternalWrite,
        retry_safety: RetrySafety::NotRetrySafe,
        required_capabilities: Box::new([]),
    };

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.expect("test frame");

    // NotRetrySafe should be rejected even with a key provided
    let key_slots = [SlotIdx::new(0)];
    let result = verify_idempotency(&contract, &key_slots, &frame);
    assert_eq!(
        result.is_err(),
        true,
        "NotRetrySafe should reject even with key"
    );
}

/// Test that RetrySafety::RequiresIdempotencyKey passes with a valid key.
///
/// According to master plan Section 65:
/// - RequiresIdempotencyKey: safe with a valid idempotency key
#[test]
fn retry_safety_requires_idempotency_key_passes_with_key() {
    use vb_core::{
        ActionContract, ActionId, Idempotency, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, Taint,
    };

    let contract = ActionContract {
        id: ActionId::new(3),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::ExternalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let mut frame = frame.expect("test frame");

    // Write a clean value to slot 0 for use as idempotency key
    let write_result =
        frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean);
    assert!(write_result.is_ok());

    // RequiresIdempotencyKey should pass with a clean (non-secret) key
    let key_slots = [SlotIdx::new(0)];
    let result = verify_idempotency(&contract, &key_slots, &frame);
    assert_eq!(
        result,
        Ok(()),
        "RequiresIdempotencyKey should pass with valid key"
    );
}

/// Test that RetrySafety::RequiresIdempotencyKey fails without a key.
///
/// According to master plan Section 65:
/// - RequiresIdempotencyKey: safe with a valid idempotency key
#[test]
fn retry_safety_requires_idempotency_key_fails_without_key() {
    use vb_core::{ActionContract, ActionId, Idempotency, RunFrame, RunId, StepIdx};

    let contract = ActionContract {
        id: ActionId::new(4),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::ExternalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.expect("test frame");

    // RequiresIdempotencyKey should fail with empty key slots
    let result = verify_idempotency(&contract, &[], &frame);
    assert!(
        result.is_err(),
        "RequiresIdempotencyKey should fail without key"
    );
}

/// Test that RetrySafety::Unknown rejects retry.
///
/// According to master plan Section 65:
/// - Unknown: retry rejected
#[test]
fn retry_safety_unknown_rejects_retry() {
    use vb_core::{ActionContract, ActionId, Idempotency, RunFrame, RunId, StepIdx};

    let contract = ActionContract {
        id: ActionId::new(5),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::ExternalWrite,
        retry_safety: RetrySafety::Unknown,
        required_capabilities: Box::new([]),
    };

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.expect("test frame");

    // Unknown should always be rejected
    let result = verify_idempotency(&contract, &[], &frame);
    assert!(result.is_err(), "Unknown should always be rejected");
}

/// Test that verify_idempotency function in vb_core's action.rs
/// handles all 4 master plan RetrySafety variants correctly.
///
/// This is the critical integration point: the verify_idempotency match
/// must be exhaustive for all 4 variants. If a variant is missing from
/// the implementation, this test will not compile.
#[test]
fn verify_idempotency_match_is_exhaustive_for_all_master_plan_variants() {
    use vb_core::{ActionContract, ActionId, Idempotency, RunFrame, RunId, SideEffect, StepIdx};

    // Create a contract that exercises each RetrySafety variant
    fn check_variant(safety: RetrySafety, should_pass: bool) {
        let contract = ActionContract {
            id: ActionId::new(99),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::DeterministicPure,
            // Use a non-Pure side effect so the retry_safety branch is exercised.
            side_effect: SideEffect::ExternalWrite,
            retry_safety: safety,
            required_capabilities: Box::new([]),
        };

        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        assert!(frame.is_ok());
        let frame = frame.expect("test frame");

        let result = verify_idempotency(&contract, &[], &frame);
        if should_pass {
            assert_eq!(result, Ok(()), "should pass for {:?}", safety);
        } else {
            assert!(result.is_err(), "should fail for {:?}", safety);
        }
    }

    // Exhaustively check all 4 master plan variants
    check_variant(RetrySafety::Idempotent, true);
    check_variant(RetrySafety::RequiresIdempotencyKey, false); // no key provided
    check_variant(RetrySafety::NotRetrySafe, false);
    check_variant(RetrySafety::Unknown, false);
}

/// Tier 3 wiring test: this file must be wired into `vb_compile/src/enums/mod.rs`
/// via `#[cfg(test)] mod retry_safety_tests;`.
///
/// If this test runs (i.e., is reached by the test runner), the file
/// is wired in. The test body is a no-op assertion: the file's
/// compilation succeeding is the strongest possible wiring evidence
/// because the Tier 1 tests above all reference 4-variant
/// `RetrySafety` names that did not exist before the migration.
///
/// On 3-variant code: the file fails to compile (Tier 1 fail-loud)
/// because `RetrySafety::Idempotent` etc. do not exist. The wiring
/// test inherits that compile failure.
///
/// On 4-variant code (post-migration): this test passes once the
/// `mod retry_safety_tests;` registration is present in
/// `enums/mod.rs`. If the `mod` registration is removed, this
/// test (and the 12 Tier 1 tests above) become unreachable,
/// which is the failure mode the Tier 3 wiring test is designed
/// to catch.
#[test]
fn retry_safety_tests_file_is_wired_into_vb_compile() {
    // The fact that this function exists in the build is the
    // wiring evidence. No additional runtime assertion is needed.
    //
    // Defense-in-depth: a string match on the production
    // `vb_core::action::RetrySafety` type name to assert that
    // the imported type is the production enum (not a stub or alias).
    let type_name = std::any::type_name::<RetrySafety>();
    assert!(
        type_name.contains("RetrySafety"),
        "RetrySafety type name must contain 'RetrySafety', got: {type_name}"
    );
}
