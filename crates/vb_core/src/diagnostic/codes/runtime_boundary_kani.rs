// Code entries for the [`CodeCategory::RuntimeBoundary`] category —
// additional journal/internal codes used by Kani models (E40xx, 0x4021–0x402E).
// // This file is a data file intended for inclusion via the `include!` macro.

CodeEntry {
    symbolic: "JOURNAL_SEQ_MISMATCH",
    numeric: 0x4021,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_CHECKPOINT_MISMATCH",
    numeric: 0x4022,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_PAGE_ORDER_VIOLATION",
    numeric: 0x4023,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_EXTRA_HYDRATION_FAIL",
    numeric: 0x4024,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_EVIDENCE_OVERFLOW",
    numeric: 0x4025,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_SLOT_NOT_WRITABLE",
    numeric: 0x4026,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_DUPLICATE_ACTION",
    numeric: 0x4027,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_UNKNOWN_ACTION",
    numeric: 0x4028,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_STALE_EVENT",
    numeric: 0x4029,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_EVENT_ORDER",
    numeric: 0x402A,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_BATCH_OVERFLOW",
    numeric: 0x402B,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_CLOCK_DRIFT",
    numeric: 0x402C,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_BUFFER_OVERFLOW",
    numeric: 0x402D,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
CodeEntry {
    symbolic: "JOURNAL_SLOT_SEALED",
    numeric: 0x402E,
    category: CodeCategory::RuntimeBoundary,
    deprecated: false,
},
