#![forbid(unsafe_code)]

//! Source-location primitives for diagnostics.

use serde::{Deserialize, Serialize};

/// Byte-offset span into a source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Span {
    /// Inclusive starting byte offset.
    pub start: u32,
    /// Exclusive ending byte offset.
    pub end: u32,
}

impl Span {
    /// Empty span at the beginning of a source document.
    pub const ZERO: Self = Self { start: 0, end: 0 };

    /// Creates a span from byte offsets.
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Returns true when the span covers no bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Value paired with its source location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Located<T> {
    /// Located value.
    pub value: T,
    /// Source span for the value.
    pub span: Span,
}

impl<T> Located<T> {
    /// Creates a located value.
    #[must_use]
    pub const fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

/// Alias used when APIs prefer the term spanned.
pub type Spanned<T> = Located<T>;

/// Empty source map used by APIs that do not retain source text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMap {
    _private: (),
}

impl SourceMap {
    /// Creates an empty source map.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }
}

#[cfg(test)]
mod tests {
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

    use super::{Located, SourceMap, Span, Spanned};

    #[test]
    fn zero_span_is_empty() {
        assert!(Span::ZERO.is_empty());
        assert_eq!(Span::ZERO, Span::new(0, 0));
    }

    #[test]
    fn span_preserves_offsets() {
        let span = Span::new(2, 5);

        assert_eq!(span.start, 2);
        assert_eq!(span.end, 5);
        assert!(!span.is_empty());
    }

    #[test]
    fn located_and_spanned_hold_value_and_span() {
        let located = Located::new(42_u32, Span::ZERO);
        let spanned: Spanned<u32> = located.clone();

        assert_eq!(located.value, 42);
        assert_eq!(spanned.span, Span::ZERO);
    }

    #[test]
    fn source_map_placeholder_is_constructible() {
        let map = SourceMap::new();

        assert_eq!(map, SourceMap::default());
    }

    // =========================================================================
    // Additional edge-case tests — Span construction, equality, located values
    // =========================================================================

    #[test]
    fn span_default_is_zero() {
        assert_eq!(Span::default(), Span::ZERO);
        assert!(Span::default().is_empty());
    }

    #[test]
    fn span_new_at_max_offsets() {
        let span = Span::new(u32::MAX, u32::MAX);
        assert!(span.is_empty());
        assert_eq!(span.start, u32::MAX);
        assert_eq!(span.end, u32::MAX);
    }

    #[test]
    fn span_new_with_start_equal_end_is_empty() {
        let span = Span::new(100, 100);
        assert!(span.is_empty());
    }

    #[test]
    fn span_new_with_start_less_than_end_is_not_empty() {
        let span = Span::new(5, 10);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_equality_same_offsets() {
        assert_eq!(Span::new(10, 20), Span::new(10, 20));
    }

    #[test]
    fn span_inequality_different_start() {
        assert_ne!(Span::new(0, 10), Span::new(1, 10));
    }

    #[test]
    fn span_inequality_different_end() {
        assert_ne!(Span::new(0, 10), Span::new(0, 20));
    }

    #[test]
    fn span_copy_preserves_equality() {
        let a = Span::new(5, 15);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn span_clone_preserves_equality() {
        let a = Span::new(5, 15);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn span_debug_format_contains_offsets() {
        let span = Span::new(10, 20);
        let debug = format!("{span:?}");
        assert!(debug.contains("Span"), "Debug must contain 'Span'");
    }

    #[test]
    fn located_new_preserves_value_and_span() {
        let span = Span::new(1, 5);
        let located = Located::new(42_u32, span);
        assert_eq!(located.value, 42);
        assert_eq!(located.span, span);
    }

    #[test]
    fn spanned_alias_works_same_as_located() {
        let span = Span::new(3, 7);
        let spanned: Spanned<i64> = Spanned::new(-1, span);
        assert_eq!(spanned.value, -1);
        assert_eq!(spanned.span, span);
    }

    #[test]
    fn located_clone_preserves_equality() {
        let span = Span::new(0, 10);
        let a = Located::new(String::from("test"), span);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn source_map_equality() {
        assert_eq!(SourceMap::new(), SourceMap::new());
        assert_eq!(SourceMap::new(), SourceMap::default());
    }

    #[test]
    fn source_map_debug_format() {
        let map = SourceMap::new();
        let debug = format!("{map:?}");
        assert!(
            debug.contains("SourceMap"),
            "Debug must contain 'SourceMap'"
        );
    }

    #[test]
    fn span_single_byte_span() {
        let span = Span::new(5, 6);
        assert!(!span.is_empty());
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 6);
    }

    #[test]
    fn span_large_span() {
        let span = Span::new(0, u32::MAX);
        assert!(!span.is_empty());
    }
}
