use serde::{Deserialize, Serialize};

/// Risk class of the action behind a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RiskClass {
    /// Low-risk action.
    Low = 0,
    /// Medium-risk action.
    Medium = 1,
    /// High-risk action; separation of duties always applies.
    High = 2,
}

/// Separation-of-duties rule carried by a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SeparationOfDuties {
    /// No separation requirement beyond the risk-class rule.
    Disabled = 0,
    /// Approver must differ from the requester at any risk class.
    DistinctFromRequester = 1,
}

/// Structured decision options (MASTER §77.15). Closed: exactly five.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GateDecision {
    /// Approve the gated action.
    Approve = 0,
    /// Deny the gated action.
    Deny = 1,
    /// Ask for more evidence before deciding.
    RequestMoreEvidence = 2,
    /// Hand the gate to another approver.
    Delegate = 3,
    /// Emergency approval outside the normal role, with justification.
    BreakGlass = 4,
}

/// Durable outcome of a gate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GateOutcome {
    /// Action approved.
    Approved = 0,
    /// Action denied.
    Denied = 1,
    /// More evidence requested; the gate stays open.
    MoreEvidenceRequested = 2,
    /// Gate delegated to another approver.
    Delegated = 3,
    /// Action approved via break-glass.
    BreakGlassApproved = 4,
    /// Gate timed out and escalated under the escalation policy.
    Escalated = 5,
}

impl GateOutcome {
    /// Outcome implied by an operator decision, or `None` for escalations,
    /// which are enforcement outcomes rather than operator decisions.
    #[must_use]
    pub const fn from_decision(decision: GateDecision) -> Self {
        match decision {
            GateDecision::Approve => Self::Approved,
            GateDecision::Deny => Self::Denied,
            GateDecision::RequestMoreEvidence => Self::MoreEvidenceRequested,
            GateDecision::Delegate => Self::Delegated,
            GateDecision::BreakGlass => Self::BreakGlassApproved,
        }
    }
}
