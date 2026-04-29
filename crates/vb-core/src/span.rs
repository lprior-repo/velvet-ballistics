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
}
