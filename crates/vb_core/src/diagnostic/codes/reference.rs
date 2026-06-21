//! Code entries for the [`CodeCategory::Reference`] category (E02xx, 0x0201–0x0209).

/// Per-category `CodeEntry` slice for [`CodeCategory::Reference`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "UNKNOWN_REFERENCE",
        numeric: 0x0201,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "FUTURE_REFERENCE",
        numeric: 0x0202,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SECRET_NOT_DECLARED",
        numeric: 0x0203,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "DIRECT_RUNTIME_REFERENCE",
        numeric: 0x0204,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SCOPE_GUARD_VIOLATION",
        numeric: 0x0205,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "DIRECT_LOOP_REFERENCE",
        numeric: 0x0206,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "DIRECT_STEP_REFERENCE",
        numeric: 0x0207,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STEP_SKIPPED_REFERENCE",
        numeric: 0x0208,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "RESULT_REFERENCE_MISSING",
        numeric: 0x0209,
        category: super::CodeCategory::Reference,
        deprecated: false,
    },
];
