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

/// Constructs an active (non-deprecated) registry entry.
const fn active_code_entry(
    symbolic: &'static str,
    numeric: u16,
    category: CodeCategory,
) -> CodeEntry {
    CodeEntry {
        symbolic,
        numeric,
        category,
        deprecated: false,
    }
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
    active_code_entry("DUPLICATE_KEY", 0x0101, CodeCategory::Schema),
    active_code_entry("FORBIDDEN_YAML_FEATURE", 0x0102, CodeCategory::Schema),
    active_code_entry("UNKNOWN_TOP_LEVEL_FIELD", 0x0103, CodeCategory::Schema),
    active_code_entry("UNKNOWN_STEP_FIELD", 0x0104, CodeCategory::Schema),
    active_code_entry("MISSING_REQUIRED_FIELD", 0x0105, CodeCategory::Schema),
    active_code_entry("INVALID_VERSION", 0x0106, CodeCategory::Schema),
    active_code_entry("INVALID_ID", 0x0107, CodeCategory::Schema),
    active_code_entry("RESERVED_ID", 0x0108, CodeCategory::Schema),
    active_code_entry("DUPLICATE_ID", 0x0109, CodeCategory::Schema),
    active_code_entry("MULTIPLE_STEP_PRIMITIVES", 0x010A, CodeCategory::Schema),
    active_code_entry("MISSING_STEP_PRIMITIVE", 0x010B, CodeCategory::Schema),
    // ---- Reference: E02xx (0x0201–0x0204) ----
    active_code_entry("UNKNOWN_REFERENCE", 0x0201, CodeCategory::Reference),
    active_code_entry("FUTURE_REFERENCE", 0x0202, CodeCategory::Reference),
    active_code_entry("SECRET_NOT_DECLARED", 0x0203, CodeCategory::Reference),
    active_code_entry("DIRECT_RUNTIME_REFERENCE", 0x0204, CodeCategory::Reference),
    active_code_entry(
        "POSSIBLY_SKIPPED_REFERENCE",
        0x0205,
        CodeCategory::Reference,
    ),
    // ---- ControlFlow: E03xx (0x0301–0x0309) ----
    active_code_entry("INVALID_THEN_TARGET", 0x0301, CodeCategory::ControlFlow),
    active_code_entry("CONTROL_FLOW_CYCLE", 0x0302, CodeCategory::ControlFlow),
    active_code_entry("UNREACHABLE_STEP", 0x0303, CodeCategory::ControlFlow),
    active_code_entry("INVALID_CHOOSE", 0x0304, CodeCategory::ControlFlow),
    active_code_entry("INVALID_FOR_EACH", 0x0305, CodeCategory::ControlFlow),
    active_code_entry("INVALID_TOGETHER", 0x0306, CodeCategory::ControlFlow),
    active_code_entry("INVALID_COLLECT", 0x0307, CodeCategory::ControlFlow),
    active_code_entry("INVALID_REDUCE", 0x0308, CodeCategory::ControlFlow),
    active_code_entry("INVALID_REPEAT", 0x0309, CodeCategory::ControlFlow),
    // ---- TypeTaint: E04xx (0x0401–0x040C) ----
    active_code_entry("INVALID_WAIT", 0x0401, CodeCategory::TypeTaint),
    active_code_entry("INVALID_ASK", 0x0402, CodeCategory::TypeTaint),
    active_code_entry("INVALID_FINISH", 0x0403, CodeCategory::TypeTaint),
    active_code_entry("INVALID_RETRY", 0x0404, CodeCategory::TypeTaint),
    active_code_entry("INVALID_ON_ERROR", 0x0405, CodeCategory::TypeTaint),
    active_code_entry("SECRET_RESULT_LEAK", 0x0406, CodeCategory::TypeTaint),
    active_code_entry("TYPE_MISMATCH", 0x0407, CodeCategory::TypeTaint),
    active_code_entry("PAYLOAD_TOO_LARGE", 0x0408, CodeCategory::TypeTaint),
    active_code_entry("LIMIT_REQUIRED", 0x0409, CodeCategory::TypeTaint),
    active_code_entry("LIMIT_EXCEEDED", 0x040A, CodeCategory::TypeTaint),
    active_code_entry("UNSUPPORTED_TRIGGER", 0x040B, CodeCategory::TypeTaint),
    active_code_entry("HTTP_TRIGGER_OUT_OF_CORE", 0x040C, CodeCategory::TypeTaint),
    // ---- Gate: E05xx (0x0501–0x0513) ----
    active_code_entry("EXPRESSION_STACK_EXCEEDED", 0x0501, CodeCategory::Gate),
    active_code_entry("EXPRESSION_STACK_MISMATCH", 0x0502, CodeCategory::Gate),
    active_code_entry("ACCESSOR_SLOT_OUT_OF_RANGE", 0x0503, CodeCategory::Gate),
    active_code_entry("ACCESSOR_PATH_INVALID", 0x0504, CodeCategory::Gate),
    active_code_entry("SLOT_REFERENCE_OUT_OF_RANGE", 0x0505, CodeCategory::Gate),
    active_code_entry("LOOP_BODY_STEP_OUT_OF_RANGE", 0x0506, CodeCategory::Gate),
    active_code_entry("SLOT_DEPENDENCY_CYCLE", 0x0507, CodeCategory::Gate),
    active_code_entry("NODE_KIND_CONSTRAINT_VIOLATION", 0x0508, CodeCategory::Gate),
    active_code_entry("ACTION_CONTRACT_MISSING", 0x0509, CodeCategory::Gate),
    active_code_entry("ACTION_CONTRACT_ORPHAN", 0x050A, CodeCategory::Gate),
    active_code_entry("SLOT_TYPE_INCONSISTENCY", 0x050B, CodeCategory::Gate),
    active_code_entry("NON_DETERMINISTIC_PATH", 0x050C, CodeCategory::Gate),
    active_code_entry("CAPABILITY_NAME_EMPTY", 0x050D, CodeCategory::Gate),
    active_code_entry("CAPABILITY_NAME_TOO_LONG", 0x050E, CodeCategory::Gate),
    active_code_entry("CAPABILITY_NAME_INVALID", 0x050F, CodeCategory::Gate),
    active_code_entry("CAPABILITY_ACTION_MISMATCH", 0x0510, CodeCategory::Gate),
    active_code_entry("CAPABILITY_DUPLICATE", 0x0511, CodeCategory::Gate),
    active_code_entry("ACCESSOR_PATH_TOO_DEEP", 0x0512, CodeCategory::Gate),
    active_code_entry("ACCESSOR_SYMBOL_OUT_OF_BOUNDS", 0x0513, CodeCategory::Gate),
    // ---- ContractDiscovery: E06xx (0x0601–0x0603) ----
    active_code_entry(
        "MISSING_SCHEMA_VERSION",
        0x0601,
        CodeCategory::ContractDiscovery,
    ),
    active_code_entry("CUE_VET_FAILED", 0x0602, CodeCategory::ContractDiscovery),
    active_code_entry(
        "VERSION_MONOTONICITY_BREACH",
        0x0603,
        CodeCategory::ContractDiscovery,
    ),
    // ---- Compilation internal: E10xx (0x1001–0x1006) ----
    active_code_entry("INVALID_PROGRAM_COUNTER", 0x1001, CodeCategory::Compilation),
    active_code_entry("MISSING_NEXT_STEP", 0x1002, CodeCategory::Compilation),
    active_code_entry(
        "UNKNOWN_INPUT_SCHEMA_FIELD",
        0x1003,
        CodeCategory::Compilation,
    ),
    active_code_entry(
        "UNSUPPORTED_TOP_LEVEL_DECLARATION",
        0x1004,
        CodeCategory::Compilation,
    ),
    active_code_entry("UNKNOWN_OUTPUT_NAME", 0x1005, CodeCategory::Compilation),
    active_code_entry(
        "UNSUPPORTED_ACCESSOR_REFERENCE",
        0x1006,
        CodeCategory::Compilation,
    ),
    active_code_entry("SLOT_OUT_OF_BOUNDS", 0x1011, CodeCategory::Compilation),
    active_code_entry("SLOT_UNINITIALIZED", 0x1012, CodeCategory::Compilation),
    active_code_entry("CONST_OUT_OF_BOUNDS", 0x1013, CodeCategory::Compilation),
    active_code_entry("IDEMPOTENCY_VIOLATION", 0x1014, CodeCategory::Compilation),
    active_code_entry("EXPR_OUT_OF_BOUNDS", 0x1015, CodeCategory::Compilation),
    // ---- WorkflowIR: E11xx (0x1101–0x1105) ----
    active_code_entry("CORE_TYPE_MISMATCH", 0x1101, CodeCategory::WorkflowIr),
    active_code_entry("NON_FINITE_NUMBER", 0x1102, CodeCategory::WorkflowIr),
    active_code_entry("DIVISION_BY_ZERO", 0x1103, CodeCategory::WorkflowIr),
    active_code_entry("NON_BOOL_CONDITION", 0x1104, CodeCategory::WorkflowIr),
    active_code_entry(
        "INVALID_COMPILED_WORKFLOW",
        0x1105,
        CodeCategory::WorkflowIr,
    ),
    // ---- Expression: E12xx (0x1201–0x1203) ----
    active_code_entry("INVALID_EXPRESSION", 0x1203, CodeCategory::Expression),
    // ---- Accessor: E13xx (0x1311–0x1315) ----
    active_code_entry(
        "ACCESSOR_CONST_OUT_OF_BOUNDS",
        0x1315,
        CodeCategory::Accessor,
    ),
    // ---- Runtime storage/journal errors (from RuntimeError): E20xx (0x2001–0x201E) ----
    active_code_entry("QUEUE_FULL", 0x2001, CodeCategory::Storage),
    active_code_entry("RUN_NOT_FOUND", 0x2002, CodeCategory::Storage),
    active_code_entry(
        "ACTIVE_RUN_CAPACITY_EXCEEDED",
        0x2003,
        CodeCategory::Storage,
    ),
    active_code_entry("RUN_ALREADY_EXISTS", 0x2004, CodeCategory::Storage),
    active_code_entry("UNSUPPORTED_OPERATION", 0x2005, CodeCategory::Storage),
    active_code_entry("SHUTDOWN_IN_PROGRESS", 0x2006, CodeCategory::Storage),
    active_code_entry("JOURNAL_POISONED", 0x2007, CodeCategory::Storage),
    active_code_entry(
        "STORAGE_JOURNAL_APPEND_FAILED",
        0x2008,
        CodeCategory::Storage,
    ),
    active_code_entry(
        "UNSUPPORTED_ASYNC_STRICT_ACK",
        0x2009,
        CodeCategory::Storage,
    ),
    active_code_entry("FRAME_POOL_UNAVAILABLE", 0x200A, CodeCategory::Storage),
    active_code_entry("INVALID_ACTION_COMPLETION", 0x200B, CodeCategory::Storage),
    active_code_entry("INVALID_TIMER_FIRE", 0x200C, CodeCategory::Storage),
    active_code_entry(
        "UNSUPPORTED_FULL_RECOVERY_HYDRATION",
        0x200D,
        CodeCategory::Storage,
    ),
    active_code_entry("INVALID_RECOVERY_HYDRATION", 0x200E, CodeCategory::Storage),
    active_code_entry(
        "COMMAND_QUEUE_CAPACITY_EXCEEDED",
        0x200F,
        CodeCategory::Storage,
    ),
    active_code_entry("ACTIVE_RUN_CAPACITY_ZERO", 0x2010, CodeCategory::Storage),
    active_code_entry(
        "ADMISSION_ARTIFACT_NOT_FOUND",
        0x2011,
        CodeCategory::Storage,
    ),
    active_code_entry("ADMISSION_CAPABILITY_DENIED", 0x2012, CodeCategory::Storage),
    active_code_entry("ENCODE_FAILED", 0x2013, CodeCategory::Storage),
    active_code_entry("ADMISSION_ARTIFACT_INVALID", 0x2014, CodeCategory::Storage),
    active_code_entry(
        "ADMISSION_HEADER_PERSISTENCE_FAILED",
        0x2015,
        CodeCategory::Storage,
    ),
    active_code_entry("SECRET_RESULT_NOT_ALLOWED", 0x2016, CodeCategory::Storage),
    active_code_entry("IPC_PAYLOAD_SIZE_EXCEEDED", 0x2017, CodeCategory::Storage),
    active_code_entry(
        "ADMISSION_ARTIFACT_DIGEST_MISMATCH",
        0x2018,
        CodeCategory::Storage,
    ),
    active_code_entry("ADMISSION_ARTIFACT_STALE", 0x2019, CodeCategory::Storage),
    active_code_entry("ADMISSION_DIGEST_MISMATCH", 0x201A, CodeCategory::Storage),
    active_code_entry("ENGINE_DRIVE_FAILED", 0x201B, CodeCategory::Storage),
    active_code_entry("SHARD_NOT_FOUND", 0x201C, CodeCategory::Storage),
    active_code_entry("MIGRATE_SELF", 0x201D, CodeCategory::Storage),
    active_code_entry("JOURNAL_FULL", 0x201E, CodeCategory::Storage),
    // ---- Legacy storage infrastructure codes (relocated) ----
    active_code_entry("STORAGE_UNAVAILABLE", 0x2070, CodeCategory::Storage),
    active_code_entry("STORAGE_CORRUPTION", 0x2071, CodeCategory::Storage),
    active_code_entry("STORAGE_IO", 0x2072, CodeCategory::Storage),
    active_code_entry("STORAGE_ENCODING", 0x2073, CodeCategory::Storage),
    active_code_entry("STORAGE_DECODING", 0x2074, CodeCategory::Storage),
    active_code_entry("STORAGE_CHECKPOINT", 0x2075, CodeCategory::Storage),
    active_code_entry("STORAGE_SNAPSHOT", 0x2076, CodeCategory::Storage),
    active_code_entry("STORAGE_PAGE_OVERFLOW", 0x2077, CodeCategory::Storage),
    active_code_entry("STORAGE_KEYSPACE_MANIFEST", 0x2078, CodeCategory::Storage),
    active_code_entry("STORAGE_BLOB_LIMIT", 0x2079, CodeCategory::Storage),
    active_code_entry("STORAGE_WRITE_BUDGET", 0x207A, CodeCategory::Storage),
    active_code_entry("STORAGE_READ_BUDGET", 0x207B, CodeCategory::Storage),
    active_code_entry("STORAGE_COMPACTION_FAILED", 0x207C, CodeCategory::Storage),
    active_code_entry("STORAGE_SEALED", 0x207D, CodeCategory::Storage),
    // ---- RuntimeCore: E30xx (0x3001–0x301B) ----
    active_code_entry("RUNTIME_TIMEOUT", 0x300F, CodeCategory::Runtime),
    active_code_entry("RUNTIME_BUDGET_EXHAUSTED", 0x3010, CodeCategory::Runtime),
    active_code_entry("RUNTIME_CYCLE_LIMIT", 0x3011, CodeCategory::Runtime),
    active_code_entry("RUNTIME_ACTION_DISPATCH", 0x3012, CodeCategory::Runtime),
    active_code_entry("RUNTIME_ACTION_TIMEOUT", 0x3013, CodeCategory::Runtime),
    active_code_entry("RUNTIME_SIGNAL_INVALID", 0x3014, CodeCategory::Runtime),
    active_code_entry("RUNTIME_QUEUE_OVERFLOW", 0x3015, CodeCategory::Runtime),
    active_code_entry("RUNTIME_JOURNAL_BATCH", 0x3016, CodeCategory::Runtime),
    active_code_entry("RUNTIME_TICK_OVERFLOW", 0x3017, CodeCategory::Runtime),
    active_code_entry("RUNTIME_STEP_LIMIT", 0x3018, CodeCategory::Runtime),
    active_code_entry("RUNTIME_TRACE_OVERFLOW", 0x3019, CodeCategory::Runtime),
    active_code_entry("RUNTIME_CAPACITY_EXCEEDED", 0x301A, CodeCategory::Runtime),
    active_code_entry("RUNTIME_INVALID_STATE", 0x301B, CodeCategory::Runtime),
    // ---- IPC: E32xx (0x3201–0x320A) ----
    active_code_entry("IPC_PAYLOAD_TOO_LARGE", 0x3201, CodeCategory::Ipc),
    active_code_entry("IPC_DECODE_FAILED", 0x3202, CodeCategory::Ipc),
    active_code_entry("IPC_ENCODE_FAILED", 0x3203, CodeCategory::Ipc),
    active_code_entry("IPC_CHANNEL_CLOSED", 0x3204, CodeCategory::Ipc),
    active_code_entry("IPC_CHANNEL_FULL", 0x3205, CodeCategory::Ipc),
    active_code_entry("IPC_CONNECTION_REFUSED", 0x3206, CodeCategory::Ipc),
    active_code_entry("IPC_TIMEOUT", 0x3207, CodeCategory::Ipc),
    active_code_entry("IPC_PROTOCOL_VIOLATION", 0x3208, CodeCategory::Ipc),
    active_code_entry("IPC_AUTH_FAILED", 0x3209, CodeCategory::Ipc),
    active_code_entry("IPC_RESOURCE_UNAVAILABLE", 0x320A, CodeCategory::Ipc),
    // ---- Lifecycle: E33xx (0x3301–0x3304) ----
    active_code_entry(
        "LIFECYCLE_STORAGE_UNAVAILABLE",
        0x3301,
        CodeCategory::Lifecycle,
    ),
    active_code_entry(
        "LIFECYCLE_DUPLICATE_REQUEST",
        0x3302,
        CodeCategory::Lifecycle,
    ),
    active_code_entry(
        "LIFECYCLE_INVALID_TRANSITION",
        0x3303,
        CodeCategory::Lifecycle,
    ),
    active_code_entry("LIFECYCLE_STALE_BEAD", 0x3304, CodeCategory::Lifecycle),
    // ---- RuntimeBoundary: E40xx (0x4001–0x4020) ----
    // Storage/Journal codes from vb_storage/src/error/codes.rs
    active_code_entry("JOURNAL_FJALL", 0x4001, CodeCategory::RuntimeBoundary),
    active_code_entry("JOURNAL_ENCODE", 0x4002, CodeCategory::RuntimeBoundary),
    active_code_entry(
        "JOURNAL_KEY_CAPACITY",
        0x4003,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_DUPLICATE_EVENT",
        0x4004,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_WRITE_LOCK_POISONED",
        0x4005,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_QUEUE_CAPACITY",
        0x4006,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry("JOURNAL_QUEUE_FULL", 0x4007, CodeCategory::RuntimeBoundary),
    active_code_entry(
        "JOURNAL_BATCH_BYTES_EXCEEDED",
        0x402F,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "MALFORMED_KEYSPACE_ROW",
        0x4030,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_DUPLICATE_STAGED_KEY",
        0x4031,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "ARTIFACT_POSTCARD_ENCODE_FAILED",
        0x4032,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry("JOURNAL_WRONG_RUN", 0x4008, CodeCategory::RuntimeBoundary),
    active_code_entry(
        "JOURNAL_SEQUENCE_GAP",
        0x4009,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_SEQUENCE_OVERFLOW",
        0x400A,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry("JOURNAL_BAD_MAGIC", 0x400B, CodeCategory::RuntimeBoundary),
    active_code_entry(
        "JOURNAL_UNSUPPORTED_SCHEMA",
        0x400C,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_MIGRATION_REQUIRED",
        0x400D,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_UNKNOWN_RECORD_KIND",
        0x400E,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_FAMILY_MISMATCH",
        0x400F,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_HEADER_LENGTH_MISMATCH",
        0x4010,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_PAYLOAD_TOO_LARGE",
        0x4011,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_HEADER_CHECKSUM",
        0x4012,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_PAYLOAD_DIGEST_MISMATCH",
        0x4013,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_UNEXPECTED_EOF",
        0x4014,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_POSTCARD_DECODE",
        0x4015,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_QUEUE_SHUTDOWN",
        0x4016,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_ARTIFACT_MALFORMED",
        0x4017,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_ARTIFACT_CHECKSUM",
        0x4018,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_INVALID_GATE_COUNT",
        0x401C,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_MISSING_PROOF_FLAG",
        0x401D,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_ARTIFACT_NOT_FOUND",
        0x4019,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_PROCESS_LOCK_HELD",
        0x401A,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_PROCESS_LOCK_IO",
        0x401B,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_TOO_MANY_EVENTS",
        0x401E,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_REPLAY_ALLOC_FAIL",
        0x401F,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_INVALID_EVENT",
        0x4020,
        CodeCategory::RuntimeBoundary,
    ),
    // ---- Additional journal/internal symbolic names used by kani models ----
    active_code_entry(
        "JOURNAL_SEQ_MISMATCH",
        0x4021,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_CHECKPOINT_MISMATCH",
        0x4022,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_PAGE_ORDER_VIOLATION",
        0x4023,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_EXTRA_HYDRATION_FAIL",
        0x4024,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_EVIDENCE_OVERFLOW",
        0x4025,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_SLOT_NOT_WRITABLE",
        0x4026,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_DUPLICATE_ACTION",
        0x4027,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry(
        "JOURNAL_UNKNOWN_ACTION",
        0x4028,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry("JOURNAL_STALE_EVENT", 0x4029, CodeCategory::RuntimeBoundary),
    active_code_entry("JOURNAL_EVENT_ORDER", 0x402A, CodeCategory::RuntimeBoundary),
    active_code_entry(
        "JOURNAL_BATCH_OVERFLOW",
        0x402B,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry("JOURNAL_CLOCK_DRIFT", 0x402C, CodeCategory::RuntimeBoundary),
    active_code_entry(
        "JOURNAL_BUFFER_OVERFLOW",
        0x402D,
        CodeCategory::RuntimeBoundary,
    ),
    active_code_entry("JOURNAL_SLOT_SEALED", 0x402E, CodeCategory::RuntimeBoundary),
    // ---- Action/audit codes (from CoreError runtime namespace) ----
    active_code_entry(
        "ACTION_RESULT_AUDIT_MISMATCH",
        0x3020,
        CodeCategory::Runtime,
    ),
    active_code_entry("ACTION_TYPE_CONSTRAINT_FAIL", 0x3021, CodeCategory::Runtime),
    active_code_entry("ACTION_CIRCUIT_BREAKER_OPEN", 0x3022, CodeCategory::Runtime),
    // ---- Core Engine: E12xx (Step budget / counter overflow) ----
    active_code_entry("STEP_BUDGET_EXHAUSTED", 0x1201, CodeCategory::Expression),
    active_code_entry("STEP_COUNTER_OVERFLOW", 0x1202, CodeCategory::Expression),
    // ---- Core Engine: E13xx (Accessor / resource / internal errors) ----
    active_code_entry("CORE_QUEUE_FULL", 0x1301, CodeCategory::Accessor),
    active_code_entry("RESOURCE_LIMIT_EXCEEDED", 0x1302, CodeCategory::Accessor),
    active_code_entry("ALLOCATION_FAILED", 0x1303, CodeCategory::Accessor),
    active_code_entry("EXPRESSION_STACK_OVERFLOW", 0x1304, CodeCategory::Accessor),
    active_code_entry("MISSING_OUTPUT_SLOT", 0x1305, CodeCategory::Accessor),
    active_code_entry("STEP_STATE_OUT_OF_BOUNDS", 0x1306, CodeCategory::Accessor),
    active_code_entry(
        "INVALID_COMPILED_WORKFLOW_CORE",
        0x1307,
        CodeCategory::Accessor,
    ),
    active_code_entry("UNSUPPORTED_PRIMITIVE", 0x1308, CodeCategory::Accessor),
    active_code_entry(
        "UNSUPPORTED_ACCESSOR_TRAVERSAL",
        0x130A,
        CodeCategory::Accessor,
    ),
    active_code_entry("EXPRESSION_STACK_UNDERFLOW", 0x130B, CodeCategory::Accessor),
    active_code_entry("OBJECT_FIELD_NOT_FOUND", 0x130C, CodeCategory::Accessor),
    active_code_entry("LIST_INDEX_OUT_OF_BOUNDS", 0x130D, CodeCategory::Accessor),
    active_code_entry("SYMBOL_OUT_OF_BOUNDS", 0x1311, CodeCategory::Accessor),
    active_code_entry("LIST_OUT_OF_BOUNDS", 0x1312, CodeCategory::Accessor),
    active_code_entry("OBJECT_OUT_OF_BOUNDS", 0x1313, CodeCategory::Accessor),
    active_code_entry("BLOB_OUT_OF_BOUNDS", 0x1314, CodeCategory::Accessor),
    // ---- Core Engine: E14xx (Lowering / budget / collect / repetition) ----
    active_code_entry("ITERATION_LIMIT_EXCEEDED", 0x1401, CodeCategory::Lowering),
    active_code_entry("REPEAT_EXHAUSTED", 0x1402, CodeCategory::Lowering),
    active_code_entry(
        "COLLECT_PAGE_LIMIT_EXCEEDED",
        0x1403,
        CodeCategory::Lowering,
    ),
    active_code_entry(
        "COLLECT_ITEM_LIMIT_EXCEEDED",
        0x1404,
        CodeCategory::Lowering,
    ),
    active_code_entry(
        "TOGETHER_BRANCH_LIMIT_EXCEEDED",
        0x1405,
        CodeCategory::Lowering,
    ),
    active_code_entry("BUDGET_EXCEEDED", 0x1406, CodeCategory::Lowering),
    active_code_entry(
        "COLLECT_TIME_LIMIT_EXCEEDED",
        0x1407,
        CodeCategory::Lowering,
    ),
    active_code_entry("PARALLEL_LIMIT_EXCEEDED", 0x1408, CodeCategory::Lowering),
    active_code_entry("CAPABILITY_DENIED", 0x1409, CodeCategory::Lowering),
    active_code_entry("BUDGET_PARSE", 0x140A, CodeCategory::Lowering),
    active_code_entry(
        "COLLECT_PAGE_ORDER_VIOLATION",
        0x140B,
        CodeCategory::Lowering,
    ),
    active_code_entry(
        "COLLECT_EXTRA_HYDRATION_FAILED",
        0x140C,
        CodeCategory::Lowering,
    ),
    active_code_entry("EVIDENCE_CAPACITY_EXCEEDED", 0x140D, CodeCategory::Lowering),
    // ---- Core Engine: E15xx (Lifecycle / journal / replay errors) ----
    active_code_entry(
        "CORE_LIFECYCLE_STORAGE_UNAVAILABLE",
        0x1501,
        CodeCategory::Lifecycle,
    ),
    active_code_entry(
        "CORE_LIFECYCLE_DUPLICATE_REQUEST",
        0x1502,
        CodeCategory::Lifecycle,
    ),
    active_code_entry("LIFECYCLE_STALE_REQUEST", 0x1503, CodeCategory::Lifecycle),
    active_code_entry(
        "CORE_LIFECYCLE_INVALID_TRANSITION",
        0x1504,
        CodeCategory::Lifecycle,
    ),
    active_code_entry("JOURNAL_WRITE_FAILURE", 0x1505, CodeCategory::Lifecycle),
    active_code_entry("REPLAY_CORRUPTION", 0x1506, CodeCategory::Lifecycle),
    active_code_entry("ACTION_FAILED", 0x1507, CodeCategory::Lifecycle),
    active_code_entry("ACTION_RESUME_REJECTED", 0x1508, CodeCategory::Lifecycle),
    // ---- Internal invariant (fallback code for HasSymbolicCode impls) ----
    active_code_entry(
        "INTERNAL_INVARIANT_VIOLATION",
        0x1309,
        CodeCategory::Internal,
    ),
];

// ---------------------------------------------------------------------------
// Registry lookup functions
// ---------------------------------------------------------------------------

/// Looks up a symbolic code name and returns its numeric encoding.
///
/// Returns `None` if the symbolic name is not registered.
#[must_use]
pub fn symbolic_to_numeric(symbolic: &str) -> Option<u16> {
    for entry in CODE_REGISTRY {
        if entry.symbolic == symbolic {
            return Some(entry.numeric);
        }
    }
    None
}

/// Looks up a numeric code and returns its symbolic name.
///
/// Returns `None` if the numeric code is not in the registry.
#[must_use]
pub fn numeric_to_symbolic(numeric: u16) -> Option<&'static str> {
    for entry in CODE_REGISTRY {
        if entry.numeric == numeric {
            return Some(entry.symbolic);
        }
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
pub(crate) fn is_supported_code(code: u16) -> bool {
    is_registered_numeric(code)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// Tests extracted to: crates/workspace_tests/tests/vb_test_validate_diagnostic_behavior.rs
// and related diagnostic test files.

#[cfg(test)]
#[path = "diagnostic/tests_and_verification.rs"]
mod tests_and_verification;
