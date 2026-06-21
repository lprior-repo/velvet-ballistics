//! Code entries for the [`CodeCategory::WorkflowIr`] category (E11xx, 0x1101–0x1105).

/// Per-category `CodeEntry` slice for [`CodeCategory::WorkflowIr`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "CORE_TYPE_MISMATCH",
        numeric: 0x1101,
        category: super::CodeCategory::WorkflowIr,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "NON_FINITE_NUMBER",
        numeric: 0x1102,
        category: super::CodeCategory::WorkflowIr,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "DIVISION_BY_ZERO",
        numeric: 0x1103,
        category: super::CodeCategory::WorkflowIr,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "NON_BOOL_CONDITION",
        numeric: 0x1104,
        category: super::CodeCategory::WorkflowIr,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_COMPILED_WORKFLOW",
        numeric: 0x1105,
        category: super::CodeCategory::WorkflowIr,
        deprecated: false,
    },
];
