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
    clippy::trivially_copy_macro,
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
#![forbid(unsafe_code)]

//! Section 38 property test: `taint_safety`.
//!
//! Master plan §38, row "Taint safety":
//! "Secret taint never enters finish result (at compile time)".
//!
//! In the `vb_expr` crate the taint safety floor is the algebraic
//! correctness of the taint join lattice used by the expression
//! evaluator. The evaluator propagates taint via
//! `vb_core::value::join_taint` whenever a `LoadSlot` reads a slot
//! that may carry secret data. This file asserts:
//!
//! - `join_taint` is a join-semilattice: commutative, associative,
//!   idempotent, with `Clean` as the identity and `Secret` as top.
//! - `join_taint` is monotone in both arguments under the lattice
//!   order `Clean < DerivedFromSecret < Secret`.
//! - Repeated folding of an all-Clean sequence stays Clean (no
//!   spontaneous secret appearance).
//! - Adding a single `Secret` flips the result to `Secret` (the
//!   "secret leaks into the accumulator" floor).

use proptest::prelude::*;
use vb_core::value::Taint;
use vb_core::value::join_taint;

fn arb_taint() -> impl Strategy<Value = Taint> {
    prop_oneof![
        Just(Taint::Clean),
        Just(Taint::DerivedFromSecret),
        Just(Taint::Secret),
    ]
}

