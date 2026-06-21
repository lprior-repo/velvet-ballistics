//! Code entries for the [`CodeCategory::Storage`] category — legacy
//! storage infrastructure codes (E20xx, 0x2070–0x207D).

/// Per-category `CodeEntry` slice for the legacy storage infrastructure codes.
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "STORAGE_UNAVAILABLE",
        numeric: 0x2070,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_CORRUPTION",
        numeric: 0x2071,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_IO",
        numeric: 0x2072,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_ENCODING",
        numeric: 0x2073,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_DECODING",
        numeric: 0x2074,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_CHECKPOINT",
        numeric: 0x2075,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_SNAPSHOT",
        numeric: 0x2076,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_PAGE_OVERFLOW",
        numeric: 0x2077,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_KEYSPACE_MANIFEST",
        numeric: 0x2078,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_BLOB_LIMIT",
        numeric: 0x2079,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_WRITE_BUDGET",
        numeric: 0x207A,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_READ_BUDGET",
        numeric: 0x207B,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_COMPACTION_FAILED",
        numeric: 0x207C,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "STORAGE_SEALED",
        numeric: 0x207D,
        category: super::CodeCategory::Storage,
        deprecated: false,
    },
];
