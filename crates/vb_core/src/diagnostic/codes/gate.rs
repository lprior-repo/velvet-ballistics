//! Code entries for the [`CodeCategory::Gate`] category (E05xx, 0x0501–0x0513).

/// Per-category `CodeEntry` slice for [`CodeCategory::Gate`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "EXPRESSION_STACK_EXCEEDED",
        numeric: 0x0501,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "EXPRESSION_STACK_MISMATCH",
        numeric: 0x0502,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACCESSOR_SLOT_OUT_OF_RANGE",
        numeric: 0x0503,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACCESSOR_PATH_INVALID",
        numeric: 0x0504,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SLOT_REFERENCE_OUT_OF_RANGE",
        numeric: 0x0505,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LOOP_BODY_STEP_OUT_OF_RANGE",
        numeric: 0x0506,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SLOT_DEPENDENCY_CYCLE",
        numeric: 0x0507,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "NODE_KIND_CONSTRAINT_VIOLATION",
        numeric: 0x0508,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACTION_CONTRACT_MISSING",
        numeric: 0x0509,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACTION_CONTRACT_ORPHAN",
        numeric: 0x050A,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SLOT_TYPE_INCONSISTENCY",
        numeric: 0x050B,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "NON_DETERMINISTIC_PATH",
        numeric: 0x050C,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CAPABILITY_NAME_EMPTY",
        numeric: 0x050D,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CAPABILITY_NAME_TOO_LONG",
        numeric: 0x050E,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CAPABILITY_NAME_INVALID",
        numeric: 0x050F,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CAPABILITY_ACTION_MISMATCH",
        numeric: 0x0510,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CAPABILITY_DUPLICATE",
        numeric: 0x0511,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACCESSOR_PATH_TOO_DEEP",
        numeric: 0x0512,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ACCESSOR_SYMBOL_OUT_OF_BOUNDS",
        numeric: 0x0513,
        category: super::CodeCategory::Gate,
        deprecated: false,
    },
];
