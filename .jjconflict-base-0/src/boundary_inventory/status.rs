#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceRequirement {
    FuzzOrIsolationOrManualQa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsafeIsolationStatus {
    Complete { boundary_count: usize },
}
