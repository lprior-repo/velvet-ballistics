#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvidenceRequirement {
    FuzzOrIsolationOrManualQa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnsafeIsolationStatus {
    Complete { boundary_count: usize },
}
