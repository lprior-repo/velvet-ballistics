//! Code entries for the [`CodeCategory::Internal`] category — fallback
//! codes used by `HasSymbolicCode` implementations.

/// Per-category `CodeEntry` slice for [`CodeCategory::Internal`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "INTERNAL_INVARIANT_VIOLATION",
        numeric: 0x1309,
        category: super::CodeCategory::Internal,
        deprecated: false,
    },
];
