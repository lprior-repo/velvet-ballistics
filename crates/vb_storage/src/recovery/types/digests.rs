#![forbid(unsafe_code)]

use vb_core::{ActionId, StepIdx, WorkflowDigest};

/// Expected/found digest pair for a recovery verification subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigestPair {
    /// Expected digest from the trusted manifest or caller contract.
    pub expected: WorkflowDigest,
    /// Observed digest recovered from durable evidence.
    pub found: WorkflowDigest,
}

impl DigestPair {
    /// Builds a digest comparison pair.
    #[must_use]
    pub const fn new(expected: WorkflowDigest, found: WorkflowDigest) -> Self {
        Self { expected, found }
    }
}

/// Action ABI digest comparison tied to the action identity it protects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionAbiDigestComparison {
    /// Action whose ABI digest is checked.
    pub action_id: ActionId,
    /// Expected/found digest pair.
    pub digest: DigestPair,
}

impl ActionAbiDigestComparison {
    /// Builds an action ABI digest comparison.
    #[must_use]
    pub const fn new(action_id: ActionId, expected: WorkflowDigest, found: WorkflowDigest) -> Self {
        Self {
            action_id,
            digest: DigestPair::new(expected, found),
        }
    }
}

/// Policy digest comparison tied to the step whose policy is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyDigestComparison {
    /// Step whose policy digest is checked.
    pub step: StepIdx,
    /// Expected/found digest pair.
    pub digest: DigestPair,
}

impl PolicyDigestComparison {
    /// Builds a policy digest comparison.
    #[must_use]
    pub const fn new(step: StepIdx, expected: WorkflowDigest, found: WorkflowDigest) -> Self {
        Self {
            step,
            digest: DigestPair::new(expected, found),
        }
    }
}

/// Explicit evidence required by full digest verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullDigestEvidence<'a> {
    action_abi: &'a [ActionAbiDigestComparison],
    policy: &'a [PolicyDigestComparison],
}

impl<'a> FullDigestEvidence<'a> {
    /// Builds full digest evidence from typed subject comparisons.
    #[must_use]
    pub const fn new(
        action_abi: &'a [ActionAbiDigestComparison],
        policy: &'a [PolicyDigestComparison],
    ) -> Self {
        Self { action_abi, policy }
    }

    /// Explicit evidence that the manifest has no action/policy subjects.
    #[must_use]
    pub const fn no_contracts() -> Self {
        Self {
            action_abi: &[],
            policy: &[],
        }
    }

    /// Evidence for action ABI subjects only.
    #[must_use]
    pub const fn action_abi_only(action_abi: &'a [ActionAbiDigestComparison]) -> Self {
        Self {
            action_abi,
            policy: &[],
        }
    }

    /// Evidence for policy subjects only.
    #[must_use]
    pub const fn policy_only(policy: &'a [PolicyDigestComparison]) -> Self {
        Self {
            action_abi: &[],
            policy,
        }
    }

    /// Action ABI comparisons carried by this evidence.
    #[must_use]
    pub const fn action_abi(self) -> &'a [ActionAbiDigestComparison] {
        self.action_abi
    }

    /// Policy comparisons carried by this evidence.
    #[must_use]
    pub const fn policy(self) -> &'a [PolicyDigestComparison] {
        self.policy
    }
}

/// Type-coupled digest verification request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestVerificationRequest<'a> {
    /// Verify only the workflow source digest from durable RunAccepted evidence.
    WorkflowSourceOnly {
        /// Expected workflow/source digest.
        expected_workflow_digest: WorkflowDigest,
    },
    /// Verify workflow/source digest and compiled IR digest.
    WorkflowAndIr {
        /// Expected workflow/source digest.
        expected_workflow_digest: WorkflowDigest,
        /// Expected compiled IR digest.
        expected_ir_digest: WorkflowDigest,
        /// Observed compiled IR digest.
        found_ir_digest: WorkflowDigest,
    },
    /// Verify workflow/source, compiled IR, action ABI, and policy digests.
    Full {
        /// Expected workflow/source digest.
        expected_workflow_digest: WorkflowDigest,
        /// Expected compiled IR digest.
        expected_ir_digest: WorkflowDigest,
        /// Observed compiled IR digest.
        found_ir_digest: WorkflowDigest,
        /// Typed full-verification evidence.
        evidence: FullDigestEvidence<'a>,
    },
}

impl<'a> DigestVerificationRequest<'a> {
    /// Builds a workflow-source-only verification request.
    #[must_use]
    pub const fn workflow_source_only(expected_workflow_digest: WorkflowDigest) -> Self {
        Self::WorkflowSourceOnly {
            expected_workflow_digest,
        }
    }

    /// Builds a workflow plus compiled-IR verification request.
    #[must_use]
    pub const fn workflow_and_ir(
        expected_workflow_digest: WorkflowDigest,
        expected_ir_digest: WorkflowDigest,
        found_ir_digest: WorkflowDigest,
    ) -> Self {
        Self::WorkflowAndIr {
            expected_workflow_digest,
            expected_ir_digest,
            found_ir_digest,
        }
    }

    /// Builds a full verification request with typed action/policy evidence.
    #[must_use]
    pub const fn full(
        expected_workflow_digest: WorkflowDigest,
        expected_ir_digest: WorkflowDigest,
        found_ir_digest: WorkflowDigest,
        evidence: FullDigestEvidence<'a>,
    ) -> Self {
        Self::Full {
            expected_workflow_digest,
            expected_ir_digest,
            found_ir_digest,
            evidence,
        }
    }
}
