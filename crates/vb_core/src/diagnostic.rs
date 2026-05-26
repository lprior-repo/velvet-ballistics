#![forbid(unsafe_code)]

//! Stable diagnostic identifiers and rendered diagnostic records.
//!
//! Contains the symbolic diagnostic code system (SymbolicCode, CodeRegistry),
//! the numeric diagnostic code internals (DiagnosticCode), and the user-facing
//! Diagnostic record. The registry is the single source of truth for all
//! diagnostic codes used anywhere in the workspace.

use crate::span::Span;
use core::fmt;
use core::str::FromStr;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

// ---------------------------------------------------------------------------
// DiagnosticCode — numeric diagnostic code (existing stable API)
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

    /// Returns the symbolic code for this numeric code, if it is registered.
    #[must_use]
    pub fn symbolic_code(self) -> Option<SymbolicCode> {
        numeric_to_symbolic(self.0)
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

        let first = parse_hex_digit(chars.next())?;
        let second = parse_hex_digit(chars.next())?;
        let third = parse_hex_digit(chars.next())?;
        let fourth = parse_hex_digit(chars.next())?;
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

/// User-facing diagnostic with stable code, source span, and optional file path.
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
    /// Source span for the diagnostic (carries optional line/column).
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
        code: DiagnosticCode,
        message: Box<str>,
        severity: Severity,
        span: Span,
        source_file: Option<Box<str>>,
    ) -> Self {
        Self {
            code,
            numeric_code,
            message,
            severity,
            span,
            source_file,
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
    ) -> Option<Self> {
        let symbolic = code.symbolic_code()?;
        Some(Self {
            code: symbolic,
            numeric_code: code,
            message,
            severity,
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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Classifies a numeric code into its [`CodeCategory`] by consulting
/// the [`CODE_REGISTRY`] first, falling back to high-byte heuristics
/// when the numeric code is not yet registered.
///
/// This ensures that registry entries with explicit categories (such as
/// `CodeCategory::Internal` for `INTERNAL_INVARIANT_VIOLATION` at
/// 0x1309) are correctly classified instead of being misclassified by
/// the high-byte alone.
#[must_use]
pub fn category_from_numeric(numeric: u16) -> CodeCategory {
    // 1. Consult registry for the authoritative category.
    for entry in CODE_REGISTRY {
        if entry.numeric == numeric {
            return entry.category;
        }
    }
    // 2. Fall back to high-byte heuristics for unregistered codes.
    let high_byte = numeric.wrapping_shr(8) & 0xFF_u16;
    match high_byte {
        0x01 => CodeCategory::Schema,
        0x02 => CodeCategory::Reference,
        0x03 => CodeCategory::ControlFlow,
        0x04 => CodeCategory::TypeTaint,
        0x05 => CodeCategory::Gate,
        0x06 => CodeCategory::ContractDiscovery,
        0x10 => CodeCategory::Compilation,
        0x11 => CodeCategory::WorkflowIr,
        0x12 => CodeCategory::Expression,
        0x13 => CodeCategory::Accessor,
        0x14 => CodeCategory::Lowering,
        0x15 => CodeCategory::Lifecycle,
        0x20 => CodeCategory::Storage,
        0x30 => CodeCategory::Runtime,
        0x32 => CodeCategory::Ipc,
        0x33 => CodeCategory::Lifecycle,
        0x40 => CodeCategory::RuntimeBoundary,
        _ => CodeCategory::Internal, // unregistered high bytes → Internal
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_hex_digit(value: Option<char>) -> Result<u16, DiagnosticCodeParseError> {
    let Some(character) = value else {
        return Err(DiagnosticCodeParseError::InvalidFormat);
    };
    let Some(digit) = character.to_digit(16) else {
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
    let first_shifted = first
        .checked_shl(12)
        .ok_or(DiagnosticCodeParseError::InvalidFormat)?;
    let second_shifted = second
        .checked_shl(8)
        .ok_or(DiagnosticCodeParseError::InvalidFormat)?;
    let third_shifted = third
        .checked_shl(4)
        .ok_or(DiagnosticCodeParseError::InvalidFormat)?;
    first_shifted
        .checked_add(second_shifted)
        .and_then(|prefix| prefix.checked_add(third_shifted))
        .and_then(|prefix| prefix.checked_add(fourth))
        .ok_or(DiagnosticCodeParseError::InvalidFormat)
}

const fn is_supported_code(code: u16) -> bool {
    matches!(
        code,
        0x0101..=0x010B
            | 0x0201..=0x0204
            | 0x0301..=0x0309
            | 0x0401..=0x040C
            |         0x0501..=0x0513
            | 0x0601..=0x0603
            | 0x1001..=0x1003
            | 0x1011..=0x1014
            | 0x1101..=0x1104
            | 0x1201..=0x1202
            | 0x1301..=0x130D
            | 0x1311..=0x1314
            | 0x1401..=0x140D
            | 0x1501..=0x1506
            | 0x2001..=0x201E
            | 0x3001..=0x300E
            | 0x4001..=0x4021
    )
}

// ---------------------------------------------------------------------------
// Code Registry — canonical source of truth for all diagnostic codes
// ---------------------------------------------------------------------------

/// Code category for classifying diagnostic codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodeCategory {
    /// Schema validation: E01xx.
    Schema,
    /// Reference validation: E02xx.
    Reference,
    /// Control-flow validation: E03xx.
    ControlFlow,
    /// Type/taint validation: E04xx.
    TypeTaint,
    /// Gate verifier: E05xx.
    Gate,
    /// Contract discovery: E06xx.
    ContractDiscovery,
    /// Compilation internal: E10xx.
    Compilation,
    /// Workflow IR: E11xx.
    WorkflowIr,
    /// Expression errors: E12xx.
    Expression,
    /// Accessor/path errors: E13xx.
    Accessor,
    /// Lowering errors: E14xx.
    Lowering,
    /// Lifecycle errors: E15xx.
    Lifecycle,
    /// Storage errors: E20xx.
    Storage,
    /// Runtime core errors: E30xx.
    Runtime,
    /// Runtime boundary errors: E40xx.
    RuntimeBoundary,
}

/// A single entry in the canonical diagnostic code registry.
pub struct CodeEntry {
    /// Symbolic name (e.g. `"DUPLICATE_KEY"`).
    pub symbolic: &'static str,
    /// Packed numeric code (e.g. `0x0101`).
    pub numeric: u16,
    /// Category this code belongs to.
    pub category: CodeCategory,
}

/// Canonical code registry — all Section 16 diagnostic codes plus extensions.
///
/// This is the single source of truth mapping symbolic names to numeric codes.
/// Every diagnostic code used in the system must be registered here.
pub const CODE_REGISTRY: &[CodeEntry] = &[
    // Schema: E01xx (11 codes)
    CodeEntry {
        symbolic: "DUPLICATE_KEY",
        numeric: 0x0101,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "FORBIDDEN_YAML_FEATURE",
        numeric: 0x0102,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "UNKNOWN_TOP_LEVEL_FIELD",
        numeric: 0x0103,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "UNKNOWN_STEP_FIELD",
        numeric: 0x0104,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "MISSING_REQUIRED_FIELD",
        numeric: 0x0105,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "INVALID_VERSION",
        numeric: 0x0106,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "INVALID_ID",
        numeric: 0x0107,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "RESERVED_ID",
        numeric: 0x0108,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "DUPLICATE_ID",
        numeric: 0x0109,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "MULTIPLE_STEP_PRIMITIVES",
        numeric: 0x010A,
        category: CodeCategory::Schema,
    },
    CodeEntry {
        symbolic: "MISSING_STEP_PRIMITIVE",
        numeric: 0x010B,
        category: CodeCategory::Schema,
    },
    // Reference: E02xx (4 codes)
    CodeEntry {
        symbolic: "UNKNOWN_REFERENCE",
        numeric: 0x0201,
        category: CodeCategory::Reference,
    },
    CodeEntry {
        symbolic: "FUTURE_REFERENCE",
        numeric: 0x0202,
        category: CodeCategory::Reference,
    },
    CodeEntry {
        symbolic: "SECRET_NOT_DECLARED",
        numeric: 0x0203,
        category: CodeCategory::Reference,
    },
    CodeEntry {
        symbolic: "DIRECT_RUNTIME_REFERENCE",
        numeric: 0x0204,
        category: CodeCategory::Reference,
    },
    // Control Flow: E03xx (9 codes)
    CodeEntry {
        symbolic: "INVALID_THEN_TARGET",
        numeric: 0x0301,
        category: CodeCategory::ControlFlow,
    },
    CodeEntry {
        symbolic: "CONTROL_FLOW_CYCLE",
        numeric: 0x0302,
        category: CodeCategory::ControlFlow,
    },
    CodeEntry {
        symbolic: "UNREACHABLE_STEP",
        numeric: 0x0303,
        category: CodeCategory::ControlFlow,
    },
    CodeEntry {
        symbolic: "INVALID_CHOOSE",
        numeric: 0x0304,
        category: CodeCategory::ControlFlow,
    },
    CodeEntry {
        symbolic: "INVALID_FOR_EACH",
        numeric: 0x0305,
        category: CodeCategory::ControlFlow,
    },
    CodeEntry {
        symbolic: "INVALID_TOGETHER",
        numeric: 0x0306,
        category: CodeCategory::ControlFlow,
    },
    CodeEntry {
        symbolic: "INVALID_COLLECT",
        numeric: 0x0307,
        category: CodeCategory::ControlFlow,
    },
    CodeEntry {
        symbolic: "INVALID_REDUCE",
        numeric: 0x0308,
        category: CodeCategory::ControlFlow,
    },
    CodeEntry {
        symbolic: "INVALID_REPEAT",
        numeric: 0x0309,
        category: CodeCategory::ControlFlow,
    },
    // Type/Taint: E04xx (12 codes)
    CodeEntry {
        symbolic: "INVALID_WAIT",
        numeric: 0x0401,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "INVALID_ASK",
        numeric: 0x0402,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "INVALID_FINISH",
        numeric: 0x0403,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "INVALID_RETRY",
        numeric: 0x0404,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "INVALID_ON_ERROR",
        numeric: 0x0405,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "SECRET_RESULT_LEAK",
        numeric: 0x0406,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "TYPE_MISMATCH",
        numeric: 0x0407,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "PAYLOAD_TOO_LARGE",
        numeric: 0x0408,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "LIMIT_REQUIRED",
        numeric: 0x0409,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "LIMIT_EXCEEDED",
        numeric: 0x040A,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_TRIGGER",
        numeric: 0x040B,
        category: CodeCategory::TypeTaint,
    },
    CodeEntry {
        symbolic: "HTTP_TRIGGER_OUT_OF_CORE",
        numeric: 0x040C,
        category: CodeCategory::TypeTaint,
    },
    // Gate Verifier: E05xx (19 codes)
    CodeEntry {
        symbolic: "EXPRESSION_STACK_EXCEEDED",
        numeric: 0x0501,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "EXPRESSION_STACK_MISMATCH",
        numeric: 0x0502,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "ACCESSOR_SLOT_OUT_OF_RANGE",
        numeric: 0x0503,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "ACCESSOR_PATH_INVALID",
        numeric: 0x0504,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "SLOT_REFERENCE_OUT_OF_RANGE",
        numeric: 0x0505,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "LOOP_BODY_STEP_OUT_OF_RANGE",
        numeric: 0x0506,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "SLOT_DEPENDENCY_CYCLE",
        numeric: 0x0507,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "NODE_KIND_CONSTRAINT_VIOLATION",
        numeric: 0x0508,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "ACTION_CONTRACT_MISSING",
        numeric: 0x0509,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "ACTION_CONTRACT_ORPHAN",
        numeric: 0x050A,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "SLOT_TYPE_INCONSISTENCY",
        numeric: 0x050B,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "NON_DETERMINISTIC_PATH",
        numeric: 0x050C,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "CAPABILITY_NAME_EMPTY",
        numeric: 0x050D,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "CAPABILITY_NAME_TOO_LONG",
        numeric: 0x050E,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "CAPABILITY_NAME_INVALID",
        numeric: 0x050F,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "CAPABILITY_ACTION_MISMATCH",
        numeric: 0x0510,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "CAPABILITY_DUPLICATE",
        numeric: 0x0511,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "ACCESSOR_PATH_TOO_DEEP",
        numeric: 0x0512,
        category: CodeCategory::Gate,
    },
    CodeEntry {
        symbolic: "ACCESSOR_SYMBOL_OUT_OF_BOUNDS",
        numeric: 0x0513,
        category: CodeCategory::Gate,
    },
    // Contract Discovery: E06xx (3 codes)
    CodeEntry {
        symbolic: "MISSING_SCHEMA_VERSION",
        numeric: 0x0601,
        category: CodeCategory::ContractDiscovery,
    },
    CodeEntry {
        symbolic: "CUE_VET_FAILED",
        numeric: 0x0602,
        category: CodeCategory::ContractDiscovery,
    },
    CodeEntry {
        symbolic: "VERSION_MONOTONICITY_BREACH",
        numeric: 0x0603,
        category: CodeCategory::ContractDiscovery,
    },
    // Note: Section 16 compilation entries (0x10xx-0x14xx) from the Kani model
    // conflict with production CoreError numeric codes. CoreError numeric
    // assignments take priority. The symbolic strings used by CompileError
    // (e.g. "INVALID_EXPRESSION", "CANONICAL_YAML_PARSE", etc.) are registered
    // below with unique codes that don't conflict with CoreError assignments.
    //
    // Compilation symbolic codes (non-CoreError):
    CodeEntry {
        symbolic: "CANONICAL_YAML_PARSE",
        numeric: 0x1030,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "TOP_LEVEL_NOT_MAPPING",
        numeric: 0x1031,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "NON_STRING_KEY",
        numeric: 0x1032,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "UNKNOWN_INPUT_SCHEMA_FIELD",
        numeric: 0x1033,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_TOP_LEVEL_DECLARATION",
        numeric: 0x1034,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "UNKNOWN_OUTPUT_NAME",
        numeric: 0x1035,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "INVALID_COMPILED_WORKFLOW",
        numeric: 0x1307,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_ACCESSOR_REFERENCE",
        numeric: 0x133C,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "INVALID_EXPRESSION",
        numeric: 0x133D,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "IDEMPOTENCY_VIOLATION",
        numeric: 0x133E,
        category: CodeCategory::Accessor,
    },
    // Workflow IR:
    CodeEntry {
        symbolic: "SLOT_COUNT_MISMATCH",
        numeric: 0x1130,
        category: CodeCategory::WorkflowIr,
    },
    CodeEntry {
        symbolic: "SYMBOL_COUNT_MISMATCH",
        numeric: 0x1131,
        category: CodeCategory::WorkflowIr,
    },
    // Expression:
    CodeEntry {
        symbolic: "EXPRESSION_BRANCH_EMPTY",
        numeric: 0x1230,
        category: CodeCategory::Expression,
    },
    // Accessor:
    CodeEntry {
        symbolic: "ACCESSOR_CYCLE",
        numeric: 0x1330,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_FIELD_MISSING",
        numeric: 0x1331,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_FIELD_NAME_EMPTY",
        numeric: 0x1332,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_ROOT_OUT_OF_RANGE",
        numeric: 0x1333,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_SLOT_COUNT_MISMATCH",
        numeric: 0x1334,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_OUTPUT_NAME_EMPTY",
        numeric: 0x1335,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_DUPLICATE_OUTPUT",
        numeric: 0x1336,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_UNUSED_OUTPUT",
        numeric: 0x1337,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_ID_EMPTY",
        numeric: 0x1338,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_INVALID_SYMBOL",
        numeric: 0x1339,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_MISSING_ROOT",
        numeric: 0x133A,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_EMPTY_PATH",
        numeric: 0x133B,
        category: CodeCategory::Accessor,
    },
    // Lowering:
    CodeEntry {
        symbolic: "LOWERING_BODY_EMPTY",
        numeric: 0x1430,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_BODY_TOO_LARGE",
        numeric: 0x1431,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_DUPLICATE_LABEL",
        numeric: 0x1432,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_INVALID_LABEL",
        numeric: 0x1433,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_NODE_OUT_OF_RANGE",
        numeric: 0x1434,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_UNKNOWN_PRIMITIVE",
        numeric: 0x1435,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_EMPTY_STEP_LIST",
        numeric: 0x1436,
        category: CodeCategory::Lowering,
    },
    // Section 16 codes that are reused by CompileError with same symbolic name:
    // INVALID_COMPILED_WORKFLOW, CONST_OUT_OF_BOUNDS, INVALID_EXPRESSION,
    // UNSUPPORTED_ACCESSOR_REFERENCE, IDEMPOTENCY_VIOLATION,
    // ACTION_RESULT_AUDIT_MISMATCH, ACTION_TYPE_CONSTRAINT_FAIL,
    // ACTION_CIRCUIT_BREAKER_OPEN are registered in the Core Engine Internal
    // section below using their CoreError-assigned numeric codes.
    // Storage: E20xx (15 codes)
    CodeEntry {
        symbolic: "STORAGE_UNAVAILABLE",
        numeric: 0x2001,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_CORRUPTION",
        numeric: 0x2002,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_IO",
        numeric: 0x2003,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_ENCODING",
        numeric: 0x2004,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_DECODING",
        numeric: 0x2005,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_CHECKPOINT",
        numeric: 0x2006,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_SNAPSHOT",
        numeric: 0x2007,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_JOURNAL",
        numeric: 0x2008,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_PAGE_OVERFLOW",
        numeric: 0x2009,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_KEYSPACE_MANIFEST",
        numeric: 0x200A,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_BLOB_LIMIT",
        numeric: 0x200B,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_WRITE_BUDGET",
        numeric: 0x200C,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_READ_BUDGET",
        numeric: 0x200D,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_COMPACTION_FAILED",
        numeric: 0x200E,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_SEALED",
        numeric: 0x200F,
        category: CodeCategory::Storage,
    },
    // Runtime Core: E30xx (14 codes)
    CodeEntry {
        symbolic: "RUNTIME_PANIC",
        numeric: 0x3001,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_TIMEOUT",
        numeric: 0x3002,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_CAPACITY_EXCEEDED",
        numeric: 0x3003,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_BUDGET_EXHAUSTED",
        numeric: 0x3004,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_INVALID_STATE",
        numeric: 0x3005,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_CYCLE_LIMIT",
        numeric: 0x3006,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_ACTION_DISPATCH",
        numeric: 0x3007,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_ACTION_TIMEOUT",
        numeric: 0x3008,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_SIGNAL_INVALID",
        numeric: 0x3009,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_QUEUE_OVERFLOW",
        numeric: 0x300A,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_JOURNAL_BATCH",
        numeric: 0x300B,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_TICK_OVERFLOW",
        numeric: 0x300C,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_STEP_LIMIT",
        numeric: 0x300D,
        category: CodeCategory::Runtime,
    },
    CodeEntry {
        symbolic: "RUNTIME_TRACE_OVERFLOW",
        numeric: 0x300E,
        category: CodeCategory::Runtime,
    },
    // Runtime Boundary: E40xx (28 codes)
    CodeEntry {
        symbolic: "IPC_PAYLOAD_TOO_LARGE",
        numeric: 0x4001,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_DECODE_FAILED",
        numeric: 0x4002,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_ENCODE_FAILED",
        numeric: 0x4003,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_CHANNEL_CLOSED",
        numeric: 0x4004,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_CHANNEL_FULL",
        numeric: 0x4005,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_CONNECTION_REFUSED",
        numeric: 0x4006,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_TIMEOUT",
        numeric: 0x4007,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_PROTOCOL_VIOLATION",
        numeric: 0x4008,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_AUTH_FAILED",
        numeric: 0x4009,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "IPC_RESOURCE_UNAVAILABLE",
        numeric: 0x400A,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_STORAGE_UNAVAILABLE",
        numeric: 0x400B,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_DUPLICATE_REQUEST",
        numeric: 0x400C,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_INVALID_TRANSITION",
        numeric: 0x400D,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_STALE_BEAD",
        numeric: 0x400E,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_SEQ_MISMATCH",
        numeric: 0x400F,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_CHECKPOINT_MISMATCH",
        numeric: 0x4010,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_PAGE_ORDER_VIOLATION",
        numeric: 0x4011,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_EXTRA_HYDRATION_FAIL",
        numeric: 0x4012,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_EVIDENCE_OVERFLOW",
        numeric: 0x4013,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_SLOT_NOT_WRITABLE",
        numeric: 0x4014,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_DUPLICATE_ACTION",
        numeric: 0x4015,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_UNKNOWN_ACTION",
        numeric: 0x4016,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_STALE_EVENT",
        numeric: 0x4017,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_EVENT_ORDER",
        numeric: 0x4018,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_BATCH_OVERFLOW",
        numeric: 0x4019,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_CLOCK_DRIFT",
        numeric: 0x401A,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_BUFFER_OVERFLOW",
        numeric: 0x401B,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_SLOT_SEALED",
        numeric: 0x401C,
        category: CodeCategory::RuntimeBoundary,
    },
    // Core Engine Internal: 0x1001-0x1506 (47 codes)
    CodeEntry {
        symbolic: "INVALID_PROGRAM_COUNTER",
        numeric: 0x1001,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "MISSING_NEXT_STEP",
        numeric: 0x1002,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "SLOT_OUT_OF_BOUNDS",
        numeric: 0x1011,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "SLOT_UNINITIALIZED",
        numeric: 0x1012,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "CONST_OUT_OF_BOUNDS",
        numeric: 0x1013,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "EXPR_OUT_OF_BOUNDS",
        numeric: 0x1014,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "NON_FINITE_NUMBER",
        numeric: 0x1102,
        category: CodeCategory::WorkflowIr,
    },
    CodeEntry {
        symbolic: "DIVISION_BY_ZERO",
        numeric: 0x1103,
        category: CodeCategory::WorkflowIr,
    },
    CodeEntry {
        symbolic: "NON_BOOL_CONDITION",
        numeric: 0x1104,
        category: CodeCategory::WorkflowIr,
    },
    CodeEntry {
        symbolic: "STEP_BUDGET_EXHAUSTED",
        numeric: 0x1201,
        category: CodeCategory::Expression,
    },
    CodeEntry {
        symbolic: "STEP_COUNTER_OVERFLOW",
        numeric: 0x1202,
        category: CodeCategory::Expression,
    },
    CodeEntry {
        symbolic: "QUEUE_FULL",
        numeric: 0x1301,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "RESOURCE_LIMIT_EXCEEDED",
        numeric: 0x1302,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ALLOCATION_FAILED",
        numeric: 0x1303,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "EXPRESSION_STACK_OVERFLOW",
        numeric: 0x1304,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "MISSING_OUTPUT_SLOT",
        numeric: 0x1305,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "STEP_STATE_OUT_OF_BOUNDS",
        numeric: 0x1306,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_PRIMITIVE",
        numeric: 0x1308,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "INTERNAL_INVARIANT_VIOLATION",
        numeric: 0x1309,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_ACCESSOR_TRAVERSAL",
        numeric: 0x130A,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "EXPRESSION_STACK_UNDERFLOW",
        numeric: 0x130B,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "OBJECT_FIELD_NOT_FOUND",
        numeric: 0x130C,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "LIST_INDEX_OUT_OF_BOUNDS",
        numeric: 0x130D,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "SYMBOL_OUT_OF_BOUNDS",
        numeric: 0x1311,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "LIST_OUT_OF_BOUNDS",
        numeric: 0x1312,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "OBJECT_OUT_OF_BOUNDS",
        numeric: 0x1313,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "BLOB_OUT_OF_BOUNDS",
        numeric: 0x1314,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ITERATION_LIMIT_EXCEEDED",
        numeric: 0x1401,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "REPEAT_EXHAUSTED",
        numeric: 0x1402,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "COLLECT_PAGE_LIMIT_EXCEEDED",
        numeric: 0x1403,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "COLLECT_ITEM_LIMIT_EXCEEDED",
        numeric: 0x1404,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "TOGETHER_BRANCH_LIMIT_EXCEEDED",
        numeric: 0x1405,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "BUDGET_EXCEEDED",
        numeric: 0x1406,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "COLLECT_TIME_LIMIT_EXCEEDED",
        numeric: 0x1407,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "PARALLEL_LIMIT_EXCEEDED",
        numeric: 0x1408,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "CAPABILITY_DENIED",
        numeric: 0x1409,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "BUDGET_PARSE",
        numeric: 0x140A,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "COLLECT_PAGE_ORDER_VIOLATION",
        numeric: 0x140B,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "COLLECT_EXTRA_HYDRATION_FAILED",
        numeric: 0x140C,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "COLLECT_EVIDENCE_CAPACITY_EXCEEDED",
        numeric: 0x140D,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_STORAGE_UNAVAILABLE",
        numeric: 0x1501,
        category: CodeCategory::Lifecycle,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_DUPLICATE_REQUEST",
        numeric: 0x1502,
        category: CodeCategory::Lifecycle,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_STALE_REQUEST",
        numeric: 0x1503,
        category: CodeCategory::Lifecycle,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_INVALID_TRANSITION",
        numeric: 0x1504,
        category: CodeCategory::Lifecycle,
    },
    CodeEntry {
        symbolic: "JOURNAL_WRITE_FAILURE",
        numeric: 0x1505,
        category: CodeCategory::Lifecycle,
    },
    CodeEntry {
        symbolic: "REPLAY_CORRUPTION",
        numeric: 0x1506,
        category: CodeCategory::Lifecycle,
    },
    // Runtime Error: 0x2001-0x201E (from RuntimeError codes)
    CodeEntry {
        symbolic: "QUEUE_FULL",
        numeric: 0x2001,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "RUN_NOT_FOUND",
        numeric: 0x2002,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ACTIVE_RUN_CAPACITY_EXCEEDED",
        numeric: 0x2003,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "RUN_ALREADY_EXISTS",
        numeric: 0x2004,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_OPERATION",
        numeric: 0x2005,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "SHUTDOWN_IN_PROGRESS",
        numeric: 0x2006,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "JOURNAL_POISONED",
        numeric: 0x2007,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "STORAGE_JOURNAL_APPEND",
        numeric: 0x2008,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_ASYNC_STRICT_ACK",
        numeric: 0x2009,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "FRAME_POOL_UNAVAILABLE",
        numeric: 0x200A,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "INVALID_ACTION_COMPLETION",
        numeric: 0x200B,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "INVALID_TIMER_FIRE",
        numeric: 0x200C,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_FULL_RECOVERY_HYDRATION",
        numeric: 0x200D,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "INVALID_RECOVERY_HYDRATION",
        numeric: 0x200E,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "COMMAND_QUEUE_CAPACITY_EXCEEDED",
        numeric: 0x200F,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ACTIVE_RUN_CAPACITY_ZERO",
        numeric: 0x2010,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ADMISSION_ARTIFACT_NOT_FOUND",
        numeric: 0x2011,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ADMISSION_CAPABILITY_DENIED",
        numeric: 0x2012,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ENCODE_FAILED",
        numeric: 0x2013,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ADMISSION_ARTIFACT_INVALID",
        numeric: 0x2014,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ADMISSION_HEADER_PERSISTENCE_FAILED",
        numeric: 0x2015,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "SECRET_RESULT_NOT_ALLOWED",
        numeric: 0x2016,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "IPC_PAYLOAD_SIZE_EXCEEDED",
        numeric: 0x2017,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ADMISSION_ARTIFACT_DIGEST_MISMATCH",
        numeric: 0x2018,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ADMISSION_ARTIFACT_STALE",
        numeric: 0x2019,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ADMISSION_DIGEST_MISMATCH",
        numeric: 0x201A,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "ENGINE_DRIVE_FAILED",
        numeric: 0x201B,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "SHARD_NOT_FOUND",
        numeric: 0x201C,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "MIGRATE_SELF",
        numeric: 0x201D,
        category: CodeCategory::Storage,
    },
    CodeEntry {
        symbolic: "JOURNAL_FULL",
        numeric: 0x201E,
        category: CodeCategory::Storage,
    },
    // Journal Storage: 0x4001-0x4021 (from JournalError codes)
    CodeEntry {
        symbolic: "FJALL_ERROR",
        numeric: 0x4001,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_ENCODE_FAILED",
        numeric: 0x4002,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "KEY_CAPACITY_EXCEEDED",
        numeric: 0x4003,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "DUPLICATE_EVENT",
        numeric: 0x4004,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "WRITE_LOCK_POISONED",
        numeric: 0x4005,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "QUEUE_CAPACITY_ZERO",
        numeric: 0x4006,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "JOURNAL_QUEUE_FULL",
        numeric: 0x4007,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "WRONG_RUN",
        numeric: 0x4008,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "SEQUENCE_GAP",
        numeric: 0x4009,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "SEQUENCE_OVERFLOW",
        numeric: 0x400A,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "BAD_MAGIC",
        numeric: 0x400B,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_SCHEMA_VERSION",
        numeric: 0x400C,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "MIGRATION_REQUIRED",
        numeric: 0x400D,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "UNKNOWN_RECORD_KIND",
        numeric: 0x400E,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "RECORD_KIND_FAMILY_MISMATCH",
        numeric: 0x400F,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "HEADER_LENGTH_MISMATCH",
        numeric: 0x4010,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "HEADER_CHECKSUM_MISMATCH",
        numeric: 0x4012,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "PAYLOAD_DIGEST_MISMATCH",
        numeric: 0x4013,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "UNEXPECTED_EOF",
        numeric: 0x4014,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "POSTCARD_DECODE_FAILED",
        numeric: 0x4015,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "QUEUE_SHUTDOWN",
        numeric: 0x4016,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "ARTIFACT_MALFORMED",
        numeric: 0x4017,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "ARTIFACT_CHECKSUM_MISMATCH",
        numeric: 0x4018,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "ARTIFACT_NOT_FOUND",
        numeric: 0x4019,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "PROCESS_LOCK_HELD",
        numeric: 0x401A,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "PROCESS_LOCK_IO",
        numeric: 0x401B,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "INVALID_GATE_COUNT",
        numeric: 0x401C,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "MISSING_REQUIRED_PROOF_FLAG",
        numeric: 0x401D,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "TOO_MANY_EVENTS",
        numeric: 0x401E,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "REPLAY_ALLOCATION_FAILED",
        numeric: 0x401F,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "INVALID_JOURNAL_EVENT",
        numeric: 0x4020,
        category: CodeCategory::RuntimeBoundary,
    },
    CodeEntry {
        symbolic: "INVALID_RUN_ID",
        numeric: 0x4021,
        category: CodeCategory::RuntimeBoundary,
    },
];

// ---------------------------------------------------------------------------
// Registry lookup helpers
// ---------------------------------------------------------------------------

/// Returns the numeric code for a registered symbolic name, or `None`.
#[must_use]
pub fn symbolic_to_numeric(symbolic: &str) -> Option<u16> {
    CODE_REGISTRY
        .iter()
        .find(|entry| entry.symbolic == symbolic)
        .map(|entry| entry.numeric)
}

/// Returns the static symbolic string for a registered name, or `None`.
///
/// Unlike `symbolic_to_numeric`, this returns the registry's `&'static str`
/// which can be stored in a `SymbolicCode`.
#[must_use]
pub fn lookup_symbolic_static(symbolic: &str) -> Option<&'static str> {
    CODE_REGISTRY
        .iter()
        .find(|entry| entry.symbolic == symbolic)
        .map(|entry| entry.symbolic)
}

/// Returns the `SymbolicCode` for a registered numeric code, or `None`.
#[must_use]
pub fn numeric_to_symbolic(numeric: u16) -> Option<SymbolicCode> {
    CODE_REGISTRY
        .iter()
        .find(|entry| entry.numeric == numeric)
        .map(|entry| SymbolicCode {
            symbolic: entry.symbolic,
            numeric: entry.numeric,
        })
}

// ---------------------------------------------------------------------------
// SymbolicCode — symbolic diagnostic code (new stable API)
// ---------------------------------------------------------------------------

/// Stable symbolic diagnostic code.
///
/// Every `SymbolicCode` is a `Copy`, zero-allocation value that corresponds
/// to exactly one registered diagnostic code. Construction is gated by the
/// canonical [`CODE_REGISTRY`].
///
/// The struct carries both the symbolic name and the pre-resolved numeric
/// code so that `numeric_code()` is a simple field access with no possibility
/// of panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolicCode {
    symbolic: &'static str,
    numeric: u16,
}

impl SymbolicCode {
    /// Smart constructor. Returns `Some(code)` iff `s` is a registered
    /// symbolic code string in [`CODE_REGISTRY`].
    #[must_use]
    pub fn from_static(s: &'static str) -> Option<Self> {
        let numeric = symbolic_to_numeric(s)?;
        Some(SymbolicCode {
            symbolic: s,
            numeric,
        })
    }

    /// Construct directly from pre-validated parts without registry lookup.
    ///
    /// The caller MUST ensure both fields are correct and correspond to a
    /// registered entry in [`CODE_REGISTRY`]. This is used only when the
    /// invariant is structurally guaranteed (e.g., unreachable fallback
    /// paths where the sentinel's numeric code is known by construction).
    ///
    /// This is not marked `unsafe` because constructing an inconsistent
    /// `SymbolicCode` only affects display output, not memory safety. Use
    /// [`from_static`](Self::from_static) for general construction.
    #[must_use]
    pub const fn from_parts(symbolic: &'static str, numeric: u16) -> Self {
        SymbolicCode { symbolic, numeric }
    }

    /// Symbolic code used when an internal invariant violation occurs at a
    /// `SymbolicCode` construction site where the registry lookup fails.
    ///
    /// This sentinel is registered in [`CODE_REGISTRY`] at numeric `0x1309`.
    /// All callers that construct `SymbolicCode` values should ensure their
    /// symbolic strings are registered; this constant exists as a centralized
    /// type-level guarantee instead of scattered `from_parts` calls.
    pub const INTERNAL_INVARIANT: SymbolicCode = SymbolicCode {
        symbolic: "INTERNAL_INVARIANT_VIOLATION",
        numeric: 0x1309,
    };

    /// Returns the symbolic name string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.symbolic
    }

    /// Returns the packed numeric code for this symbolic code.
    ///
    /// A `SymbolicCode` can only be constructed from a registered string,
    /// so the numeric code is always valid by construction.
    #[must_use]
    pub const fn numeric_code(self) -> u16 {
        self.numeric
    }

    /// Returns the numeric `DiagnosticCode` for this symbolic code.
    #[must_use]
    pub fn as_diagnostic_code(self) -> DiagnosticCode {
        DiagnosticCode::new(self.numeric_code())
    }
}

impl fmt::Display for SymbolicCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.symbolic)
    }
}

