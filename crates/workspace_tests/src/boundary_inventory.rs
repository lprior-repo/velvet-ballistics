mod api;
mod inventory;
mod parser;
mod record;
mod status;
mod types;
mod validation;

pub use api::{
    classify_boundary, discover_boundaries, inventory_completion_status, required_evidence,
    validate_inventory,
};
pub use inventory::{BoundaryInventory, ValidatedBoundaryInventory};
pub use parser::parse_inventory;
pub use record::{
    BoundaryRecord, BoundaryRecordDraft, BoundaryRecordParts, CompleteBoundaryRecord, FieldState,
    Owner, ReviewDecision, ThreatStatement, ValidatedBoundaryRecord,
};
pub use status::{EvidenceRequirement, UnsafeIsolationStatus};
pub use types::{
    BoundaryCandidate, BoundaryClass, BoundaryExposure, BoundaryInventoryError, BoundaryRisk,
    ClassifiedBoundary, ClassifiedBoundaryInput, DiscoveryEvidence, EvidenceKind,
    EvidenceReference, FreshnessMarker, OptionalDiscoveryEvidence, ReviewStatus,
    ReviewSummary, WorkspaceRoot,
};
pub use validation::validate_evidence_reference_bytes;