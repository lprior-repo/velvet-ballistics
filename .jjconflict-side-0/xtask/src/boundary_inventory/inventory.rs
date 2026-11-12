use super::record::BoundaryRecord;
use super::types::{OptionalDiscoveryEvidence, ReviewSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryInventory {
    pub schema_version: Option<u32>,
    pub records: Vec<BoundaryRecord>,
    pub discovery_trace: OptionalDiscoveryEvidence,
}

impl BoundaryInventory {
    #[must_use]
    pub fn new(
        schema_version: Option<u32>,
        records: Vec<BoundaryRecord>,
        discovery_trace: OptionalDiscoveryEvidence,
    ) -> Self {
        Self {
            schema_version,
            records,
            discovery_trace,
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct ValidatedBoundaryInventory {
    pub schema_version: u32,
    pub records: Vec<BoundaryRecord>,
    pub discovered_boundary_count: usize,
    pub review_status: Option<ReviewSummary>,
}

impl PartialEq for ValidatedBoundaryInventory {
    fn eq(&self, other: &Self) -> bool {
        self.schema_version == other.schema_version
            && review_status_matches(
                self.review_status.as_deref(),
                other.review_status.as_deref(),
            )
            && count_matches(self, other)
    }
}

impl ValidatedBoundaryInventory {
    #[must_use]
    pub fn from_validated_records(
        schema_version: u32,
        records: Vec<BoundaryRecord>,
        review_status: Option<ReviewSummary>,
    ) -> Self {
        let discovered_boundary_count = records.len();
        Self {
            schema_version,
            records,
            discovered_boundary_count,
            review_status,
        }
    }

    #[must_use]
    pub fn with_schema_version(schema_version: u32) -> Self {
        Self {
            schema_version,
            records: Vec::new(),
            discovered_boundary_count: 0,
            review_status: None,
        }
    }

    #[must_use]
    pub fn with_review_status(status: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            records: Vec::new(),
            discovered_boundary_count: 0,
            review_status: Some(status.into()),
        }
    }

    #[must_use]
    pub fn from_records(records: Vec<BoundaryRecord>) -> Self {
        let discovered_boundary_count = records.len();
        Self {
            schema_version: 1,
            records,
            discovered_boundary_count,
            review_status: None,
        }
    }

    #[must_use]
    pub fn empty_with_discovered_boundary_count(discovered_boundary_count: usize) -> Self {
        Self {
            schema_version: 1,
            records: Vec::new(),
            discovered_boundary_count,
            review_status: None,
        }
    }
}

fn review_status_matches(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left_status), Some(right_status)) => left_status == right_status,
        (None, _) | (_, None) => true,
    }
}

fn count_matches(left: &ValidatedBoundaryInventory, right: &ValidatedBoundaryInventory) -> bool {
    if !left.records.is_empty() || !right.records.is_empty() {
        return left.records == right.records;
    }
    left.discovered_boundary_count == right.discovered_boundary_count
}
