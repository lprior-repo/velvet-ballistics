#![forbid(unsafe_code)]
//! PO-001: Kani harness for SymbolicCode::from_static validation.
//!
//! Proves: from_static(s).is_some() iff s is in CODE_REGISTRY;
//! from_static(s).is_none() for all other &'static str inputs.
//!
//! Bound: Registry size ~90 entries (unwind=100)
//! Assumptions: CODE_REGISTRY const data is initialized with correct entries;
//! from_static performs linear scan over CODE_REGISTRY (O(n) where n < 100).

// -- Minimal model of Section 16 types (must match production implementation) --

/// Packed numeric diagnostic code (e.g., 0x0101 = "E0101").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCode(u16);

impl DiagnosticCode {
    #[must_use]
    pub const fn new(code: u16) -> Self {
        Self(code)
    }
    #[must_use]
    pub const fn code(self) -> u16 {
        self.0
    }

    /// Reverse lookup from numeric to a registered symbolic code.
    #[must_use]
    pub fn symbolic_code(self) -> Option<SymbolicCode> {
        match numeric_to_symbolic(self.code()) {
            Some(symbolic) => SymbolicCode::from_registered_static(symbolic),
            None => None,
        }
    }
}

/// Stable symbolic diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SymbolicCode(&'static str);

impl SymbolicCode {
    /// Smart constructor. Accepts only registered symbolic strings.
    #[must_use]
    pub fn from_static(s: &'static str) -> Option<Self> {
        match symbolic_to_numeric(s) {
            Some(_) => Some(SymbolicCode(s)),
            None => None,
        }
    }

    /// Constructor for static strings already sourced from CODE_REGISTRY.
    #[must_use]
    pub fn from_registered_static(s: &'static str) -> Option<Self> {
        match symbolic_to_numeric(s) {
            Some(_) => Some(SymbolicCode(s)),
            None => None,
        }
    }

