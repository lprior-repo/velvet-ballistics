#![forbid(unsafe_code)]
//! Semantic observation types for stable cross-run comparison.
//!
//! These types normalize raw `JournalEvent` streams into a schema-versioned
//! observation format that hides nondeterministic storage details while
//! preserving semantic differences. The resulting `JournalObservationSignature`
//! provides a digest for fast equality checks and a full observation list for
//! detailed diff analysis.

use vb_core::{ActionId, ConstValue, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};

// ============================================================================
// Schema version
// ============================================================================

/// Semantic observation schema version.
///
/// Bump this whenever the observation model changes in a way that breaks
/// comparison of signatures produced by earlier versions.
pub const SEMANTIC_OBSERVATION_SCHEMA_VERSION: u32 = 2;

// ============================================================================
// Error types
// ============================================================================

/// Errors that can occur while building a `JournalObservationSignature`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationSignatureError {
    /// An allocation failed while building the observation list.
    AllocationFailed,
}

impl std::fmt::Display for ObservationSignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservationSignatureError::AllocationFailed => {
                write!(f, "allocation failed while building observations")
            }
        }
    }
}

impl std::error::Error for ObservationSignatureError {}

// ============================================================================
// Main signature type
// ============================================================================

/// A complete normalized semantic signature derived from a journal event stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalObservationSignature {
    /// Schema version used to produce this signature.
    pub schema_version: u32,
    /// Ordered list of semantic observations.
    pub observations: Vec<JournalObservation>,
    /// BLAKE3 digest over the ordered observations for fast comparison.
    pub digest: [u8; 32],
}

// ============================================================================
// Observation discriminants
// ============================================================================

/// All semantic observation categories produced from a journal event stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalObservation {
    /// Lifecycle observation (accepted, resumed, retried, admitted).
    Lifecycle(LifecycleObservation),
    /// Step observation (started, succeeded, failed).
    Step(StepObservation),
    /// Terminal observation (finished, failed, cancelled, killed).
    Terminal(TerminalObservation),
    /// Slot write observation.
    Slot(SlotObservation),
    /// Action observation (scheduled, completed, failed, abandoned).
    Action(ActionObservation),
    /// Wait observation (scheduled, resolved).
    Wait(WaitObservation),
    /// Ask observation (scheduled, answered, timed_out, answer_recorded).
    Ask(AskObservation),
    /// Timer observation (retry scheduled, ask timed out).
    Timer(TimerObservation),
    /// Cryptographic digest observation.
    Digest(DigestObservation),
}

// ============================================================================
// Lifecycle observations
// ============================================================================

/// Observations about run lifecycle transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleObservation {
    /// Run was accepted after input mapping.
    Accepted,
    /// Run was admitted with specific policy and capabilities.
    Admitted {
        /// Policy used for admission.
        policy: RuntimePolicy,
        /// Stable digest of the capability set.
        capabilities_digest: [u8; 32],
        /// Canonical count of granted capabilities.
        capability_count: u32,
    },
    /// Run was resumed from a waiting state.
    Resumed,
    /// Run was retried after failure.
    Retried,
}

// ============================================================================
// Step observations
// ============================================================================

/// Observations about step execution states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepObservation {
    /// Step began execution.
    Started {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Step completed successfully and wrote an output slot.
    Succeeded {
        /// Step index.
        step: StepIdx,
        /// Output slot index.
        output: SlotIdx,
    },
    /// Step failed.
    Failed {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
}

// ============================================================================
// Terminal observations
// ============================================================================

/// Observations about terminal run states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalObservation {
    /// Run was cancelled (possibly with a reason).
    Cancelled {
        /// Attempt number (1-based).
        attempt: u16,
        /// Optional BLAKE3 digest of the cancellation reason string.
        reason_digest: Option<[u8; 32]>,
    },
    /// Run was forcefully killed.
    Killed {
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Run completed successfully.
    Finished {
        /// Result slot index.
        result: SlotIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Run failed.
    Failed {
        /// Attempt number (1-based).
        attempt: u16,
    },
}

// ============================================================================
// Slot observations
// ============================================================================

/// Observation of a slot write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotObservation {
    /// Slot index.
    pub slot: SlotIdx,
    /// Attempt number (1-based).
    pub attempt: u16,
    /// BLAKE3 digest of the slot value bytes, if present.
    pub value_digest: Option<[u8; 32]>,
    /// BLAKE3 digest of the extra envelope bytes, if present.
    pub extra_digest: Option<[u8; 32]>,
}

// ============================================================================
// Action observations
// ============================================================================

/// Observations about action lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionObservation {
    /// Action was scheduled.
    Scheduled {
        /// Action identifier.
        action: ActionId,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
        /// Action ABI digest, if available (from `ActionScheduledTicket` or
        /// `ActionCompletedEnvelope`).
        action_abi_digest: Option<WorkflowDigest>,
    },
    /// Action completed successfully.
    Completed {
        /// Action identifier.
        action: ActionId,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Action failed.
    Failed {
        /// Action identifier.
        action: ActionId,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Action was abandoned because the run was terminated before completion.
    Abandoned {
        /// Action identifier.
        action: ActionId,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
        /// Capacity of the action ticket at abandonment time.
        capacity: u16,
    },
}

// ============================================================================
// Wait observations
// ============================================================================

/// Observations about wait/suspend points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitObservation {
    /// Wait was scheduled (run entered a waiting state).
    Scheduled {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Wait was resolved by an external signal.
    Resolved {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
}

// ============================================================================
// Ask observations
// ============================================================================

/// Observations about ask/answer interactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskObservation {
    /// Ask was scheduled (run entered an asking state).
    Scheduled {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Ask was answered by an external event.
    Answered {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Answer was recorded to a slot (from `RunAnswered`).
    AnswerRecorded {
        /// Slot that received the answer.
        slot: SlotIdx,
        /// The answer value.
        answer: ConstValue,
    },
    /// Ask timed out.
    TimedOut {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
}

// ============================================================================
// Timer observations
// ============================================================================

/// Observations about timer-based scheduling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerObservation {
    /// Retry was scheduled.
    RetryScheduled {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Ask timed out and triggered a timer event.
    AskTimedOut {
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
}

// ============================================================================
// Digest observations
// ============================================================================

/// Subject of a digest observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestSubject {
    /// Workflow source digest.
    Workflow,
    /// Compiled artifact digest.
    Artifact,
}

/// A digest observation recorded during normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestObservation {
    /// What the digest represents.
    pub subject: DigestSubject,
    /// The BLAKE3 digest bytes.
    pub digest: [u8; 32],
}
