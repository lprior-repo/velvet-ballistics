//! Code entries for the [`CodeCategory::Accessor`] category (E13xx, 0x1301–0x1315).

/// Per-category `CodeEntry` slice for [`CodeCategory::Accessor`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "ACCESSOR_CONST_OUT_OF_BOUNDS",
        numeric: 0x1315,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CORE_QUEUE_FULL",
        numeric: 0x1301,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RESOURCE_LIMIT_EXCEEDED",
        numeric: 0x1302,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "ALLOCATION_FAILED",
        numeric: 0x1303,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "EXPRESSION_STACK_OVERFLOW",
        numeric: 0x1304,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "MISSING_OUTPUT_SLOT",
        numeric: 0x1305,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STEP_STATE_OUT_OF_BOUNDS",
        numeric: 0x1306,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_COMPILED_WORKFLOW_CORE",
        numeric: 0x1307,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNSUPPORTED_PRIMITIVE",
        numeric: 0x1308,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNSUPPORTED_ACCESSOR_TRAVERSAL",
        numeric: 0x130A,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "EXPRESSION_STACK_UNDERFLOW",
        numeric: 0x130B,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "OBJECT_FIELD_NOT_FOUND",
        numeric: 0x130C,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LIST_INDEX_OUT_OF_BOUNDS",
        numeric: 0x130D,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SYMBOL_OUT_OF_BOUNDS",
        numeric: 0x1311,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LIST_OUT_OF_BOUNDS",
        numeric: 0x1312,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "OBJECT_OUT_OF_BOUNDS",
        numeric: 0x1313,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "BLOB_OUT_OF_BOUNDS",
        numeric: 0x1314,
        category: super::CodeCategory::Accessor,
        deprecated: false,
    },
];
