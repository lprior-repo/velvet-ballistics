//! Code entries for the [`CodeCategory::Compilation`] category (E10xx, 0x1001–0x1015).

/// Per-category `CodeEntry` slice for [`CodeCategory::Compilation`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "INVALID_PROGRAM_COUNTER",
        numeric: 0x1001,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "MISSING_NEXT_STEP",
        numeric: 0x1002,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNKNOWN_INPUT_SCHEMA_FIELD",
        numeric: 0x1003,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNSUPPORTED_TOP_LEVEL_DECLARATION",
        numeric: 0x1004,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNKNOWN_OUTPUT_NAME",
        numeric: 0x1005,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNSUPPORTED_ACCESSOR_REFERENCE",
        numeric: 0x1006,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SLOT_OUT_OF_BOUNDS",
        numeric: 0x1011,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SLOT_UNINITIALIZED",
        numeric: 0x1012,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CONST_OUT_OF_BOUNDS",
        numeric: 0x1013,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IDEMPOTENCY_VIOLATION",
        numeric: 0x1014,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "EXPR_OUT_OF_BOUNDS",
        numeric: 0x1015,
        category: super::CodeCategory::Compilation,
        deprecated: false,
    },
];