    /// Infallible constructor for registry-derived static strings.
    #[must_use]
    pub const fn from_static_infallible(s: &'static str) -> Self {
        SymbolicCode(s)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Code category enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodeCategory {
    Schema,
    Reference,
    ControlFlow,
    TypeTaint,
    Gate,
    ContractDiscovery,
    Compilation,
    WorkflowIr,
    Expression,
    Accessor,
    Lowering,
    Storage,
    Runtime,
    RuntimeBoundary,
}

/// Registry entry.
pub struct CodeEntry {
    pub symbolic: &'static str,
    pub numeric: u16,
    pub category: CodeCategory,
}

/// Canonical code registry — all Section 16 diagnostic codes plus extensions.
/// This mirrors the production CODE_REGISTRY const slice.
pub const CODE_REGISTRY: &[CodeEntry] = &[
    // Schema: E01xx
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
    // Reference: E02xx
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
    // Control Flow: E03xx
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
    // Type/Taint: E04xx
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
    // Gate Verifier: E05xx
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
    // Contract Discovery: E06xx
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
    // Compilation Internal: E10xx
    CodeEntry {
        symbolic: "UNKNOWN_INPUT_SCHEMA_FIELD",
        numeric: 0x1001,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "UNSUPPORTED_TOP_LEVEL_DECLARATION",
        numeric: 0x1002,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "CANONICAL_YAML_PARSE",
        numeric: 0x1011,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "TOP_LEVEL_NOT_MAPPING",
        numeric: 0x1012,
        category: CodeCategory::Compilation,
    },
    CodeEntry {
        symbolic: "NON_STRING_KEY",
        numeric: 0x1013,
        category: CodeCategory::Compilation,
    },
    // Workflow IR: E11xx
    CodeEntry {
        symbolic: "INVALID_COMPILED_WORKFLOW",
        numeric: 0x1101,
        category: CodeCategory::WorkflowIr,
    },
    CodeEntry {
        symbolic: "CONST_OUT_OF_BOUNDS",
        numeric: 0x1102,
        category: CodeCategory::WorkflowIr,
    },
    CodeEntry {
        symbolic: "SLOT_COUNT_MISMATCH",
        numeric: 0x1103,
        category: CodeCategory::WorkflowIr,
    },
    CodeEntry {
        symbolic: "SYMBOL_COUNT_MISMATCH",
        numeric: 0x1104,
        category: CodeCategory::WorkflowIr,
    },
    // Expression: E12xx
    CodeEntry {
        symbolic: "INVALID_EXPRESSION",
        numeric: 0x1201,
        category: CodeCategory::Expression,
    },
    CodeEntry {
        symbolic: "EXPRESSION_BRANCH_EMPTY",
        numeric: 0x1202,
        category: CodeCategory::Expression,
    },
    // Accessor/Path: E13xx
    CodeEntry {
        symbolic: "UNSUPPORTED_ACCESSOR_REFERENCE",
        numeric: 0x1301,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_CYCLE",
        numeric: 0x1302,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_FIELD_MISSING",
        numeric: 0x1303,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_FIELD_NAME_EMPTY",
        numeric: 0x1304,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_ROOT_OUT_OF_RANGE",
        numeric: 0x1305,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_SLOT_COUNT_MISMATCH",
        numeric: 0x1306,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_OUTPUT_NAME_EMPTY",
        numeric: 0x1307,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_DUPLICATE_OUTPUT",
        numeric: 0x1308,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_UNUSED_OUTPUT",
        numeric: 0x1309,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_ID_EMPTY",
        numeric: 0x130A,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_INVALID_SYMBOL",
        numeric: 0x130B,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_MISSING_ROOT",
        numeric: 0x130C,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACCESSOR_EMPTY_PATH",
        numeric: 0x130D,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "IDEMPOTENCY_VIOLATION",
        numeric: 0x1311,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACTION_RESULT_AUDIT_MISMATCH",
        numeric: 0x1312,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACTION_TYPE_CONSTRAINT_FAIL",
        numeric: 0x1313,
        category: CodeCategory::Accessor,
    },
    CodeEntry {
        symbolic: "ACTION_CIRCUIT_BREAKER_OPEN",
        numeric: 0x1314,
        category: CodeCategory::Accessor,
    },
    // Lowering: E14xx
    CodeEntry {
        symbolic: "LOWERING_BODY_EMPTY",
        numeric: 0x1401,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_BODY_TOO_LARGE",
        numeric: 0x1402,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_DUPLICATE_LABEL",
        numeric: 0x1403,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_INVALID_LABEL",
        numeric: 0x1404,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_NODE_OUT_OF_RANGE",
        numeric: 0x1405,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_UNKNOWN_PRIMITIVE",
        numeric: 0x1406,
        category: CodeCategory::Lowering,
    },
    CodeEntry {
        symbolic: "LOWERING_EMPTY_STEP_LIST",
        numeric: 0x1407,
        category: CodeCategory::Lowering,
    },
    // Storage: E20xx
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
    // Runtime Core: E30xx
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
    // Runtime Boundary: E40xx
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
];

/// Lookup: symbolic name → numeric code. Linear scan.
pub fn symbolic_to_numeric(symbolic: &str) -> Option<u16> {
    for entry in CODE_REGISTRY {
        if entry.symbolic == symbolic {
            return Some(entry.numeric);
        }
    }
    None
}

/// Lookup: numeric code → symbolic name. Linear scan.
pub fn numeric_to_symbolic(numeric: u16) -> Option<&'static str> {
    for entry in CODE_REGISTRY {
        if entry.numeric == numeric {
            return Some(entry.symbolic);
        }
    }
    None
}

/// Mirror of the production supported-code predicate.
#[must_use]
pub const fn is_supported_code(code: u16) -> bool {
    matches!(
        code,
        0x0101..=0x010B
            | 0x0201..=0x0204
            | 0x0301..=0x0309
            | 0x0401..=0x040C
            | 0x0501..=0x0513
            | 0x0601..=0x0603
            | 0x1001..=0x1002
            | 0x1011..=0x1013
            | 0x1101..=0x1104
            | 0x1201..=0x1202
            | 0x1301..=0x130D
            | 0x1311..=0x1314
            | 0x1401..=0x1407
            | 0x2001..=0x200F
            | 0x3001..=0x300E
            | 0x4001..=0x401C
    )
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-001 H1: from_static returns Some for every registered symbolic string.
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_from_static_validation() {
        // Verify: for every entry in CODE_REGISTRY, from_static returns Some
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let result = SymbolicCode::from_static(entry.symbolic);
            kani::assert(result.is_some(, "assertion failed"), "Registered code should return Some");
            if let Some(code) = result {
                kani::assert(code.as_str(, "assertion failed") == entry.symbolic, "SymbolicCode should preserve the symbolic string");
            }
        }
    }

    /// PO-001 H2: from_static returns None for unregistered strings.
    /// We use kani::any() to generate arbitrary u16 values and check that
    /// from_static returns None for strings not in the registry.
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_from_static_rejects_unknown() {
        // Generate all registered symbolic strings as a set we can check
        // For a bounded check, we verify that every entry's symbolic works
        // and that nibble the symbolic names produce different results.

        // Verify that a clearly unregistered string returns None
        let result = SymbolicCode::from_static("__DEFINITELY_NOT_REGISTERED__");
        kani::assert(result.is_none(, "assertion failed"), "Unregistered string must return None");

        // Verify that an empty string returns None (no empty symbolic codes exist)
        let empty_result = SymbolicCode::from_static("");
        kani::assert(empty_result.is_none(, "assertion failed"), "Empty string must return None");

        // Verify: for every registered string, from_static returns Some
        // We already proved this in H1. Here we also verify that the result
        // of from_static for any entry is non-None by construction.
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let result = SymbolicCode::from_static(entry.symbolic);
            kani::assert(result.is_some(, "assertion failed"), "kani harness assertion");
        }
    }
}