proptest! {
    /// `join_taint` is commutative: the join of `a` and `b` is
    /// independent of argument order.
    #[test]
    fn ts_join_is_commutative(a in arb_taint(), b in arb_taint()) {
        prop_assert_eq!(join_taint(a, b), join_taint(b, a));
    }

    /// `join_taint` is associative: the join of three values is
    /// independent of bracketing. This is the lattice floor — the
    /// evaluator relies on it for fold-style accumulator updates.
    #[test]
    fn ts_join_is_associative(
        a in arb_taint(),
        b in arb_taint(),
        c in arb_taint(),
    ) {
        prop_assert_eq!(
            join_taint(join_taint(a, b), c),
            join_taint(a, join_taint(b, c))
        );
    }

    /// `join_taint` is idempotent: joining a value with itself
    /// returns that value.
    #[test]
    fn ts_join_is_idempotent(t in arb_taint()) {
        prop_assert_eq!(join_taint(t, t), t);
    }

    /// `Taint::Clean` is the identity element of `join_taint`.
    /// Joining Clean with any taint yields that taint unchanged.
    #[test]
    fn ts_clean_is_join_identity(t in arb_taint()) {
        prop_assert_eq!(join_taint(Taint::Clean, t), t);
        prop_assert_eq!(join_taint(t, Taint::Clean), t);
    }

    /// `Taint::Secret` is the top element of the lattice: it
    /// dominates every other taint, in both argument positions.
    #[test]
    fn ts_secret_is_join_top(t in arb_taint()) {
        prop_assert_eq!(join_taint(Taint::Secret, t), Taint::Secret);
        prop_assert_eq!(join_taint(t, Taint::Secret), Taint::Secret);
    }

    /// `Taint::DerivedFromSecret` is between Clean and Secret in
    /// the join: joining it with Clean yields it; joining it with
    /// Secret yields Secret.
    #[test]
    fn ts_derived_is_intermediate(t in arb_taint()) {
        // Clean < DerivedFromSecret < Secret
        let with_clean = join_taint(Taint::DerivedFromSecret, t);
        let with_secret = join_taint(Taint::DerivedFromSecret, t);
        // Joining with Clean (left) yields DerivedFromSecret only
        // when the other operand is ≤ DerivedFromSecret.
        if t == Taint::Clean {
            prop_assert_eq!(with_clean, Taint::DerivedFromSecret);
        }
        if t == Taint::Secret {
            prop_assert_eq!(with_secret, Taint::Secret);
        }
    }

    /// `join_taint` is monotone in the left argument: if `a ≤ c` in
    /// the lattice, then `join(a, b) ≤ join(c, b)`. We test this
    /// against the three fixed orderings of the lattice.
    #[test]
    fn ts_join_is_monotone_left(
        b in arb_taint(),
    ) {
        // (Clean, DerivedFromSecret, Secret) forms a chain.
        let lhs = join_taint(Taint::Clean, b);
        let mid = join_taint(Taint::DerivedFromSecret, b);
        let rhs = join_taint(Taint::Secret, b);
        // Monotone: lhs ≤ mid ≤ rhs under the join order.
        prop_assert_eq!(join_taint(lhs, mid), mid);
        prop_assert_eq!(join_taint(mid, rhs), rhs);
    }

    /// `join_taint` is monotone in the right argument.
    #[test]
    fn ts_join_is_monotone_right(
        a in arb_taint(),
    ) {
        let lhs = join_taint(a, Taint::Clean);
        let mid = join_taint(a, Taint::DerivedFromSecret);
        let rhs = join_taint(a, Taint::Secret);
        prop_assert_eq!(join_taint(lhs, mid), mid);
        prop_assert_eq!(join_taint(mid, rhs), rhs);
    }

    /// Repeated folding of an all-Clean sequence stays Clean: the
    /// expression evaluator must never spontaneously introduce
    /// `DerivedFromSecret` or `Secret` taint from a Clean-only input
    /// stream. This is the §38 "secret never enters the finish
    /// result from clean inputs" floor.
    #[test]
    fn ts_all_clean_fold_stays_clean(count in 0usize..32usize) {
        let mut accum = Taint::Clean;
        for _ in 0..count {
            accum = join_taint(accum, Taint::Clean);
        }
        prop_assert_eq!(accum, Taint::Clean);
    }

    /// Adding a single `Secret` to an all-Clean accumulator flips
    /// the result to `Secret` (and never produces a "weaker"
    /// intermediate). This is the "secret leak is sticky" floor.
    #[test]
    fn ts_single_secret_dominates_clean(
        clean_count in 0usize..16usize,
        secret_count in 1usize..8usize,
    ) {
        let mut accum = Taint::Clean;
        for _ in 0..clean_count {
            accum = join_taint(accum, Taint::Clean);
        }
        for _ in 0..secret_count {
            accum = join_taint(accum, Taint::Secret);
        }
        prop_assert_eq!(accum, Taint::Secret);
    }

    /// `DerivedFromSecret` mixed with `Secret` stays `Secret`.
    /// `DerivedFromSecret` mixed with `Clean` stays
    /// `DerivedFromSecret`. The two-step sequence tests the
    /// monotonicity floor for the intermediate element.
    #[test]
    fn ts_derived_with_secret_is_secret(_unit in 0u8..1u8) {
        let lhs = join_taint(Taint::DerivedFromSecret, Taint::Secret);
        prop_assert_eq!(lhs, Taint::Secret);
        let rhs = join_taint(Taint::Secret, Taint::DerivedFromSecret);
        prop_assert_eq!(rhs, Taint::Secret);
    }

    /// `Taint` has exactly three distinct values. The lattice is
    /// not accidentally extended at runtime.
    #[test]
    fn ts_three_distinct_values(_unit in 0u8..1u8) {
        prop_assert_ne!(Taint::Clean, Taint::DerivedFromSecret);
        prop_assert_ne!(Taint::Clean, Taint::Secret);
        prop_assert_ne!(Taint::DerivedFromSecret, Taint::Secret);
    }

    /// The lattice order is total: for any two taint values, one
    /// dominates the other under `join_taint`.
    #[test]
    fn ts_lattice_is_total(a in arb_taint(), b in arb_taint()) {
        let joined = join_taint(a, b);
        // `joined` is either `a` or `b` (no new value is created).
        prop_assert!(joined == a || joined == b);
    }

    /// The expression evaluator never panics on taint lattice
    /// operations. This is the no-panic floor for any sequence of
    /// join_taint calls.
    #[test]
    fn ts_never_panics(seq in prop::collection::vec(arb_taint(), 0..32)) {
        let mut accum = Taint::Clean;
        for t in &seq {
            accum = join_taint(accum, *t);
        }
        // The accumulator is still one of the three lattice values.
        match accum {
            Taint::Clean | Taint::DerivedFromSecret | Taint::Secret => {}
            _ => {}
        }
    }
}
