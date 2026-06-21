// Code entries for the [`CodeCategory::RuntimeBoundary`] category —
// runtime boundary failure codes (E40xx, 0x4031–0x4036).
// // This file is a data file intended for inclusion via the `include!` macro.

CodeEntry {
    symbolic: "ASK_TIMEOUT",
    numeric: 0x4031,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "WAIT_TIMEOUT",
    numeric: 0x4032,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "COLLECT_PAGE_FAILED",
    numeric: 0x4033,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "REDUCE_ITEM_FAILED",
    numeric: 0x4034,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "TOGETHER_BRANCH_FAILED",
    numeric: 0x4035,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "FOR_EACH_ITEM_FAILED",
    numeric: 0x4036,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
