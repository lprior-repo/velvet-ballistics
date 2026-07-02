#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Lifecycle, step, wait, timer, and terminal observation variants.
//!
//! These cover the run-execution spine of the semantic schema:
//! admission / resume / retry transitions, step starts / completions,
//! wait suspensions / resolutions, retry / ask-timeout timers, and the
//! terminal run state. The split is mechanical: each observation group
//! belongs to a different event family and is encoded under its own
//! sub-tag in [`super::encode`].

use vb_core::{SlotIdx, StepIdx};

use super::subject::DigestObservation;

/// Run-lifecycle observation derived from admission / resume / retry events.
///
/// Lifecycle observations appear at most once per run start, in event order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum LifecycleObservation {
    /// Run was accepted; carries the workflow digest for divergence detection.
    Accepted {
        /// Workflow digest captured by the journal.
        workflow: DigestObservation,
    },
    /// Run was admitted; carries artifact, policy, and capability-set digests.
    Admitted {
        /// Artifact digest admitted for this run.
        artifact: DigestObservation,
        /// Capability-set digest covering all grants.
        capabilities: DigestObservation,
        /// Capability grant count, preserved for divergent grants.
        capability_count: u32,
        /// Policy discriminant, captured for stable ordering.
        policy_tag: u8,
    },
    /// Run was resumed after suspension.
    Resumed,
    /// Run was retried after a previous failure.
    Retried,
}

/// Step-level observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum StepObservation {
    /// Step began execution.
    Started {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Step completed and wrote an output slot.
    Succeeded {
        /// Step index.
        step: StepIdx,
        /// Output slot index.
        output: SlotIdx,
    },
}

/// Wait-suspension observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum WaitObservation {
    /// Wait was scheduled at this step.
    Scheduled {
        /// Step index.
        step: StepIdx,
        /// Attempt number.
        attempt: u16,
    },
    /// Wait was resolved by an external timer / trigger.
    Resolved {
        /// Step index.
        step: StepIdx,
        /// Attempt number.
        attempt: u16,
    },
}

/// Timer observation covering retries and ask-timeouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum TimerObservation {
    /// Retry was scheduled at a step.
    RetryScheduled {
        /// Step index.
        step: StepIdx,
        /// Attempt number.
        attempt: u16,
    },
    /// Ask timer fired without an answer.
    AskTimedOut {
        /// Step index.
        step: StepIdx,
        /// Attempt number.
        attempt: u16,
    },
}

/// Terminal observation marking the final state of a run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum TerminalObservation {
    /// Run completed and produced a result slot.
    Finished {
        /// Result slot.
        result: SlotIdx,
        /// Attempt number.
        attempt: u16,
    },
    /// Run failed.
    Failed {
        /// Attempt number.
        attempt: u16,
    },
    /// Run was cancelled.
    Cancelled {
        /// Attempt number.
        attempt: u16,
        /// Optional cancellation-reason digest (None when no reason recorded).
        reason: Option<DigestObservation>,
    },
    /// Run was killed.
    Killed {
        /// Attempt number.
        attempt: u16,
    },
}
