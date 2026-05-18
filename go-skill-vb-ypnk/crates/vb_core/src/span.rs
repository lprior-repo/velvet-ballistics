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

/// Placeholder source map until later phases attach workflow sources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMap {
    _private: (),
}

impl SourceMap {
    /// Creates an empty source map placeholder.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }
}

#[cfg(test)]
mod tests {
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
