//! Code entries for the [`CodeCategory::ControlFlow`] category (E03xx, 0x0301–0x0309).

/// Per-category `CodeEntry` slice for [`CodeCategory::ControlFlow`].
pub(super) const ENTRIES: &[super::CodeEntry] = &[
    super::CodeEntry {
        symbolic: "INVALID_THEN_TARGET",
        numeric: 0x0301,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "CONTROL_FLOW_CYCLE",
        numeric: 0x0302,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "UNREACHABLE_STEP",
        numeric: 0x0303,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_CHOOSE",
        numeric: 0x0304,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_FOR_EACH",
        numeric: 0x0305,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_TOGETHER",
        numeric: 0x0306,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_COLLECT",
        numeric: 0x0307,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_REDUCE",
        numeric: 0x0308,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
    super::CodeEntry {
        symbolic: "INVALID_REPEAT",
        numeric: 0x0309,
        category: super::CodeCategory::ControlFlow,
        deprecated: false,
    },
];
