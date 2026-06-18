//! Domain types: SymbolicCode, DiagnosticCode, Severity, Diagnostic, HasSymbolicCode.
//!
//! These types are pure, zero-allocation where possible, and bound to the
//! registry declared in [`super::codes`].

use crate::span::Span;
use core::fmt;
use core::str::FromStr;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use super::codes::{CODE_REGISTRY, CodeCategory};
use super::helpers::{
    category_from_numeric, is_registered_symbolic, is_supported_code, numeric_to_symbolic,
    symbolic_to_numeric,
};

// ---------------------------------------------------------------------------
// SymbolicCode — primary stable diagnostic identifier
// ---------------------------------------------------------------------------

/// Stable symbolic diagnostic code.
///
/// A `SymbolicCode` always contains a registered diagnostic code name.
/// It is `Copy`, zero-allocation, `Send` + `Sync`, and cannot represent
/// invalid or unregistered codes.
///
/// # Construction
///
/// Use [`SymbolicCode::from_static`] to construct from a `&'static str`.
/// Construction succeeds only if the string is in [`CODE_REGISTRY`].
///
/// # Display
///
/// Formats as the symbolic name (e.g., `"DUPLICATE_KEY"`), not the E-hex form.
///
/// # Serialization
///
/// `Serialize` outputs the symbolic name as a string. `Deserialize` validates
/// against the registry and rejects unknown names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SymbolicCode(&'static str);

impl SymbolicCode {
    /// The fallback `SymbolicCode` used when an error variant has no
    /// registered diagnostic code mapping.
    ///
    /// This always maps to the `"INTERNAL_INVARIANT_VIOLATION"` entry
    /// in [`CODE_REGISTRY`] and is guaranteed to be valid.
    pub const INTERNAL_INVARIANT: Self = Self("INTERNAL_INVARIANT_VIOLATION");

    /// Creates a `SymbolicCode` from a static string.
    ///
    /// Returns `Some(code)` iff `s` is registered in [`CODE_REGISTRY`].
    /// Returns `None` for all other strings.
    ///
    /// This is the primary constructor for symbolic codes.
    #[must_use]
    pub fn from_static(s: &'static str) -> Option<Self> {
        if is_registered_symbolic(s) {
            Some(Self(s))
        } else {
            None
        }
    }

    /// Returns the symbolic string name (e.g., `"DUPLICATE_KEY"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// Returns the numeric `u16` encoding for this symbolic code.
    ///
    /// Returns `None` only when the symbolic code is not registered in
    /// [`CODE_REGISTRY`]. Every `SymbolicCode` constructed via
    /// [`from_static`](Self::from_static) or deserialization is
    /// guaranteed to be registered, so `None` indicates an internal
    /// invariant violation.
    #[must_use]
    pub fn numeric_code(self) -> Option<u16> {
        symbolic_to_numeric(self.0)
    }

    /// Returns the equivalent [`DiagnosticCode`] for backward-compatible
    /// consumers that expect a numeric code.
    ///
    /// Returns `None` when the symbolic code is not registered (internal
    /// invariant violation).
    #[must_use]
    pub fn as_diagnostic_code(self) -> Option<DiagnosticCode> {
        self.numeric_code().map(DiagnosticCode::new)
    }

    /// Returns the [`CodeCategory`] for this symbolic code.
    ///
    /// The result is determined by the high byte of the numeric encoding.
    /// Returns `None` when the symbolic code is not registered.
    #[must_use]
    pub fn category(self) -> Option<CodeCategory> {
        self.numeric_code().map(category_from_numeric)
    }
}

impl fmt::Display for SymbolicCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, formatter)
    }
}

impl FromStr for SymbolicCode {
    type Err = SymbolicCodeParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // We can't use from_static because input might not be &'static str.
        // Instead, scan the registry for a matching symbolic name.
        for entry in CODE_REGISTRY {
            if entry.symbolic == input {
                return Ok(SymbolicCode(entry.symbolic));
            }
        }
        Err(SymbolicCodeParseError {
            name: Box::<str>::from(input),
        })
    }
}

impl Serialize for SymbolicCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SymbolicCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SymbolicCodeVisitor;

        impl<'de> Visitor<'de> for SymbolicCodeVisitor {
            type Value = SymbolicCode;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a registered symbolic diagnostic code")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<SymbolicCode, E> {
                for entry in CODE_REGISTRY {
                    if entry.symbolic == value {
                        return Ok(SymbolicCode(entry.symbolic));
                    }
                }
                Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(SymbolicCodeVisitor)
    }
}

/// Failure when parsing a symbolic code from an unknown string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
#[error("unknown symbolic diagnostic code: {name}")]
pub struct SymbolicCodeParseError {
    /// The name that could not be found in the registry.
    pub name: Box<str>,
}

// ---------------------------------------------------------------------------
// DiagnosticCode — internal numeric encoding (evolved)
// ---------------------------------------------------------------------------

/// Stable diagnostic code stored as a packed `E0101`-style value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct DiagnosticCode(u16);

