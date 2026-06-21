//! Code entries for the [`CodeCategory::Lifecycle`] category (E33xx, E15xx, 0x3301–0x3304, 0x1501–0x1506).

/// Per-category `CodeEntry` slice for [`CodeCategory::Lifecycle`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "LIFECYCLE_STORAGE_UNAVAILABLE",
        numeric: 0x3301,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LIFECYCLE_DUPLICATE_REQUEST",
        numeric: 0x3302,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LIFECYCLE_INVALID_TRANSITION",
        numeric: 0x3303,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LIFECYCLE_STALE_BEAD",
        numeric: 0x3304,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CORE_LIFECYCLE_STORAGE_UNAVAILABLE",
        numeric: 0x1501,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CORE_LIFECYCLE_DUPLICATE_REQUEST",
        numeric: 0x1502,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LIFECYCLE_STALE_REQUEST",
        numeric: 0x1503,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CORE_LIFECYCLE_INVALID_TRANSITION",
        numeric: 0x1504,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "JOURNAL_WRITE_FAILURE",
        numeric: 0x1505,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "REPLAY_CORRUPTION",
        numeric: 0x1506,
        category: super::CodeCategory::Lifecycle,
        deprecated: false,
    },
];
