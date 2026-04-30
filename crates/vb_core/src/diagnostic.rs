#![forbid(unsafe_code)]

//! Stable diagnostic identifiers and rendered diagnostic records.

use crate::span::Span;
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable diagnostic code stored as a packed `E0101`-style value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

        let first = parse_digit(chars.next())?;
        let second = parse_digit(chars.next())?;
        let third = parse_digit(chars.next())?;
        let fourth = parse_digit(chars.next())?;
        if chars.next().is_some() {
            return Err(DiagnosticCodeParseError::InvalidFormat);
        }

        let code = pack_digits(first, second, third, fourth)?;
        if is_supported_code(code) {
            Ok(Self::new(code))
        } else {
            Err(DiagnosticCodeParseError::UnsupportedCode)
        }
    }
}

/// Diagnostic code parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DiagnosticCodeParseError {
    /// Input was not exactly `E` followed by four decimal digits.
    #[error("diagnostic code must use format E0101")]
    InvalidFormat,
    /// Input was syntactically valid but not in a supported code range.
    #[error("diagnostic code is outside the supported ranges")]
    UnsupportedCode,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Blocking error.
    Error,
    /// Non-blocking warning.
    Warning,
    /// Informational message.
    Info,
}

/// User-facing diagnostic with stable code and source span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: DiagnosticCode,
    /// Owned human-readable message.
    pub message: Box<str>,
    /// Diagnostic severity.
    pub severity: Severity,
    /// Source span for the diagnostic.
    pub span: Span,
}

impl Diagnostic {
    /// Creates a diagnostic record.
    #[must_use]
    pub const fn new(
        code: DiagnosticCode,
        message: Box<str>,
        severity: Severity,
        span: Span,
    ) -> Self {
        Self {
            code,
            message,
            severity,
            span,
        }
    }
}

fn parse_digit(value: Option<char>) -> Result<u16, DiagnosticCodeParseError> {
    let Some(character) = value else {
        return Err(DiagnosticCodeParseError::InvalidFormat);
    };
    let Some(digit) = character.to_digit(10) else {
        return Err(DiagnosticCodeParseError::InvalidFormat);
    };
    u16::try_from(digit).map_err(|_| DiagnosticCodeParseError::InvalidFormat)
}

fn pack_digits(
    first: u16,
    second: u16,
    third: u16,
    fourth: u16,
) -> Result<u16, DiagnosticCodeParseError> {
    let first = first
        .checked_shl(12)
        .ok_or(DiagnosticCodeParseError::InvalidFormat)?;
    let second = second
        .checked_shl(8)
        .ok_or(DiagnosticCodeParseError::InvalidFormat)?;
    let third = third
        .checked_shl(4)
        .ok_or(DiagnosticCodeParseError::InvalidFormat)?;
    first
        .checked_add(second)
        .and_then(|prefix| prefix.checked_add(third))
        .and_then(|prefix| prefix.checked_add(fourth))
        .ok_or(DiagnosticCodeParseError::InvalidFormat)
}

const fn is_supported_code(code: u16) -> bool {
    matches!(
        code,
        0x0101..=0x0109
            | 0x0111..=0x0119
            | 0x0201..=0x0209
            | 0x0301..=0x0309
            | 0x0401..=0x0409
    )
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticCode, DiagnosticCodeParseError, Severity};
    use crate::span::Span;
    use core::str::FromStr;

    #[test]
    fn diagnostic_code_preserves_packed_value() {
        let code = DiagnosticCode::new(0x0101);

        assert_eq!(code.code(), 0x0101);
        assert_eq!(code.to_string(), "E0101");
    }

    #[test]
    fn diagnostic_code_parses_supported_ranges() {
        assert_eq!(
            DiagnosticCode::from_str("E0101"),
            Ok(DiagnosticCode::new(0x0101))
        );
        assert_eq!(
            DiagnosticCode::from_str("E0119"),
            Ok(DiagnosticCode::new(0x0119))
        );
        assert_eq!(
            DiagnosticCode::from_str("E0409"),
            Ok(DiagnosticCode::new(0x0409))
        );
    }

    #[test]
    fn diagnostic_code_rejects_malformed_or_unsupported_input() {
        assert_eq!(
            DiagnosticCode::from_str("0101"),
            Err(DiagnosticCodeParseError::InvalidFormat)
        );
        assert_eq!(
            DiagnosticCode::from_str("E010A"),
            Err(DiagnosticCodeParseError::InvalidFormat)
        );
        assert_eq!(
            DiagnosticCode::from_str("E0410"),
            Err(DiagnosticCodeParseError::UnsupportedCode)
        );
    }

    #[test]
    fn diagnostic_record_owns_message_and_span() {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(0x0101),
            Box::<str>::from("invalid workflow"),
            Severity::Error,
            Span::ZERO,
        );

        assert_eq!(diagnostic.code, DiagnosticCode::new(0x0101));
        assert_eq!(diagnostic.message.as_ref(), "invalid workflow");
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.span, Span::ZERO);
    }
}