/// Parse error for `SymbolicCode` deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SymbolicCodeParseError {
    /// The symbolic code string is not registered in [`CODE_REGISTRY`].
    #[error("unknown symbolic diagnostic code")]
    UnknownCode,
}

impl FromStr for SymbolicCode {
    type Err = SymbolicCodeParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let static_s = lookup_symbolic_static(input).ok_or(SymbolicCodeParseError::UnknownCode)?;
        let numeric = symbolic_to_numeric(static_s).ok_or(SymbolicCodeParseError::UnknownCode)?;
        Ok(SymbolicCode {
            symbolic: static_s,
            numeric,
        })
    }
}

// Custom serde: serialize as the symbolic string; deserialize only registered names.
impl Serialize for SymbolicCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.symbolic)
    }
}

impl<'de> Deserialize<'de> for SymbolicCode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s: &str = Deserialize::deserialize(deserializer)?;
        let static_s = lookup_symbolic_static(s).ok_or_else(|| {
            serde::de::Error::custom(format_args!("unknown symbolic diagnostic code: {s}"))
        })?;
        let numeric = symbolic_to_numeric(static_s).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "symbolic code '{s}' has no numeric mapping in registry"
            ))
        })?;
        Ok(SymbolicCode {
            symbolic: static_s,
            numeric,
        })
    }
}

