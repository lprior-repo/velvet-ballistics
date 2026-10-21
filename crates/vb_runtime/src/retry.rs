#![forbid(unsafe_code)]
//! Typed retry-attempt planning from ordered durable run evidence.

use vb_core::ids::{RunId, StepIdx};
use vb_storage::JournalEvent;

/// A validated transition from a failed attempt to its next attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryAttemptPlan {
    run: RunId,
    step: StepIdx,
    failed_attempt: u16,
    next_attempt: u16,
}

impl RetryAttemptPlan {
    /// Derives the next attempt from the journal prefix for the observed failed step.
    ///
    /// The caller must establish that `step` is the run's currently failed step. This
    /// constructor independently requires the durable run failure and attempt evidence,
    /// and rejects attempt overflow before any journal mutation.
    pub fn from_failed_events(
        run: RunId,
        step: StepIdx,
        events: &[JournalEvent],
    ) -> Result<Self, RetryAttemptError> {
        validate_failure_suffix(run, step, events)?;
        let failed_attempt = attempt_for_step(run, step, events)?;
        let next_attempt =
            failed_attempt
                .checked_add(1)
                .ok_or(RetryAttemptError::AttemptOverflow {
                    attempt: failed_attempt,
                })?;
        Ok(Self {
            run,
            step,
            failed_attempt,
            next_attempt,
        })
    }

    /// Run whose failed step is being retried.
    #[must_use]
    pub const fn run(self) -> RunId {
        self.run
    }

    /// Exact observed failed step selected for retry.
    #[must_use]
    pub const fn step(self) -> StepIdx {
        self.step
    }

    /// Attempt number whose failure is being retried.
    #[must_use]
    pub const fn failed_attempt(self) -> u16 {
        self.failed_attempt
    }

    /// Checked next attempt number.
    #[must_use]
    pub const fn next_attempt(self) -> u16 {
        self.next_attempt
    }
}

/// Failures encountered while deriving durable retry authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAttemptError {
    /// The journal does not end in a failed run event.
    RunNotFailed,
    /// The journal contains evidence for a different run.
    RunMismatch,
    /// No attempt-tagged failure evidence exists for the observed step.
    FailedStepMissing { step: StepIdx },
    /// The attempt counter cannot be advanced within the v1 `u16` domain.
    AttemptOverflow { attempt: u16 },
}

impl std::fmt::Display for RetryAttemptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RunNotFailed => formatter.write_str("run is not durably failed"),
            Self::RunMismatch => formatter.write_str("retry journal contains another run"),
            Self::FailedStepMissing { step } => {
                write!(formatter, "no failed attempt evidence for step {step:?}")
            }
            Self::AttemptOverflow { attempt } => {
                write!(formatter, "retry attempt {attempt} cannot be incremented")
            }
        }
    }
}

impl std::error::Error for RetryAttemptError {}

fn validate_failure_suffix(
    run: RunId,
    step: StepIdx,
    events: &[JournalEvent],
) -> Result<(), RetryAttemptError> {
    if events.iter().any(|event| event.run_id() != run) {
        return Err(RetryAttemptError::RunMismatch);
    }
    let latest_attempt = events
        .iter()
        .filter_map(JournalEvent::attempt)
        .max()
        .unwrap_or(1);
    match events.last() {
        Some(JournalEvent::RunFailedEvent { attempt, .. }) if *attempt == latest_attempt => Ok(()),
        Some(JournalEvent::StepFailed {
            step: failed_step, ..
        }) if *failed_step == step => Err(RetryAttemptError::RunNotFailed),
        _ => Err(RetryAttemptError::RunNotFailed),
    }
}

fn attempt_for_step(
    run: RunId,
    step: StepIdx,
    events: &[JournalEvent],
) -> Result<u16, RetryAttemptError> {
    let run_attempt = events
        .iter()
        .filter_map(JournalEvent::attempt)
        .max()
        .unwrap_or(1);
    let latest = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::StepStarted {
                run: event_run,
                step: event_step,
                attempt,
                ..
            }
            | JournalEvent::StepFailed {
                run: event_run,
                step: event_step,
                attempt,
                ..
            }
            | JournalEvent::ActionFailedEvent {
                run: event_run,
                step: event_step,
                attempt,
                ..
            } if *event_run == run && *event_step == step && *attempt == run_attempt => {
                Some(*attempt)
            }
            JournalEvent::RetryScheduledEvent {
                run: event_run,
                step: event_step,
                attempt,
                ..
            } if *event_run == run && *event_step == step && *attempt == run_attempt => {
                Some(*attempt)
            }
            _ => None,
        })
        .max();
    latest.ok_or(RetryAttemptError::FailedStepMissing { step })
}

#[cfg(test)]
#[path = "retry/tests.rs"]
mod tests;
