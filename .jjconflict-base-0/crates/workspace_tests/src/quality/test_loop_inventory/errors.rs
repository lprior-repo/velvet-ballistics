use super::{DispositionKind, Location};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryError {
    WorkspaceUnreadable {
        root: String,
    },
    InputRootOutOfScope {
        path: String,
    },
    FileReadFailed {
        path: String,
        operation: String,
    },
    InvalidUtf8 {
        path: String,
        byte_offset: usize,
    },
    ParseFailed {
        path: String,
        location: Location,
    },
    AmbiguousCaseLabel {
        label: String,
        behavior: Option<String>,
        case_count: usize,
    },
    UnassignedRiskyPattern {
        finding_id: String,
    },
    ConflictingDisposition {
        finding_id: String,
        dispositions: Vec<DispositionKind>,
    },
    DestructiveChangeDetected {
        path: String,
        previous_finding: String,
    },
    UnsupportedGeneratedSource {
        path_or_macro: String,
        reason: String,
    },
    PolicyViolation {
        rule: String,
        field: String,
    },
}