// ---------------------------------------------------------------------------
// HasSymbolicCode trait
// ---------------------------------------------------------------------------

/// Trait for error types that can produce a stable symbolic diagnostic code.
pub trait HasSymbolicCode {
    /// Returns the stable symbolic diagnostic code for this error.
    #[must_use]
    fn symbolic_code(&self) -> SymbolicCode;
}

// ---------------------------------------------------------------------------
// Compile-time assertions for SymbolicCode
// ---------------------------------------------------------------------------

const _: () = {
    // Ensure SymbolicCode is both Send and Sync.
    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}
    assert_send::<SymbolicCode>();
    assert_sync::<SymbolicCode>();
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{
        CODE_REGISTRY, CodeCategory, Diagnostic, DiagnosticCode, DiagnosticCodeParseError,
        Severity, SymbolicCode, SymbolicCodeParseError, numeric_to_symbolic, symbolic_to_numeric,
    };
    use crate::span::Span;
    use core::str::FromStr;

    // -- DiagnosticCode existing tests (preserved) --

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
            DiagnosticCode::from_str("E010B"),
            Ok(DiagnosticCode::new(0x010B))
        );
        assert_eq!(
            DiagnosticCode::from_str("E0409"),
            Ok(DiagnosticCode::new(0x0409))
        );
        assert_eq!(
            DiagnosticCode::from_str("E040C"),
            Ok(DiagnosticCode::new(0x040C))
        );
        assert_eq!(
            DiagnosticCode::from_str("E1315"),
            Ok(DiagnosticCode::new(0x1315))
        );
        assert_eq!(
            DiagnosticCode::from_str("E4015"),
            Ok(DiagnosticCode::new(0x4015))
        );
        // New: E3020 action/audit codes (REPAIR-7 range fix)
        assert_eq!(
            DiagnosticCode::from_str("E3020"),
            Ok(DiagnosticCode::new(0x3020))
        );
        // New: E05xx gate verifier codes
        assert_eq!(
            DiagnosticCode::from_str("E0501"),
            Ok(DiagnosticCode::new(0x0501))
        );
        // New: E06xx contract discovery codes
        assert_eq!(
            DiagnosticCode::from_str("E0601"),
            Ok(DiagnosticCode::new(0x0601))
        );
        // New: E4020 boundary
        assert_eq!(
            DiagnosticCode::from_str("E4020"),
            Ok(DiagnosticCode::new(0x4020))
        );
    }

    #[test]
    fn diagnostic_code_rejects_malformed_or_unsupported_input() {
        assert_eq!(
            DiagnosticCode::from_str("0101"),
            Err(DiagnosticCodeParseError::InvalidFormat)
        );
        assert_eq!(
            DiagnosticCode::from_str("E010C"),
            Err(DiagnosticCodeParseError::UnsupportedCode)
        );
        assert_eq!(
            DiagnosticCode::from_str("E0410"),
            Err(DiagnosticCodeParseError::UnsupportedCode)
        );
    }

    // ---- SymbolicCode tests ----

    #[test]
    fn symbolic_code_from_static_known_code() {
        let code = SymbolicCode::from_static("DUPLICATE_KEY");
        assert!(code.is_some());
        assert_eq!(code.expect("should be Some").as_str(), "DUPLICATE_KEY");
    }

    #[test]
    fn symbolic_code_from_static_unknown_code() {
        let code = SymbolicCode::from_static("BOGUS_CODE");
        assert!(code.is_none());
    }

    #[test]
    fn symbolic_code_numeric_code_roundtrip() {
        let code = SymbolicCode::from_static("DUPLICATE_KEY").unwrap();
        assert_eq!(code.numeric_code(), Some(0x0101));
        assert_eq!(code.as_diagnostic_code(), Some(DiagnosticCode::new(0x0101)));
    }

    #[test]
    fn symbolic_code_display_is_name_not_hex() {
        let code = SymbolicCode::from_static("DUPLICATE_KEY").unwrap();
        assert_eq!(code.to_string(), "DUPLICATE_KEY");
    }

    #[test]
    fn symbolic_code_is_copy() {
        let a = SymbolicCode::from_static("TYPE_MISMATCH").unwrap();
        let b = a;
        assert_eq!(a, b);
        // Both usable after copy
        assert_eq!(a.as_str(), "TYPE_MISMATCH");
        assert_eq!(b.as_str(), "TYPE_MISMATCH");
    }

    #[test]
    fn symbolic_code_category() {
        let schema = SymbolicCode::from_static("DUPLICATE_KEY").unwrap();
        assert_eq!(schema.category(), Some(CodeCategory::Schema));

        let gate = SymbolicCode::from_static("EXPRESSION_STACK_EXCEEDED").unwrap();
        assert_eq!(gate.category(), Some(CodeCategory::Gate));

        let runtime = SymbolicCode::from_static("RUNTIME_TIMEOUT").unwrap();
        assert_eq!(runtime.category(), Some(CodeCategory::Runtime));
    }

    #[test]
    fn symbolic_code_from_str_accepts_registered_name() {
        let result: Result<SymbolicCode, _> = "DUPLICATE_KEY".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "DUPLICATE_KEY");
    }

    #[test]
    fn symbolic_code_from_str_rejects_unknown_name() {
        let result: Result<SymbolicCode, _> = "BOGUS_CODE".parse();
        assert!(result.is_err());
    }

    // ---- CODE_REGISTRY tests ----

    #[test]
    fn registry_symbolic_to_numeric_roundtrip() {
        let numeric = symbolic_to_numeric("DUPLICATE_KEY");
        assert_eq!(numeric, Some(0x0101));

        let symbolic = numeric_to_symbolic(0x0101);
        assert_eq!(symbolic, Some("DUPLICATE_KEY"));
    }

    #[test]
    fn registry_all_codes_non_zero() {
        for entry in CODE_REGISTRY {
            assert_ne!(
                entry.numeric, 0,
                "code {} has zero numeric value",
                entry.symbolic
            );
        }
    }

    #[test]
    fn registry_no_duplicate_numeric() {
        for i in 0..CODE_REGISTRY.len() {
            for j in (i + 1)..CODE_REGISTRY.len() {
                assert_ne!(
                    CODE_REGISTRY[i].numeric, CODE_REGISTRY[j].numeric,
                    "duplicate numeric {:04X} for {} and {}",
                    CODE_REGISTRY[i].numeric, CODE_REGISTRY[i].symbolic, CODE_REGISTRY[j].symbolic,
                );
            }
        }
    }

    #[test]
    fn registry_no_duplicate_symbolic() {
        for i in 0..CODE_REGISTRY.len() {
            for j in (i + 1)..CODE_REGISTRY.len() {
                assert_ne!(
                    CODE_REGISTRY[i].symbolic, CODE_REGISTRY[j].symbolic,
                    "duplicate symbolic '{}' at indices {} and {}",
                    CODE_REGISTRY[i].symbolic, i, j,
                );
            }
        }
    }

    #[test]
    fn diagnostic_code_symbolic_lookup_known_code() {
        let dc = DiagnosticCode::new(0x0101);
        let sc = dc.symbolic_code();
        assert!(sc.is_some());
        assert_eq!(sc.unwrap().as_str(), "DUPLICATE_KEY");
    }

    #[test]
    fn diagnostic_code_symbolic_lookup_unsupported_code() {
        let dc = DiagnosticCode::new(0xDEAD);
        let sc = dc.symbolic_code();
        assert!(sc.is_none());
    }

    // ---- Serialization tests ----

    #[test]
    fn symbolic_code_serde_json_roundtrip() {
        let code =
            SymbolicCode::from_static("DUPLICATE_KEY").expect("DUPLICATE_KEY should be registered");
        let json =
            serde_json::to_string(&code).expect("serialization must succeed for SymbolicCode");
        assert_eq!(json, "\"DUPLICATE_KEY\"");
        let restored: SymbolicCode =
            serde_json::from_str(&json).expect("deserialization must succeed for registered code");
        assert_eq!(restored, code);
    }

    #[test]
    fn symbolic_code_serde_json_rejects_unknown() {
        let result: Result<SymbolicCode, _> = serde_json::from_str("\"BOGUS_CODE\"");
        assert!(result.is_err(), "unregistered codes must be rejected");
    }

    // ---- Diagnostic tests ----

    #[test]
    fn diagnostic_new_from_symbolic_code() {
        let code = SymbolicCode::from_static("DUPLICATE_KEY").unwrap();
        let diag = Diagnostic::new(
            code,
            Box::<str>::from("duplicate key found"),
            Severity::Error,
            Span::ZERO,
            None,
        );

        assert_eq!(diagnostic.code, DiagnosticCode::new(0x0101));
        assert_eq!(diagnostic.message.as_ref(), "invalid workflow");
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.span, Span::ZERO);
        assert_eq!(diagnostic.source_file, None);
    }

    #[test]
    fn diagnostic_carries_source_file_when_provided() {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(0x0101),
            Box::<str>::from("test"),
            Severity::Error,
            Span::ZERO,
            Some(Box::<str>::from("workflow.yaml")),
        );

        assert_eq!(diagnostic.source_file.as_deref(), Some("workflow.yaml"));
    }

    #[test]
    fn diagnostic_backward_compat_source_file_none() {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(0x0101),
            Box::<str>::from("runtime error"),
            Severity::Warning,
            Span::ZERO,
            None,
        );

        assert!(diagnostic.source_file.is_none());
    }

    #[test]
    fn diagnostic_from_numeric_when_registered() {
        let diag = Diagnostic::from_numeric(
            DiagnosticCode::new(0x0101),
            Box::<str>::from("duplicate key"),
            Severity::Error,
            Span::ZERO,
        );

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.code.as_str(), "DUPLICATE_KEY");
        assert_eq!(diag.numeric_code.code(), 0x0101);
    }

    #[test]
    fn diagnostic_from_numeric_when_unregistered() {
        let diag = Diagnostic::from_numeric(
            DiagnosticCode::new(0xDEAD),
            Box::<str>::from("unknown"),
            Severity::Error,
            Span::ZERO,
        );

        assert!(diag.is_none());
    }

    // ---- DiagnosticCodeParseError exact variant assertions ----

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_missing_prefix() {
        let result = DiagnosticCode::from_str("0101");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_hex_digits() {
        let result = DiagnosticCode::from_str("E010G");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_too_short() {
        let result = DiagnosticCode::from_str("E01");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_too_long() {
        let result = DiagnosticCode::from_str("E010101");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_empty() {
        let result = DiagnosticCode::from_str("");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_unsupported_code_when_out_of_range() {
        let result = DiagnosticCode::from_str("E0410");
        assert_eq!(result, Err(DiagnosticCodeParseError::UnsupportedCode));
    }

    #[test]
    fn diagnostic_code_parse_error_unsupported_code_when_fully_outside_ranges() {
        let result = DiagnosticCode::from_str("E9999");
        assert_eq!(result, Err(DiagnosticCodeParseError::UnsupportedCode));
    }

    #[test]
    fn diagnostic_code_rejects_wrong_prefix_g() {
        // B26 variant: wrong prefix "G" should be InvalidFormat
        let result = DiagnosticCode::from_str("G0101");

        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parses_e010b_uppercase_hex() {
        // B25: E010B (uppercase B) is supported and within range
        let result = DiagnosticCode::from_str("E010B");

        assert_eq!(result, Ok(DiagnosticCode::new(0x010B)));
    }

    #[test]
    fn diagnostic_code_e0000_is_unsupported() {
        // E0000 is valid format but outside supported ranges
        let result = DiagnosticCode::from_str("E0000");

        assert_eq!(result, Err(DiagnosticCodeParseError::UnsupportedCode));
    }

    #[test]
    fn severity_has_three_variants() {
        // B32: Severity enum has Error, Warning, Info
        let error = Severity::Error;
        let warning = Severity::Warning;
        let info = Severity::Info;

        assert_ne!(error, warning);
        assert_ne!(warning, info);
        assert_ne!(error, info);
    }

    #[test]
    fn diagnostic_new_preserves_source_file_exactly() {
        // B20: source_file is preserved exactly
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(0x0201),
            Box::<str>::from("test msg"),
            Severity::Error,
            Span::ZERO,
            Some(Box::<str>::from("path/to/workflow.yaml")),
        );
        assert_eq!(
            diagnostic.source_file.as_deref(),
            Some("path/to/workflow.yaml")
        );
    }

    #[test]
    fn diagnostic_backward_compat_span_zero_none_source() {
        // B22: backward compat: Span::ZERO + None source_file
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(0x0101),
            Box::<str>::from("runtime error"),
            Severity::Warning,
            Span::ZERO,
            None,
        );

        assert_eq!(diagnostic.span, Span::ZERO);
        assert!(diagnostic.source_file.is_none());
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(diagnostic.message.as_ref(), "runtime error");
    }
}
