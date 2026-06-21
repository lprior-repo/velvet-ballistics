//! Code entries for the [`CodeCategory::TypeTaint`] category (E04xx, 0x0401–0x040C).

/// Per-category `CodeEntry` slice for [`CodeCategory::TypeTaint`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "INVALID_WAIT",
        numeric: 0x0401,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_ASK",
        numeric: 0x0402,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_FINISH",
        numeric: 0x0403,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_RETRY",
        numeric: 0x0404,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_ON_ERROR",
        numeric: 0x0405,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "SECRET_RESULT_LEAK",
        numeric: 0x0406,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "TYPE_MISMATCH",
        numeric: 0x0407,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "PAYLOAD_TOO_LARGE",
        numeric: 0x0408,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LIMIT_REQUIRED",
        numeric: 0x0409,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "LIMIT_EXCEEDED",
        numeric: 0x040A,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNSUPPORTED_TRIGGER",
        numeric: 0x040B,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "HTTP_TRIGGER_OUT_OF_CORE",
        numeric: 0x040C,
        category: super::CodeCategory::TypeTaint,
        deprecated: false,
    },
];