impl DiagnosticCode {
    /// Creates a diagnostic code from its packed numeric value.
    #[must_use]
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    /// Returns the packed numeric code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self.0
    }

    /// Returns the symbolic diagnostic code if this numeric value is
    /// registered in [`CODE_REGISTRY`].
    ///
    /// Returns `None` if the numeric code has no registered symbolic
    /// counterpart.
    ///
    /// `numeric_to_symbolic` already returns `None` for unregistered
    /// codes, so the previous `is_supported_code` pre-check has been
    /// removed to avoid a redundant second registry scan.
    #[must_use]
    pub fn symbolic_code(self) -> Option<SymbolicCode> {
        numeric_to_symbolic(self.0).map(SymbolicCode)
    }

    /// Returns the [`CodeCategory`] for this numeric code, if it falls
    /// within a recognized category range.
    #[must_use]
    pub fn category(self) -> Option<CodeCategory> {
        if !is_supported_code(self.0) {
            return None;
        }
        Some(category_from_numeric(self.0))
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "E{:04X}", self.0)
    }
}

impl FromStr for DiagnosticCode {
    type Err = DiagnosticCodeParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut chars = input.chars();
        if chars.next() != Some('E') {
            return Err(DiagnosticCodeParseError::InvalidFormat);
        }

        let first = super::helpers::parse_hex_digit(chars.next())?;
        let second = super::helpers::parse_hex_digit(chars.next())?;
        let third = super::helpers::parse_hex_digit(chars.next())?;
        let fourth = super::helpers::parse_hex_digit(chars.next())?;
        if chars.next().is_some() {
            return Err(DiagnosticCodeParseError::InvalidFormat);
        }

        let code = super::helpers::pack_digits(first, second, third, fourth)?;
        if is_supported_code(code) {
            Ok(Self::new(code))
        } else {
            Err(DiagnosticCodeParseError::UnsupportedCode)
        }
    }
}

/// Diagnostic code parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DiagnosticCodeParseError {
    /// Input was not exactly `E` followed by four hexadecimal digits.
    #[error("diagnostic code must use format E0101")]
    InvalidFormat,
    /// Input was syntactically valid but not in a supported code range.
    #[error("diagnostic code is outside the supported ranges")]
    UnsupportedCode,
}

// ---------------------------------------------------------------------------
// Diagnostic severity
// ---------------------------------------------------------------------------

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Severity {
    /// Blocking error.
    Error,
    /// Non-blocking warning.
    Warning,
    /// Informational message.
    Info,
}

// ---------------------------------------------------------------------------
// Diagnostic — user-facing record (evolved)
// ---------------------------------------------------------------------------

/// User-facing diagnostic with stable symbolic code and source span.
///
/// The primary code field is [`Diagnostic::code`] ([`SymbolicCode`]).
/// For backward-compatible consumers, [`Diagnostic::numeric_code`]
/// provides the packed numeric encoding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Symbolic diagnostic code (primary identifier).
    pub code: SymbolicCode,
    /// Derived numeric code for backward-compatible consumers.
    /// Invariant: `numeric_code.symbolic_code() == Some(code)`.
    pub numeric_code: DiagnosticCode,
    /// Owned human-readable message.
    pub message: Box<str>,
    /// Diagnostic severity.
    pub severity: Severity,
    /// Source span for the diagnostic.
    pub span: Span,
    /// Path to source file (present for authoring-time diagnostics, absent at runtime).
    pub source_file: Option<Box<str>>,
}

impl Diagnostic {
    /// Creates a [`Diagnostic`] record from a [`SymbolicCode`].
    ///
    /// The `numeric_code` field is derived from `code` via
    /// [`SymbolicCode::as_diagnostic_code`]. Falls back to
    /// `DiagnosticCode::new(0x1309)` when the symbolic code is not
    /// registered (internal invariant violation).
    #[must_use]
    pub fn new(
        code: SymbolicCode,
        message: Box<str>,
        severity: Severity,
        span: Span,
        source_file: Option<Box<str>>,
    ) -> Self {
        // SAFETY: Every SymbolicCode constructed via from_static() or deserialization
        // is guaranteed to be registered, so as_diagnostic_code() always returns Some.
        // The None branch is only reachable through crate-internal raw construction.
        let numeric_code = match code.as_diagnostic_code() {
            Some(nc) => nc,
            // Internal invariant fallback: 0x1309 = INTERNAL_INVARIANT_VIOLATION
            None => DiagnosticCode::new(0x1309),
        };
        Self {
            code,
            numeric_code,
            message,
            severity,
            source_file,
            span,
        }
    }

    /// Creates a [`Diagnostic`] from a [`DiagnosticCode`] by looking up
    /// its symbolic counterpart in the registry.
    ///
    /// Returns `None` if the numeric code has no registered symbolic entry.
    /// This is the backward-compatible constructor for consumers that
    /// currently use numeric codes.
    #[must_use]
    pub fn from_numeric(
        code: DiagnosticCode,
        message: Box<str>,
        severity: Severity,
        span: Span,
        source_file: Option<Box<str>>,
    ) -> Option<Self> {
        let symbolic = code.symbolic_code()?;
        Some(Self {
            code: symbolic,
            numeric_code: code,
            message,
            severity,
            source_file,
            span,
        })
    }
}

// ---------------------------------------------------------------------------
// HasSymbolicCode trait
// ---------------------------------------------------------------------------

/// Trait for error types that carry a symbolic diagnostic code.
///
/// Implementors include `ValidationError`, `CompileError`, `YamlError`,
/// `CoreError`, `RuntimeError`, and `JournalError`.
///
/// All implementations must be pure functions: no I/O, no allocation,
/// no side effects.
pub trait HasSymbolicCode {
    /// Returns the symbolic diagnostic code for this error.
    #[must_use]
    fn symbolic_code(&self) -> SymbolicCode;
}
