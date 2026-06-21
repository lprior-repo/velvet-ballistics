//! Code entries for the [`CodeCategory::RuntimeBoundary`] category —
//! runtime boundary failure codes (E40xx, 0x4031–0x4036).

/// Per-category `CodeEntry` slice for the RuntimeBoundary failure codes.
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "ASK_TIMEOUT",
        numeric: 0x4031,
        category: super::CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "WAIT_TIMEOUT",
        numeric: 0x4032,
        category: super::CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "COLLECT_PAGE_FAILED",
        numeric: 0x4033,
        category: super::CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "REDUCE_ITEM_FAILED",
        numeric: 0x4034,
        category: super::CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "TOGETHER_BRANCH_FAILED",
        numeric: 0x4035,
        category: super::CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "FOR_EACH_ITEM_FAILED",
        numeric: 0x4036,
        category: super::CodeCategory::RuntimeBoundary,
        deprecated: false,
    },
];
