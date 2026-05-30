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
// CodeCategory — high-level code grouping
// ---------------------------------------------------------------------------

/// Groups diagnostic codes by their functional domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodeCategory {
    /// Schema validation errors: E01xx
    Schema,
    /// Reference validation errors: E02xx
    Reference,
    /// Control-flow validation errors: E03xx
    ControlFlow,
    /// Type, taint, and resource errors: E04xx
    TypeTaint,
    /// Gate verifier errors: E05xx
    Gate,
    /// Contract discovery errors: E06xx
    ContractDiscovery,
    /// Internal compilation errors: E10xx
    Compilation,
    /// Workflow IR errors: E11xx
    WorkflowIr,
    /// Expression errors: E12xx
    Expression,
    /// Accessor and path errors: E13xx
    Accessor,
    /// Lowering errors: E14xx
    Lowering,
    /// Storage errors: E20xx
    Storage,
    /// Runtime core errors: E30xx
    Runtime,
    /// IPC errors: E32xx
    Ipc,
    /// Lifecycle errors: E33xx
    Lifecycle,
    /// Runtime boundary errors: E40xx
    RuntimeBoundary,
    /// Internal invariant violations (fallback codes): E13xx
    Internal,
}

impl CodeCategory {
    /// Returns the canonical uppercase category short-name string.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            CodeCategory::Schema => "SCHEMA",
            CodeCategory::Reference => "REFERENCE",
            CodeCategory::ControlFlow => "CONTROL_FLOW",
            CodeCategory::TypeTaint => "TYPE_TAINT",
            CodeCategory::Gate => "GATE",
            CodeCategory::ContractDiscovery => "CONTRACT_DISCOVERY",
            CodeCategory::Compilation => "COMPILATION",
            CodeCategory::WorkflowIr => "WORKFLOW_IR",
            CodeCategory::Expression => "EXPRESSION",
            CodeCategory::Accessor => "ACCESSOR",
            CodeCategory::Lowering => "LOWERING",
            CodeCategory::Storage => "STORAGE",
            CodeCategory::Runtime => "RUNTIME",
            CodeCategory::Ipc => "IPC",
            CodeCategory::Lifecycle => "LIFECYCLE",
            CodeCategory::RuntimeBoundary => "RUNTIME_BOUNDARY",
            CodeCategory::Internal => "INTERNAL",
        }
    }
}

// ---------------------------------------------------------------------------
// CodeEntry and CODE_REGISTRY — single source of truth
// ---------------------------------------------------------------------------

/// An entry in the canonical diagnostic code registry.
///
/// Every diagnostic code used anywhere in the workspace MUST appear in
/// [`CODE_REGISTRY`]. The registry is immutable at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeEntry {
    /// The symbolic name (e.g., `"DUPLICATE_KEY"`).
    pub symbolic: &'static str,
    /// The packed numeric encoding (e.g., `0x0101`).
    pub numeric: u16,
    /// The functional category.
    pub category: CodeCategory,
    /// Whether this code is deprecated (retained for backward compatibility,
    /// should not appear in new diagnostics).
    pub deprecated: bool,
}

