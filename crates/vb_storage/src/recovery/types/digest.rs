#![forbid(unsafe_code)]
//! Digest check levels and configuration for recovery validation.

use vb_core::{ActionId, StepIdx, WorkflowDigest};

/// Digest check level for recovery validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DigestCheck {
    /// Only verify workflow source digest.
    WorkflowSourceOnly,
    /// Verify workflow source and compiled IR digests.
    WorkflowAndIr,
    /// Verify all digests including action ABI and policy.
    Full,
}

impl DigestCheck {
    /// Numeric rank for proof and testing of the strict digest hierarchy.
    #[must_use]
    pub const fn hierarchy_rank(self) -> u8 {
        match self {
            Self::WorkflowSourceOnly => 1,
            Self::WorkflowAndIr => 2,
            Self::Full => 3,
        }
    }

    /// Whether this level requires workflow-source digest verification.
    #[must_use]
    pub const fn checks_workflow_source(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()
    }

    /// Whether this level requires compiled-IR digest verification.
    #[must_use]
    pub const fn checks_compiled_ir(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()
    }

    /// Whether this level requires all currently-modeled digest checks.
    #[must_use]
    pub const fn checks_full(self) -> bool {
        self.hierarchy_rank() >= Self::Full.hierarchy_rank()
    }

    /// Production proof surface for strict ordering between two levels.
    #[must_use]
    pub const fn is_strictly_weaker_than(self, other: Self) -> bool {
        self.hierarchy_rank() < other.hierarchy_rank()
    }
}

/// Configuration for Full-level digest verification.
///
/// Contains the expected action ABI and policy digests that must match
/// during replay validation at Full strictness.
#[derive(Debug, Clone, Copy)]
pub struct DigestCheckConfig<'a> {
    pub action_abi_entries: Option<&'a [(ActionId, WorkflowDigest, WorkflowDigest)]>,
    pub policy_entries: Option<&'a [(StepIdx, WorkflowDigest, WorkflowDigest)]>,
}
