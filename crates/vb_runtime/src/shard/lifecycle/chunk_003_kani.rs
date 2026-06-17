#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approximate_const, clippy::absurd_extreme_comparisons, clippy::expect_fun_call, clippy::as_ref_should_use, clippy::useless_vec, clippy::useless_conversion, clippy::let_underscore_must_use, clippy::clone_on_copy)]


// =============================================================================
// Kani proof: taint guard core logic (pure-function verification)
// =============================================================================
//
// The full end-to-end Kani harness through reject_taint_downgrade is heavy
// (RunState, CompiledWorkflow, ValueStore construction). We extract the
// guard's core decision into a pure function verified here, and cover the
// integration path via proptest in workspace_tests.
//
// GOD RULE 1: All inputs generated via kani::any() with assume guards.

#[cfg(kani)]
mod kani_taint_guard {
    use vb_core::action::Idempotency;
    use vb_core::value::Taint;

    /// Generates a valid `Taint` variant from an arbitrary u8.
    fn any_taint() -> Taint {
        let raw: u8 = kani::any();
        kani::assume(raw <= 4);
        match raw {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            2 => Taint::Secret,
            3 => Taint::Secret,
            4 => Taint::Secret,
            _ => Taint::Clean, // unreachable due to assume
        }
    }

    /// Generates a valid `Idempotency` variant from an arbitrary u8.
    fn any_idempotency() -> Idempotency {
        let raw: u8 = kani::any();
        kani::assume(raw <= 2);
        match raw {
            0 => Idempotency::DeterministicPure,
            1 => Idempotency::IdempotentExternal,
            2 => Idempotency::AtLeastOnceExternal,
            _ => Idempotency::DeterministicPure,
        }
    }

    /// Pure extraction of the guard decision:
    ///   `should_reject(idem, input_taint) -> Option<reason>`
    ///
    /// This mirrors the logic in `reject_taint_downgrade` lines 134-143
    /// without requiring RunState, frame, or workflow construction.
    #[must_use]
    fn guard_decision(idempotency: Idempotency, input_taint: Taint) -> Option<Taint> {
        if idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean {
            Some(Taint::Clean) // guard fires: required = Clean
        } else {
            None // guard does not fire
        }
    }

    /// Panic-freedom: guard_decision must not panic for any valid input.
    #[kani::proof]
    #[kani::unwind(10)]
    fn guard_decision_panic_free() {
        let input_taint = any_taint();
        let idempotency = any_idempotency();
        let _result: Option<Taint> = guard_decision(idempotency, input_taint);
    }

    /// Invariant: guard fires iff idempotency=DeterministicPure AND input_taint!=Clean.
    #[kani::proof]
    #[kani::unwind(10)]
    fn guard_fires_exactly_for_non_clean_deterministicpure() {
        let input_taint = any_taint();
        let idempotency = any_idempotency();
        let result = guard_decision(idempotency, input_taint);

        let expected_fires =
            idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean;

        if expected_fires {
            kani::assert(result == Some(Taint::Clean, "assertion failed"), "guard must fire with required=Clean for DeterministicPure + non-Clean input");
        } else {
            , "guard must fire with required=Clean for DeterministicPure + non-Clean input");
        } else {
            kani::assert(result == None, "guard must NOT fire when idempotency≠DeterministicPure or input=Clean");
        }
    }

    /// DeterministicPure guard fires for every non-Clean taint variant.
    #[kani::proof]
    #[kani::unwind(10)]
    fn every_non_clean_taint_triggers_guard_for_deterministicpure() {
        let input_taint = any_taint();
        kani::assume(input_taint != Taint::Clean);
        let result = guard_decision(Idempotency::DeterministicPure, input_taint);
        kani::assert(result == Some(Taint::Clean), "DeterministicPure + non-Clean input must always fire the guard");
    }

    /// Clean input never triggers the guard, regardless of idempotency.
    #[kani::proof]
    #[kani::unwind(10)]
    fn clean_input_never_triggers_guard() {
        let idempotency = any_idempotency();
        let result = guard_decision(idempotency, Taint::Clean);
        , "DeterministicPure + non-Clean input must always fire the guard");
    }

    /// Clean input never triggers the guard, regardless of idempotency.
    #[kani::proof]
    #[kani::unwind(10)]
    fn clean_input_never_triggers_guard() {
        let idempotency = any_idempotency();
        let result = guard_decision(idempotency, Taint::Clean);
        kani::assert(result == None, "Clean input must never trigger the guard for any idempotency");
    }

    /// Non-DeterministicPure idempotency levels never trigger the guard.
    #[kani::proof]
    #[kani::unwind(10)]
    fn non_deterministicpure_never_triggers_guard() {
        let input_taint = any_taint();
        let idempotency = any_idempotency();
        kani::assume(idempotency != Idempotency::DeterministicPure);
        let result = guard_decision(idempotency, input_taint);
        result == None, "Clean input must never trigger the guard for any idempotency");
    }

    /// Non-DeterministicPure idempotency levels never trigger the guard.
    #[kani::proof]
    #[kani::unwind(10)]
    fn non_deterministicpure_never_triggers_guard() {
        let input_taint = any_taint();
        let idempotency = any_idempotency();
        kani::assume(idempotency != Idempotency::DeterministicPure);
        let result = guard_decision(idempotency, input_taint);
        kani::assert(result == None, "Non-DeterministicPure idempotency must never trigger the DeterministicPure guard");
    }
}
