//! Code entries for the [`CodeCategory::Lowering`] category (E14xx, 0x1401–0x140D).

/// Per-category `CodeEntry` slice for [`CodeCategory::Lowering`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "ITERATION_LIMIT_EXCEEDED",
        numeric: 0x1401,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "REPEAT_EXHAUSTED",
        numeric: 0x1402,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "COLLECT_PAGE_LIMIT_EXCEEDED",
        numeric: 0x1403,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "COLLECT_ITEM_LIMIT_EXCEEDED",
        numeric: 0x1404,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "TOGETHER_BRANCH_LIMIT_EXCEEDED",
        numeric: 0x1405,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "BUDGET_EXCEEDED",
        numeric: 0x1406,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "COLLECT_TIME_LIMIT_EXCEEDED",
        numeric: 0x1407,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "PARALLEL_LIMIT_EXCEEDED",
        numeric: 0x1408,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CAPABILITY_DENIED",
        numeric: 0x1409,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "BUDGET_PARSE",
        numeric: 0x140A,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "COLLECT_PAGE_ORDER_VIOLATION",
        numeric: 0x140B,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "COLLECT_EXTRA_HYDRATION_FAILED",
        numeric: 0x140C,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "COLLECT_EVIDENCE_CAPACITY_EXCEEDED",
        numeric: 0x140D,
        category: super::CodeCategory::Lowering,
        deprecated: false,
    },
];
