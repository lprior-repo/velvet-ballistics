/// Typed manual-gate evaluation errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GateError {
    /// Submitted gate identifier does not match the gate.
    #[error("gate id mismatch: submitted {submitted} does not match gate {expected}")]
    GateIdMismatch {
        /// Gate identifier from the gate definition.
        expected: u64,
        /// Gate identifier from the submission.
        submitted: u64,
    },
    /// Approver role does not match the gate's required role.
    #[error("approver role mismatch: required {required} but approver holds {actual}")]
    WrongRole {
        /// Required role name.
        required: String,
        /// Approver's claimed role name.
        actual: String,
    },
    /// Evidence bundle shown to the approver is not the gate's bundle.
    #[error("evidence bundle digest does not match the gate evidence bundle")]
    EvidenceDigestMismatch,
    /// High-risk, non-compensatable approval without evidence display.
    #[error("evidence must be displayed before approving a high-risk non-compensatable action")]
    EvidenceNotShown,
    /// Requester and approver are the same operator where separation
    /// of duties applies.
    #[error("separation of duties violated: requester and approver must differ")]
    SeparationOfDutiesViolation,
    /// Approval without a reason where the gate requires one.
    #[error("approval reason is required by this gate")]
    ReasonRequired,
    /// Break-glass decision on a gate that does not allow it.
    #[error("break-glass is not allowed by this gate")]
    BreakGlassNotAllowed,
    /// Break-glass decision without a justification.
    #[error("break-glass justification is required")]
    BreakGlassJustificationRequired,
    /// Delegation without a target.
    #[error("delegation requires a target approver")]
    DelegateTargetMissing,
    /// Delegation target is the delegating approver.
    #[error("delegation target must differ from the delegating approver")]
    DelegateTargetIsApprover,
    /// Decision submitted after the gate deadline.
    #[error("decision at {decided_at_ms} is after the gate deadline {deadline_ms}")]
    DecisionAfterTimeout {
        /// Gate deadline in milliseconds.
        deadline_ms: u64,
        /// Submitted decision time in milliseconds.
        decided_at_ms: u64,
    },
    /// Timeout evaluation before the deadline.
    #[error("gate has not timed out yet")]
    NotTimedOut,
    /// Deadline arithmetic overflowed.
    #[error("gate deadline arithmetic overflowed")]
    DeadlineOverflow,
    /// Approver identity is empty or over-long.
    #[error("approver identity is invalid")]
    InvalidApprover,
    /// Requester identity is empty or over-long.
    #[error("requester identity is invalid")]
    InvalidRequester,
    /// Approver role is empty or over-long.
    #[error("approver role is invalid")]
    InvalidRole,
    /// Receipt digest encoding failed (approver length does not fit u64).
    #[error("receipt digest encoding failed")]
    ReceiptDigestEncoding,
    /// Record fails structural validation.
    #[error("gate outcome record is structurally invalid")]
    InvalidRecord,
    /// Record does not match its gate.
    #[error("gate outcome record does not match its gate")]
    RecordGateMismatch,
    /// Replay found a receipt on a non-approval outcome.
    #[error("replay record carries a receipt without an approval outcome")]
    ReplayReceiptMismatch,
    /// Replay found a gate opened twice without a closing decision.
    #[error("replay opened gate {gate} twice without a closing decision")]
    ReplayDuplicateOpen {
        /// Gate identifier opened twice.
        gate: u64,
    },
    /// Replay found a gate-opened event carrying a structurally invalid gate.
    #[error("replay gate-opened event carries a structurally invalid gate")]
    ReplayInvalidGate,
}
