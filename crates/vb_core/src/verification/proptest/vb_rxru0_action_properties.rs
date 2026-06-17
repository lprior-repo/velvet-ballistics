//! Proptest properties for vb_core action module.
//!
//! Verifier lane: proptest
//! Obligations: OBL-007, OBL-008, OBL-NEW-PS-004
//!
//! Tests Copy/Clone preservation, ActionTicket field mapping,
//! and idempotency key determinism.

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
    unused,
    unused_imports,
    unused_variables,
)]

use proptest::prelude::*;

use crate::action::{self, ActionTicket, Idempotency, RetrySafety, SideEffect};
use crate::ids::{ActionId, RunId, SeqNo, StepIdx};

prop_compose! {
    fn arb_u64_nonmax()(val in 0..u64::MAX) -> u64 {
        val
    }
}

prop_compose! {
    fn arb_run_id()(val in arb_u64_nonmax()) -> RunId {
        RunId::new(val)
    }
}

prop_compose! {
    fn arb_step_id()(val in 0u16..u16::MAX) -> StepIdx {
        StepIdx::new(val)
    }
}

prop_compose! {
    fn arb_seq()(val in arb_u64_nonmax()) -> SeqNo {
        SeqNo::new(val)
    }
}

prop_compose! {
    fn arb_action_id()(val in 0u16..u16::MAX) -> ActionId {
        ActionId::new(val)
    }
}

// Custom arbitraries for enums that don't implement Arbitrary.

prop_compose! {
    fn arb_idempotency()(disc in 0u8..3) -> Idempotency {
        match disc {
            0 => Idempotency::DeterministicPure,
            1 => Idempotency::IdempotentExternal,
            _ => Idempotency::AtLeastOnceExternal,
        }
    }
}

prop_compose! {
    fn arb_side_effect()(disc in 0u8..7) -> SideEffect {
        match disc {
            0 => SideEffect::Pure,
            1 => SideEffect::LocalRead,
            2 => SideEffect::LocalWrite,
            3 => SideEffect::ExternalRead,
            4 => SideEffect::ExternalWrite,
            5 => SideEffect::Process,
            _ => SideEffect::UnsafeShell,
        }
    }
}

prop_compose! {
    fn arb_retry_safety()(disc in 0u8..4) -> RetrySafety {
        match disc {
            0 => RetrySafety::Idempotent,
            1 => RetrySafety::RequiresIdempotencyKey,
            2 => RetrySafety::NotRetrySafe,
            _ => RetrySafety::Unknown,
        }
    }
}

// ─── OBL-007: Copy/Clone preservation for enum types ───────────────────────────

proptest! {
    /// Idempotency is Copy and Clone.
    #[test]
    fn test_idempotency_is_copy_clone(idemp in arb_idempotency()) {
        let cloned = idemp.clone();
        let copied = idemp;
        prop_assert_eq!(copied, cloned);
    }

    /// SideEffect is Copy and Clone.
    #[test]
    fn test_side_effect_is_copy_clone(side in arb_side_effect()) {
        let cloned = side.clone();
        prop_assert_eq!(side, cloned);
    }

    /// RetrySafety is Copy and Clone.
    #[test]
    fn test_retry_safety_is_copy_clone(safety in arb_retry_safety()) {
        let cloned = safety.clone();
        prop_assert_eq!(safety, cloned);
    }

    /// ActionTicket is Copy and Clone.
    #[test]
    fn test_action_ticket_is_copy_clone(
        run in arb_run_id(),
        step in arb_step_id(),
        seq in arb_seq(),
        action in arb_action_id(),
        attempt in any::<u16>(),
        idempotency_key in any::<u128>(),
        capacity in any::<u16>(),
    ) {
        let ticket = ActionTicket {
            run, step, seq, action, attempt, idempotency_key, capacity,
            ..Default::default()
        };
        let cloned = ticket.clone();
        prop_assert_eq!(ticket, cloned);
    }

    // ─── OBL-008: ActionTicket field mapping ────────────────────────────────────

    /// issue_action_ticket produces a ticket where all fields match the inputs.
    #[test]
    fn test_issue_action_ticket_field_mapping(
        run in arb_run_id(),
        step in arb_step_id(),
        seq in arb_seq(),
        action in arb_action_id(),
        attempt in any::<u16>(),
        idempotency_key in any::<u128>(),
        capacity in any::<u16>(),
    ) {
        let ticket = action::issue_action_ticket(
            run, step, seq, action, attempt, idempotency_key, capacity,
        );

        prop_assert_eq!(ticket.run, run);
        prop_assert_eq!(ticket.step, step);
        prop_assert_eq!(ticket.seq, seq);
        prop_assert_eq!(ticket.action, action);
        prop_assert_eq!(ticket.attempt, attempt);
        prop_assert_eq!(ticket.idempotency_key, idempotency_key);
        prop_assert_eq!(ticket.capacity, capacity);
    }

    // ─── OBL-NEW-PS-004: Idempotency key determinism ────────────────────────────

    /// compute_action_idempotency_key is deterministic: same inputs always produce the same key.
    #[test]
    fn test_idempotency_key_determinism(
        run in arb_run_id(),
        seq in arb_seq(),
        action in arb_action_id(),
    ) {
        let key1 = action::compute_action_idempotency_key(run, seq, action);
        let key2 = action::compute_action_idempotency_key(run, seq, action);
        prop_assert_eq!(key1, key2);
    }

    /// compute_action_idempotency_key is injective on (run, seq, action) for small values.
    #[test]
    fn test_idempotency_key_injectivity_small_values(
        run_a in 0u64..100,
        seq_a in 0u64..100,
        action_a in 0u16..100,
        run_b in 0u64..100,
        seq_b in 0u64..100,
        action_b in 0u16..100,
    ) {
        let key_a = action::compute_action_idempotency_key(
            RunId::new(run_a),
            SeqNo::new(seq_a),
            ActionId::new(action_a),
        );
        let key_b = action::compute_action_idempotency_key(
            RunId::new(run_b),
            SeqNo::new(seq_b),
            ActionId::new(action_b),
        );
        if (run_a, seq_a, action_a) != (run_b, seq_b, action_b) {
            prop_assert_ne!(key_a, key_b, "Different inputs should produce different keys");
        }
    }

    /// action_ticket_has_valid_key returns true for a ticket with the correct key.
    #[test]
    fn test_valid_key_returns_true(
        run in arb_run_id(),
        seq in arb_seq(),
        action in arb_action_id(),
        step in arb_step_id(),
        attempt in any::<u16>(),
        capacity in any::<u16>(),
    ) {
        let correct_key = action::compute_action_idempotency_key(run, seq, action);
        let ticket = ActionTicket {
            run, step, seq, action, attempt, idempotency_key: correct_key, capacity,
            ..Default::default()
        };
        prop_assert!(action::action_ticket_has_valid_key(ticket));
    }

    /// action_ticket_has_valid_key returns false for a ticket with an incorrect key.
    #[test]
    fn test_invalid_key_returns_false(
        run in arb_run_id(),
        seq in arb_seq(),
        action in arb_action_id(),
        step in arb_step_id(),
        attempt in any::<u16>(),
        capacity in any::<u16>(),
    ) {
        let wrong_key: u128 = 0xBAD;
        let ticket = ActionTicket {
            run, step, seq, action, attempt, idempotency_key: wrong_key, capacity,
            ..Default::default()
        };
        prop_assert!(!action::action_ticket_has_valid_key(ticket));
    }
}
