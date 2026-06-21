//! Code entries for the [`CodeCategory::Expression`] category (E12xx, 0x1201–0x1203).

/// Per-category `CodeEntry` slice for [`CodeCategory::Expression`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "INVALID_EXPRESSION",
        numeric: 0x1203,
        category: super::CodeCategory::Expression,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STEP_BUDGET_EXHAUSTED",
        numeric: 0x1201,
        category: super::CodeCategory::Expression,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STEP_COUNTER_OVERFLOW",
        numeric: 0x1202,
        category: super::CodeCategory::Expression,
        deprecated: false,
    },
];
