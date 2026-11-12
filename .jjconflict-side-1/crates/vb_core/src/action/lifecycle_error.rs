use super::error::ActionFailure;
use super::ticket::ActionTicket;
use crate::ids::{ActionId, StepIdx};
use core::fmt;

/// Terminal action failure with the engine context that observed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionFailureReport {
    /// Step whose action failed.
    pub step: StepIdx,
    /// Action that failed.
    pub action: ActionId,
    /// Failure details reported by the action lifecycle.
    pub failure: ActionFailure,
}

/// Rejected action resume with the rejected ticket for diagnostics and replay triage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionResumeReport {
    /// Rejection reason.
    pub rejection: ActionResumeRejection,
    /// Ticket that was rejected before frame mutation.
    pub ticket: ActionTicket,
}

impl ActionResumeReport {
    /// Creates a rejected resume report.
    #[must_use]
    pub const fn new(rejection: ActionResumeRejection, ticket: ActionTicket) -> Self {
        Self { rejection, ticket }
    }
}

impl fmt::Display for ActionResumeReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} for ticket {:?}", self.rejection, self.ticket)
    }
}

impl ActionFailureReport {
    /// Creates an action failure report from checked lifecycle context.
    #[must_use]
    pub const fn new(step: StepIdx, action: ActionId, failure: ActionFailure) -> Self {
        Self {
            step,
            action,
            failure,
        }
    }
}

impl fmt::Display for ActionFailureReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "action {:?} failed at step {:?}: {:?}",
            self.action, self.step, self.failure.code
        )
    }
}

/// Reason an externally supplied action resume was rejected before mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionResumeRejection {
    /// Ticket belongs to a different run.
    RunMismatch,
    /// Ticket step is not the current program counter.
    StepNotCurrentPc,
    /// Ticket step is not running.
    StepNotRunning,
    /// Ticket action does not match the Do node action.
    ActionMismatch,
    /// Ticket step does not resolve to a Do node.
    NonDoNode,
    /// Completion output slot does not match the Do node output.
    OutputMismatch,
    /// Ticket attempt is zero.
    AttemptZero,
    /// Ticket capacity is zero.
    CapacityZero,
    /// Ticket attempt exceeds ticket capacity.
    AttemptExceedsCapacity,
    /// Ticket idempotency key does not match its deterministic ingredients.
    IdempotencyKeyMismatch,
    /// Encoded payload length does not match the reported encoded length.
    EncodedPayloadLenMismatch,
    /// Encoded payload length exceeds the action contract bound.
    EncodedPayloadTooLarge,
    /// Action contract does not match the ticket action.
    ContractMismatch,
    /// Action contract declares no output for a Do-node output target.
    ContractOutputUndeclared,
}

impl ActionResumeRejection {
    /// Stable reason string used by diagnostics and logs.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::RunMismatch => "action_resume_run_mismatch",
            Self::StepNotCurrentPc => "action_resume_step_not_current_pc",
            Self::StepNotRunning => "action_resume_step_not_running",
            Self::ActionMismatch => "action_resume_action_mismatch",
            Self::NonDoNode => "action_resume_non_do_node",
            Self::OutputMismatch => "action_resume_output_mismatch",
            Self::AttemptZero => "action_resume_attempt_zero",
            Self::CapacityZero => "action_resume_capacity_zero",
            Self::AttemptExceedsCapacity => "action_resume_attempt_exceeds_capacity",
            Self::IdempotencyKeyMismatch => "action_resume_idempotency_key_mismatch",
            Self::EncodedPayloadLenMismatch => "action_resume_encoded_payload_len_mismatch",
            Self::EncodedPayloadTooLarge => "action_resume_encoded_payload_too_large",
            Self::ContractMismatch => "action_resume_contract_mismatch",
            Self::ContractOutputUndeclared => "action_resume_contract_output_undeclared",
        }
    }
}

impl fmt::Display for ActionResumeRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}
