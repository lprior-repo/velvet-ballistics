//! Code entries for the [`CodeCategory::Schema`] category (E01xx, 0x0101–0x010B).

/// Per-category `CodeEntry` slice for [`CodeCategory::Schema`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "DUPLICATE_KEY",
        numeric: 0x0101,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "FORBIDDEN_YAML_FEATURE",
        numeric: 0x0102,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNKNOWN_TOP_LEVEL_FIELD",
        numeric: 0x0103,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNKNOWN_STEP_FIELD",
        numeric: 0x0104,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "MISSING_REQUIRED_FIELD",
        numeric: 0x0105,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_VERSION",
        numeric: 0x0106,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_ID",
        numeric: 0x0107,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RESERVED_ID",
        numeric: 0x0108,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "DUPLICATE_ID",
        numeric: 0x0109,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "MULTIPLE_STEP_PRIMITIVES",
        numeric: 0x010A,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "MISSING_STEP_PRIMITIVE",
        numeric: 0x010B,
        category: super::CodeCategory::Schema,
        deprecated: false,
    },
];
