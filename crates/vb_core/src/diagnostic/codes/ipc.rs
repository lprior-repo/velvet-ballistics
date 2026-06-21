//! Code entries for the [`CodeCategory::Ipc`] category (E32xx, 0x3201–0x320A).

/// Per-category `CodeEntry` slice for [`CodeCategory::Ipc`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "IPC_PAYLOAD_TOO_LARGE",
        numeric: 0x3201,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_DECODE_FAILED",
        numeric: 0x3202,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_ENCODE_FAILED",
        numeric: 0x3203,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_CHANNEL_CLOSED",
        numeric: 0x3204,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_CHANNEL_FULL",
        numeric: 0x3205,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_CONNECTION_REFUSED",
        numeric: 0x3206,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_TIMEOUT",
        numeric: 0x3207,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_PROTOCOL_VIOLATION",
        numeric: 0x3208,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_AUTH_FAILED",
        numeric: 0x3209,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "IPC_RESOURCE_UNAVAILABLE",
        numeric: 0x320A,
        category: super::CodeCategory::Ipc,
        deprecated: false,
    },
];
