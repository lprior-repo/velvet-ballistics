//! Code entries for the [`CodeCategory::Runtime`] category (E30xx, 0x300F–0x3022).

/// Per-category `CodeEntry` slice for [`CodeCategory::Runtime`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "RUNTIME_TIMEOUT",
        numeric: 0x300F,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_BUDGET_EXHAUSTED",
        numeric: 0x3010,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_CYCLE_LIMIT",
        numeric: 0x3011,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_ACTION_DISPATCH",
        numeric: 0x3012,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_ACTION_TIMEOUT",
        numeric: 0x3013,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_SIGNAL_INVALID",
        numeric: 0x3014,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_QUEUE_OVERFLOW",
        numeric: 0x3015,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_JOURNAL_BATCH",
        numeric: 0x3016,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_TICK_OVERFLOW",
        numeric: 0x3017,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_STEP_LIMIT",
        numeric: 0x3018,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_TRACE_OVERFLOW",
        numeric: 0x3019,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_CAPACITY_EXCEEDED",
        numeric: 0x301A,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RUNTIME_INVALID_STATE",
        numeric: 0x301B,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACTION_RESULT_AUDIT_MISMATCH",
        numeric: 0x3020,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACTION_TYPE_CONSTRAINT_FAIL",
        numeric: 0x3021,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACTION_CIRCUIT_BREAKER_OPEN",
        numeric: 0x3022,
        category: super::CodeCategory::Runtime,
        deprecated: false,
    },
];