/// Canonical code registry. All diagnostic codes used anywhere in the
/// workspace MUST appear here. The registry is a bijection: no duplicate
/// symbolic names, no duplicate numeric codes.
///
/// # Bijection Guarantees
///
/// - Symbolic → numeric: exactly one mapping.
/// - Numeric → symbolic: exactly one mapping.
/// - All numeric codes are non-zero.
/// - Category consistency: `(numeric >> 8) & 0xFF` matches the category range.
pub const CODE_REGISTRY: &[CodeEntry] = &[
    // ---- Schema: E01xx (0x0101–0x010B) ----
    CodeEntry {
        symbolic: "DUPLICATE_KEY",
        numeric: 0x0101,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "FORBIDDEN_YAML_FEATURE",
        numeric: 0x0102,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNKNOWN_TOP_LEVEL_FIELD",
        numeric: 0x0103,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNKNOWN_STEP_FIELD",
        numeric: 0x0104,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "MISSING_REQUIRED_FIELD",
        numeric: 0x0105,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_VERSION",
        numeric: 0x0106,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_ID",
        numeric: 0x0107,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RESERVED_ID",
        numeric: 0x0108,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "DUPLICATE_ID",
        numeric: 0x0109,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "MULTIPLE_STEP_PRIMITIVES",
        numeric: 0x010A,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "MISSING_STEP_PRIMITIVE",
        numeric: 0x010B,
        category: CodeCategory::Schema,
        deprecated: false,
    },
    // ---- Reference: E02xx (0x0201–0x0204) ----
    CodeEntry {
        symbolic: "UNKNOWN_REFERENCE",
        numeric: 0x0201,
        category: CodeCategory::Reference,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "FUTURE_REFERENCE",
        numeric: 0x0202,
        category: CodeCategory::Reference,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SECRET_NOT_DECLARED",
        numeric: 0x0203,
        category: CodeCategory::Reference,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "DIRECT_RUNTIME_REFERENCE",
        numeric: 0x0204,
        category: CodeCategory::Reference,
        deprecated: false,
    },
    // ---- ControlFlow: E03xx (0x0301–0x0309) ----
    CodeEntry {
        symbolic: "INVALID_THEN_TARGET",
        numeric: 0x0301,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CONTROL_FLOW_CYCLE",
        numeric: 0x0302,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNREACHABLE_STEP",
        numeric: 0x0303,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_CHOOSE",
        numeric: 0x0304,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_FOR_EACH",
        numeric: 0x0305,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_TOGETHER",
        numeric: 0x0306,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_COLLECT",
        numeric: 0x0307,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_REDUCE",
        numeric: 0x0308,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_REPEAT",
        numeric: 0x0309,
        category: CodeCategory::ControlFlow,
        deprecated: false,
    },
    // ---- TypeTaint: E04xx (0x0401–0x040C) ----
    CodeEntry {
        symbolic: "INVALID_WAIT",
        numeric: 0x0401,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_ASK",
        numeric: 0x0402,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_FINISH",
        numeric: 0x0403,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_RETRY",
        numeric: 0x0404,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_ON_ERROR",
        numeric: 0x0405,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SECRET_RESULT_LEAK",
        numeric: 0x0406,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "TYPE_MISMATCH",
        numeric: 0x0407,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "PAYLOAD_TOO_LARGE",
        numeric: 0x0408,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LIMIT_REQUIRED",
        numeric: 0x0409,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LIMIT_EXCEEDED",
        numeric: 0x040A,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_TRIGGER",
        numeric: 0x040B,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "HTTP_TRIGGER_OUT_OF_CORE",
        numeric: 0x040C,
        category: CodeCategory::TypeTaint,
        deprecated: false,
    },
    // ---- Gate: E05xx (0x0501–0x0513) ----
    CodeEntry {
        symbolic: "EXPRESSION_STACK_EXCEEDED",
        numeric: 0x0501,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "EXPRESSION_STACK_MISMATCH",
        numeric: 0x0502,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACCESSOR_SLOT_OUT_OF_RANGE",
        numeric: 0x0503,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACCESSOR_PATH_INVALID",
        numeric: 0x0504,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SLOT_REFERENCE_OUT_OF_RANGE",
        numeric: 0x0505,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LOOP_BODY_STEP_OUT_OF_RANGE",
        numeric: 0x0506,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SLOT_DEPENDENCY_CYCLE",
        numeric: 0x0507,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "NODE_KIND_CONSTRAINT_VIOLATION",
        numeric: 0x0508,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACTION_CONTRACT_MISSING",
        numeric: 0x0509,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACTION_CONTRACT_ORPHAN",
        numeric: 0x050A,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SLOT_TYPE_INCONSISTENCY",
        numeric: 0x050B,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "NON_DETERMINISTIC_PATH",
        numeric: 0x050C,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CAPABILITY_NAME_EMPTY",
        numeric: 0x050D,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CAPABILITY_NAME_TOO_LONG",
        numeric: 0x050E,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CAPABILITY_NAME_INVALID",
        numeric: 0x050F,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CAPABILITY_ACTION_MISMATCH",
        numeric: 0x0510,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CAPABILITY_DUPLICATE",
        numeric: 0x0511,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACCESSOR_PATH_TOO_DEEP",
        numeric: 0x0512,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACCESSOR_SYMBOL_OUT_OF_BOUNDS",
        numeric: 0x0513,
        category: CodeCategory::Gate,
        deprecated: false,
    },
    // ---- ContractDiscovery: E06xx (0x0601–0x0603) ----
    CodeEntry {
        symbolic: "MISSING_SCHEMA_VERSION",
        numeric: 0x0601,
        category: CodeCategory::ContractDiscovery,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CUE_VET_FAILED",
        numeric: 0x0602,
        category: CodeCategory::ContractDiscovery,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "VERSION_MONOTONICITY_BREACH",
        numeric: 0x0603,
        category: CodeCategory::ContractDiscovery,
        deprecated: false,
    },
    // ---- Compilation internal: E10xx (0x1001–0x1006) ----
    CodeEntry {
        symbolic: "INVALID_PROGRAM_COUNTER",
        numeric: 0x1001,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "MISSING_NEXT_STEP",
        numeric: 0x1002,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNKNOWN_INPUT_SCHEMA_FIELD",
        numeric: 0x1003,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_TOP_LEVEL_DECLARATION",
        numeric: 0x1004,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNKNOWN_OUTPUT_NAME",
        numeric: 0x1005,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_ACCESSOR_REFERENCE",
        numeric: 0x1006,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SLOT_OUT_OF_BOUNDS",
        numeric: 0x1011,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SLOT_UNINITIALIZED",
        numeric: 0x1012,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CONST_OUT_OF_BOUNDS",
        numeric: 0x1013,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IDEMPOTENCY_VIOLATION",
        numeric: 0x1014,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "EXPR_OUT_OF_BOUNDS",
        numeric: 0x1015,
        category: CodeCategory::Compilation,
        deprecated: false,
    },
    // ---- WorkflowIR: E11xx (0x1101–0x1105) ----
    CodeEntry {
        symbolic: "CORE_TYPE_MISMATCH",
        numeric: 0x1101,
        category: CodeCategory::WorkflowIr,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "NON_FINITE_NUMBER",
        numeric: 0x1102,
        category: CodeCategory::WorkflowIr,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "DIVISION_BY_ZERO",
        numeric: 0x1103,
        category: CodeCategory::WorkflowIr,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "NON_BOOL_CONDITION",
        numeric: 0x1104,
        category: CodeCategory::WorkflowIr,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_COMPILED_WORKFLOW",
        numeric: 0x1105,
        category: CodeCategory::WorkflowIr,
        deprecated: false,
    },
    // ---- Expression: E12xx (0x1201–0x1203) ----
    CodeEntry {
        symbolic: "INVALID_EXPRESSION",
        numeric: 0x1203,
        category: CodeCategory::Expression,
        deprecated: false,
    },
    // ---- Accessor: E13xx (0x1311–0x1315) ----
    CodeEntry {
        symbolic: "ACCESSOR_CONST_OUT_OF_BOUNDS",
        numeric: 0x1315,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    // ---- Runtime storage/journal errors (from RuntimeError): E20xx (0x2001–0x201E) ----
    CodeEntry {
        symbolic: "QUEUE_FULL",
        numeric: 0x2001,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUN_NOT_FOUND",
        numeric: 0x2002,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACTIVE_RUN_CAPACITY_EXCEEDED",
        numeric: 0x2003,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUN_ALREADY_EXISTS",
        numeric: 0x2004,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_OPERATION",
        numeric: 0x2005,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SHUTDOWN_IN_PROGRESS",
        numeric: 0x2006,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_POISONED",
        numeric: 0x2007,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_JOURNAL_APPEND_FAILED",
        numeric: 0x2008,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_ASYNC_STRICT_ACK",
        numeric: 0x2009,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "FRAME_POOL_UNAVAILABLE",
        numeric: 0x200A,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_ACTION_COMPLETION",
        numeric: 0x200B,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_TIMER_FIRE",
        numeric: 0x200C,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_FULL_RECOVERY_HYDRATION",
        numeric: 0x200D,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_RECOVERY_HYDRATION",
        numeric: 0x200E,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "COMMAND_QUEUE_CAPACITY_EXCEEDED",
        numeric: 0x200F,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACTIVE_RUN_CAPACITY_ZERO",
        numeric: 0x2010,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ADMISSION_ARTIFACT_NOT_FOUND",
        numeric: 0x2011,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ADMISSION_CAPABILITY_DENIED",
        numeric: 0x2012,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ENCODE_FAILED",
        numeric: 0x2013,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ADMISSION_ARTIFACT_INVALID",
        numeric: 0x2014,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ADMISSION_HEADER_PERSISTENCE_FAILED",
        numeric: 0x2015,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SECRET_RESULT_NOT_ALLOWED",
        numeric: 0x2016,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_PAYLOAD_SIZE_EXCEEDED",
        numeric: 0x2017,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ADMISSION_ARTIFACT_DIGEST_MISMATCH",
        numeric: 0x2018,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ADMISSION_ARTIFACT_STALE",
        numeric: 0x2019,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ADMISSION_DIGEST_MISMATCH",
        numeric: 0x201A,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ENGINE_DRIVE_FAILED",
        numeric: 0x201B,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SHARD_NOT_FOUND",
        numeric: 0x201C,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "MIGRATE_SELF",
        numeric: 0x201D,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_FULL",
        numeric: 0x201E,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    // ---- Legacy storage infrastructure codes (relocated) ----
    CodeEntry {
        symbolic: "STORAGE_UNAVAILABLE",
        numeric: 0x2070,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_CORRUPTION",
        numeric: 0x2071,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_IO",
        numeric: 0x2072,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_ENCODING",
        numeric: 0x2073,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_DECODING",
        numeric: 0x2074,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_CHECKPOINT",
        numeric: 0x2075,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_SNAPSHOT",
        numeric: 0x2076,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_PAGE_OVERFLOW",
        numeric: 0x2077,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_KEYSPACE_MANIFEST",
        numeric: 0x2078,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_BLOB_LIMIT",
        numeric: 0x2079,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_WRITE_BUDGET",
        numeric: 0x207A,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_READ_BUDGET",
        numeric: 0x207B,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_COMPACTION_FAILED",
        numeric: 0x207C,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STORAGE_SEALED",
        numeric: 0x207D,
        category: CodeCategory::Storage,
        deprecated: false,
    },
    // ---- RuntimeCore: E30xx (0x3001–0x301B) ----
    CodeEntry {
        symbolic: "RUNTIME_TIMEOUT",
        numeric: 0x300F,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_BUDGET_EXHAUSTED",
        numeric: 0x3010,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_CYCLE_LIMIT",
        numeric: 0x3011,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_ACTION_DISPATCH",
        numeric: 0x3012,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_ACTION_TIMEOUT",
        numeric: 0x3013,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_SIGNAL_INVALID",
        numeric: 0x3014,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_QUEUE_OVERFLOW",
        numeric: 0x3015,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_JOURNAL_BATCH",
        numeric: 0x3016,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_TICK_OVERFLOW",
        numeric: 0x3017,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_STEP_LIMIT",
        numeric: 0x3018,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_TRACE_OVERFLOW",
        numeric: 0x3019,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_CAPACITY_EXCEEDED",
        numeric: 0x301A,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RUNTIME_INVALID_STATE",
        numeric: 0x301B,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    // ---- IPC: E32xx (0x3201–0x320A) ----
    CodeEntry {
        symbolic: "IPC_PAYLOAD_TOO_LARGE",
        numeric: 0x3201,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_DECODE_FAILED",
        numeric: 0x3202,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_ENCODE_FAILED",
        numeric: 0x3203,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_CHANNEL_CLOSED",
        numeric: 0x3204,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_CHANNEL_FULL",
        numeric: 0x3205,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_CONNECTION_REFUSED",
        numeric: 0x3206,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_TIMEOUT",
        numeric: 0x3207,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_PROTOCOL_VIOLATION",
        numeric: 0x3208,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_AUTH_FAILED",
        numeric: 0x3209,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "IPC_RESOURCE_UNAVAILABLE",
        numeric: 0x320A,
        category: CodeCategory::Ipc,
        deprecated: false,
    },
    // ---- Lifecycle: E33xx (0x3301–0x3304) ----
    CodeEntry {
        symbolic: "LIFECYCLE_STORAGE_UNAVAILABLE",
        numeric: 0x3301,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_DUPLICATE_REQUEST",
        numeric: 0x3302,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_INVALID_TRANSITION",
        numeric: 0x3303,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_STALE_BEAD",
        numeric: 0x3304,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    // ---- RuntimeBoundary: E40xx (0x4001–0x4020) ----
    // Storage/Journal codes from vb_storage/src/error/codes.rs
    CodeEntry {
        symbolic: "JOURNAL_FJALL",
        numeric: 0x4001,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_ENCODE",
        numeric: 0x4002,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_KEY_CAPACITY",
        numeric: 0x4003,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_DUPLICATE_EVENT",
        numeric: 0x4004,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_WRITE_LOCK_POISONED",
        numeric: 0x4005,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_QUEUE_CAPACITY",
        numeric: 0x4006,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_QUEUE_FULL",
        numeric: 0x4007,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_WRONG_RUN",
        numeric: 0x4008,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_SEQUENCE_GAP",
        numeric: 0x4009,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_SEQUENCE_OVERFLOW",
        numeric: 0x400A,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_BAD_MAGIC",
        numeric: 0x400B,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_UNSUPPORTED_SCHEMA",
        numeric: 0x400C,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_MIGRATION_REQUIRED",
        numeric: 0x400D,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_UNKNOWN_RECORD_KIND",
        numeric: 0x400E,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_FAMILY_MISMATCH",
        numeric: 0x400F,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_HEADER_LENGTH_MISMATCH",
        numeric: 0x4010,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_PAYLOAD_TOO_LARGE",
        numeric: 0x4011,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_HEADER_CHECKSUM",
        numeric: 0x4012,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_PAYLOAD_DIGEST_MISMATCH",
        numeric: 0x4013,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_UNEXPECTED_EOF",
        numeric: 0x4014,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_POSTCARD_DECODE",
        numeric: 0x4015,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_QUEUE_SHUTDOWN",
        numeric: 0x4016,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_ARTIFACT_MALFORMED",
        numeric: 0x4017,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_ARTIFACT_CHECKSUM",
        numeric: 0x4018,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_INVALID_GATE_COUNT",
        numeric: 0x401C,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_MISSING_PROOF_FLAG",
        numeric: 0x401D,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_ARTIFACT_NOT_FOUND",
        numeric: 0x4019,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_PROCESS_LOCK_HELD",
        numeric: 0x401A,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_PROCESS_LOCK_IO",
        numeric: 0x401B,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_TOO_MANY_EVENTS",
        numeric: 0x401E,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_REPLAY_ALLOC_FAIL",
        numeric: 0x401F,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_INVALID_EVENT",
        numeric: 0x4020,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    // ---- Additional journal/internal symbolic names used by kani models ----
    CodeEntry {
        symbolic: "JOURNAL_SEQ_MISMATCH",
        numeric: 0x4021,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_CHECKPOINT_MISMATCH",
        numeric: 0x4022,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_PAGE_ORDER_VIOLATION",
        numeric: 0x4023,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_EXTRA_HYDRATION_FAIL",
        numeric: 0x4024,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_EVIDENCE_OVERFLOW",
        numeric: 0x4025,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_SLOT_NOT_WRITABLE",
        numeric: 0x4026,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_DUPLICATE_ACTION",
        numeric: 0x4027,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_UNKNOWN_ACTION",
        numeric: 0x4028,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_STALE_EVENT",
        numeric: 0x4029,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_EVENT_ORDER",
        numeric: 0x402A,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_BATCH_OVERFLOW",
        numeric: 0x402B,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_CLOCK_DRIFT",
        numeric: 0x402C,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_BUFFER_OVERFLOW",
        numeric: 0x402D,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_SLOT_SEALED",
        numeric: 0x402E,
        category: CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    // ---- Action/audit codes (from CoreError runtime namespace) ----
    CodeEntry {
        symbolic: "ACTION_RESULT_AUDIT_MISMATCH",
        numeric: 0x3020,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACTION_TYPE_CONSTRAINT_FAIL",
        numeric: 0x3021,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ACTION_CIRCUIT_BREAKER_OPEN",
        numeric: 0x3022,
        category: CodeCategory::Runtime,
        deprecated: false,
    },
    // ---- Core Engine: E12xx (Step budget / counter overflow) ----
    CodeEntry {
        symbolic: "STEP_BUDGET_EXHAUSTED",
        numeric: 0x1201,
        category: CodeCategory::Expression,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STEP_COUNTER_OVERFLOW",
        numeric: 0x1202,
        category: CodeCategory::Expression,
        deprecated: false,
    },
    // ---- Core Engine: E13xx (Accessor / resource / internal errors) ----
    CodeEntry {
        symbolic: "CORE_QUEUE_FULL",
        numeric: 0x1301,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "RESOURCE_LIMIT_EXCEEDED",
        numeric: 0x1302,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "ALLOCATION_FAILED",
        numeric: 0x1303,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "EXPRESSION_STACK_OVERFLOW",
        numeric: 0x1304,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "MISSING_OUTPUT_SLOT",
        numeric: 0x1305,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "STEP_STATE_OUT_OF_BOUNDS",
        numeric: 0x1306,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "INVALID_COMPILED_WORKFLOW_CORE",
        numeric: 0x1307,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_PRIMITIVE",
        numeric: 0x1308,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_ACCESSOR_TRAVERSAL",
        numeric: 0x130A,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "EXPRESSION_STACK_UNDERFLOW",
        numeric: 0x130B,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "OBJECT_FIELD_NOT_FOUND",
        numeric: 0x130C,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LIST_INDEX_OUT_OF_BOUNDS",
        numeric: 0x130D,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "SYMBOL_OUT_OF_BOUNDS",
        numeric: 0x1311,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LIST_OUT_OF_BOUNDS",
        numeric: 0x1312,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "OBJECT_OUT_OF_BOUNDS",
        numeric: 0x1313,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "BLOB_OUT_OF_BOUNDS",
        numeric: 0x1314,
        category: CodeCategory::Accessor,
        deprecated: false,
    },
    // ---- Core Engine: E14xx (Lowering / budget / collect / repetition) ----
    CodeEntry {
        symbolic: "ITERATION_LIMIT_EXCEEDED",
        numeric: 0x1401,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "REPEAT_EXHAUSTED",
        numeric: 0x1402,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "COLLECT_PAGE_LIMIT_EXCEEDED",
        numeric: 0x1403,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "COLLECT_ITEM_LIMIT_EXCEEDED",
        numeric: 0x1404,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "TOGETHER_BRANCH_LIMIT_EXCEEDED",
        numeric: 0x1405,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "BUDGET_EXCEEDED",
        numeric: 0x1406,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "COLLECT_TIME_LIMIT_EXCEEDED",
        numeric: 0x1407,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "PARALLEL_LIMIT_EXCEEDED",
        numeric: 0x1408,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CAPABILITY_DENIED",
        numeric: 0x1409,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "BUDGET_PARSE",
        numeric: 0x140A,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "COLLECT_PAGE_ORDER_VIOLATION",
        numeric: 0x140B,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "COLLECT_EXTRA_HYDRATION_FAILED",
        numeric: 0x140C,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "COLLECT_EVIDENCE_CAPACITY_EXCEEDED",
        numeric: 0x140D,
        category: CodeCategory::Lowering,
        deprecated: false,
    },
    // ---- Core Engine: E15xx (Lifecycle / journal / replay errors) ----
    CodeEntry {
        symbolic: "CORE_LIFECYCLE_STORAGE_UNAVAILABLE",
        numeric: 0x1501,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CORE_LIFECYCLE_DUPLICATE_REQUEST",
        numeric: 0x1502,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "LIFECYCLE_STALE_REQUEST",
        numeric: 0x1503,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "CORE_LIFECYCLE_INVALID_TRANSITION",
        numeric: 0x1504,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "JOURNAL_WRITE_FAILURE",
        numeric: 0x1505,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    CodeEntry {
        symbolic: "REPLAY_CORRUPTION",
        numeric: 0x1506,
        category: CodeCategory::Lifecycle,
        deprecated: false,
    },
    // ---- Internal invariant (fallback code for HasSymbolicCode impls) ----
    CodeEntry {
        symbolic: "INTERNAL_INVARIANT_VIOLATION",
        numeric: 0x1309,
        category: CodeCategory::Internal,
        deprecated: false,
    },
];

// ---------------------------------------------------------------------------
// Registry lookup functions
// ---------------------------------------------------------------------------

/// Looks up a symbolic code name and returns its numeric encoding.
///
/// Returns `None` if the symbolic name is not registered.
#[must_use]
pub const fn symbolic_to_numeric(symbolic: &str) -> Option<u16> {
    let mut i = 0;
    while i < CODE_REGISTRY.len() {
        if let Some(entry) = CODE_REGISTRY.get(i) && entry.symbolic == symbolic {
            return Some(entry.numeric);
        }
        i = i.wrapping_add(1);
    }
    None
}

/// Looks up a numeric code and returns its symbolic name.
///
/// Returns `None` if the numeric code is not in the registry.
#[must_use]
pub const fn numeric_to_symbolic(numeric: u16) -> Option<&'static str> {
    let mut i = 0;
    while i < CODE_REGISTRY.len() {
        if let Some(entry) = CODE_REGISTRY.get(i) && entry.numeric == numeric {
            return Some(entry.symbolic);
        }
        i = i.wrapping_add(1);
    }
    None
}

/// Looks up a numeric code and returns the corresponding [`SymbolicCode`].
///
/// Returns `None` if the numeric code is not registered.
#[must_use]
pub fn numeric_to_symbolic_code(numeric: u16) -> Option<SymbolicCode> {
    numeric_to_symbolic(numeric).map(SymbolicCode)
}

/// Returns `true` when the given symbolic string is registered in
/// [`CODE_REGISTRY`].
#[must_use]
pub fn is_registered_symbolic(name: &str) -> bool {
    symbolic_to_numeric(name).is_some()
}

/// Returns `true` when the given numeric code is registered in
/// [`CODE_REGISTRY`].
#[must_use]
pub fn is_registered_numeric(code: u16) -> bool {
    numeric_to_symbolic(code).is_some()
}

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

/// Returns `true` when the numeric code is registered in
/// [`CODE_REGISTRY`].
///
/// Delegates to [`is_registered_numeric`], which uses `iter().find()`
/// over the CODE_REGISTRY.  This eliminates hardcoded range lists that
/// had previously drifted from the registry (e.g. the runtime range
/// 0x3001..=0x301B was missing entries at 0x3020-0x3022).
#[must_use]
fn is_supported_code(code: u16) -> bool {
    is_registered_numeric(code)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{
        CodeCategory, Diagnostic, DiagnosticCode, DiagnosticCodeParseError, Severity, SymbolicCode,
    };
    use crate::diagnostic::{CODE_REGISTRY, numeric_to_symbolic, symbolic_to_numeric};
    use crate::span::Span;
    use core::str::FromStr;

    // ---- DiagnosticCode existing tests ----

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

        assert_eq!(diag.code, code);
        assert_eq!(diag.numeric_code.code(), 0x0101);
        assert_eq!(diag.message.as_ref(), "duplicate key found");
        assert_eq!(diag.severity, Severity::Error);
        // Invariant: numeric_code.symbolic_code() == Some(code)
        assert_eq!(diag.numeric_code.symbolic_code(), Some(code));
    }

    #[test]
    fn diagnostic_from_numeric_when_registered() {
        let diag = Diagnostic::from_numeric(
            DiagnosticCode::new(0x0101),
            Box::<str>::from("duplicate key"),
            Severity::Error,
            Span::ZERO,
            None,
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
            None,
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

    // ---- is_supported_code extended range tests ----

    #[test]
    fn is_supported_code_accepts_e0501() {
        assert!(super::is_supported_code(0x0501));
    }

    #[test]
    fn is_supported_code_accepts_e0601() {
        assert!(super::is_supported_code(0x0601));
    }

    #[test]
    fn is_supported_code_accepts_e4020() {
        assert!(super::is_supported_code(0x4020));
    }

    #[test]
    fn is_supported_code_accepts_e402e() {
        assert!(super::is_supported_code(0x402E));
    }

    #[test]
    fn is_supported_code_rejects_e0604() {
        assert!(!super::is_supported_code(0x0604));
    }

    // ---- is_supported_code REPAIR-7: action/audit codes (0x3020-0x3022) ----

    #[test]
    fn is_supported_code_accepts_e3020() {
        assert!(
            super::is_supported_code(0x3020),
            "ACTION_RESULT_AUDIT_MISMATCH"
        );
    }

    #[test]
    fn is_supported_code_accepts_e3021() {
        assert!(
            super::is_supported_code(0x3021),
            "ACTION_TYPE_CONSTRAINT_FAIL"
        );
    }

    #[test]
    fn is_supported_code_accepts_e3022() {
        assert!(
            super::is_supported_code(0x3022),
            "ACTION_CIRCUIT_BREAKER_OPEN"
        );
    }

    #[test]
    fn is_supported_code_rejects_e301c_through_e301f() {
        // 0x301C-0x301F are genuine gaps between Runtime 0x301B and
        // action/audit codes at 0x3020-0x3022.
        for code in 0x301Cu16..=0x301F {
            assert!(
                !super::is_supported_code(code),
                "E{:04X} must be rejected",
                code
            );
        }
    }
}
