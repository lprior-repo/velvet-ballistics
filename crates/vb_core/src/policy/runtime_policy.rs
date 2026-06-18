#![forbid(unsafe_code)]
//! Runtime admission policy controlling verification strictness.
//!
//! Three policies (`Strict`, `Journaled`, `Relaxed`) control how
//! artifact admission verification is enforced during run execution.

/// Controls how strictly artifact admission verification is enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum RuntimePolicy {
    /// Require accepted artifact for all runs, SyncAll before return.
    Strict,
    /// Accept runs without artifact, queue events without sync barrier.
    Journaled,
    /// No verification required, testing only.
    Relaxed,
}

#[cfg(test)]
mod tests {
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
        unused_imports,
        dead_code,
        unused_variables
    )]

    use super::RuntimePolicy;

    #[test]
    fn policy_variants_are_distinct() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Journaled);
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Relaxed);
        assert_ne!(RuntimePolicy::Journaled, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_copy_semantics_preserve_equality() {
        let a = RuntimePolicy::Strict;
        let b = a;
        assert_eq!(a, b, "copy must preserve equality");
    }

    #[test]
    fn policy_strict_is_not_journaled() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Journaled);
    }

    #[test]
    fn policy_strict_is_not_relaxed() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_journaled_is_not_relaxed() {
        assert_ne!(RuntimePolicy::Journaled, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_debug_output_contains_variant_name() {
        let formatted = format!("{:?}", RuntimePolicy::Strict);
        assert!(
            formatted.contains("Strict"),
            "debug output must contain variant name: {formatted}"
        );
    }

    #[test]
    fn policy_clone_produces_equal_value() {
        let original = RuntimePolicy::Journaled;
        let cloned = original.clone();
        assert_eq!(original, cloned, "clone must produce equal value");
    }
}
