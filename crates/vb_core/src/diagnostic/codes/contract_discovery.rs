//! Code entries for the [`CodeCategory::ContractDiscovery`] category (E06xx, 0x0601–0x0603).

/// Per-category `CodeEntry` slice for [`CodeCategory::ContractDiscovery`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "MISSING_SCHEMA_VERSION",
        numeric: 0x0601,
        category: super::CodeCategory::ContractDiscovery,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CUE_VET_FAILED",
        numeric: 0x0602,
        category: super::CodeCategory::ContractDiscovery,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "VERSION_MONOTONICITY_BREACH",
        numeric: 0x0603,
        category: super::CodeCategory::ContractDiscovery,
        deprecated: false,
    },
];
