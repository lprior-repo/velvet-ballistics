#![forbid(unsafe_code)]
//! Duplicate retry, resolution commit, and recovery outcome decisions.

pub(super) mod duplicate;
pub(super) mod recovery;
pub(super) mod resolution;

// Re-export kernel types for direct access
pub use super::mrwe6_kernel::{
    Mrwe6DuplicateRetryDecision, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision,
};

// Re-export duplicate module
pub use duplicate::{
    mrwe6_duplicate_retry_decision, mrwe6_duplicate_retry_decision_from_facts,
    mrwe6_idempotent_duplicate_retry_from_facts,
};

// Re-export resolution module
pub use resolution::{
    mrwe6_committed_resolution_from_facts, mrwe6_resolution_commit_decision,
    mrwe6_resolution_commit_decision_from_facts,
};

// Re-export recovery module
pub use recovery::{
    mrwe6_pending_inventory_from_facts, mrwe6_recovery_outcome, mrwe6_recovery_outcome_from_facts,
};
