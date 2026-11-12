#![forbid(unsafe_code)]

//! Source-location primitives for diagnostics.

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

    /// Creates a span from byte offsets without validation.
    ///
    /// This is the **unchecked** constructor: any pair of `u32` values
    /// is accepted, including inverted inputs where `start > end`. The
    /// resulting `Span` preserves the offsets verbatim, so a downstream
    /// `is_empty()` check, a length computation, or a slice operation
    /// can produce surprising results. New code should prefer
    /// [`Span::try_new`] unless it can prove the invariant by
    /// construction (for example, when carrying offsets forward from
    /// a previously-validated source).
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Creates a span from byte offsets, rejecting inverted inputs.
    ///
    /// Returns `Ok(Span { start, end })` when `start <= end`, including
    /// the empty-span case `start == end`. Returns
    /// `Err(SpanError::StartGreaterThanEnd { start, end })` when
    /// `start > end`, with the offending operands carried verbatim
    /// for diagnostics. This constructor is the safe entry point for
    /// any `Span` whose offsets come from untrusted input (parser
    /// output, user-supplied coordinates, recovered journal data, etc.).
    pub const fn try_new(start: u32, end: u32) -> Result<Self, SpanError> {
        if start > end {
            return Err(SpanError::StartGreaterThanEnd { start, end });
        }
        Ok(Self { start, end })
    }

    /// Returns true when the span covers no bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Failure mode for [`Span::try_new`].
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpanError {
    /// `start > end`; the offset pair describes a negative-length range.
    #[error("span start {start} is greater than end {end}")]
    StartGreaterThanEnd {
        /// Inclusive start offset that exceeded the end offset.
        start: u32,
        /// Exclusive end offset that was smaller than the start offset.
        end: u32,
    },
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
#[path = "span/tests.rs"]
mod tests;
