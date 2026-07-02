#![forbid(unsafe_code)]
#![deny(unused_must_use)]
//! [`BoundaryEvent`] enum and the monotonic [`TranscriptSeq`] sequence
//! number type used by the boundary transcript.

use std::time::Instant;

use vb_core::action::ActionTicket;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::Taint;

use crate::shard::PendingTimerKind;

/// Single boundary transcript entry capturing one action/ask/timer boundary
/// event with full payload and replay identifiers.
///
/// All variants include the full fields needed to replay the event without
/// consulting the runtime state — the original bead review rejected a thinner
/// payload-only summary that dropped the authority fields required to
/// reconstruct a timer firing or a fail-action outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryEvent {
    /// Action scheduled (modern envelope) with full ticket authority.
    ActionScheduled {
        /// Owning run.
        run: RunId,
        /// Full ticket issued by the runtime.
        ticket: ActionTicket,
    },
    /// Action scheduled (legacy path) — no ticket authority; only the
    /// legacy `(run, step, action)` triple is available.
    ActionScheduledLegacy {
        /// Owning run.
        run: RunId,
        /// Step that scheduled the action.
        step: StepIdx,
        /// Action identifier.
        action: vb_core::ids::ActionId,
    },
    /// Action completed (modern envelope) with full payload.
    ActionCompletedModern {
        /// Owning run.
        run: RunId,
        /// Full ticket completed by the boundary.
        ticket: ActionTicket,
        /// Output slot written.
        output_slot: SlotIdx,
        /// Encoded byte length.
        encoded_len: u32,
        /// Taint of the result.
        taint: Taint,
        /// BLAKE3 digest of the encoded value.
        value_digest: [u8; 32],
    },
    /// Action completed (legacy path) — no output payload; only the
    /// `(run, step, action)` triple is available.
    ActionCompletedLegacy {
        /// Owning run.
        run: RunId,
        /// Step that received completion.
        step: StepIdx,
        /// Action identifier.
        action: vb_core::ids::ActionId,
    },
    /// Action failed with full failure payload.
    ActionFailed {
        /// Owning run.
        run: RunId,
        /// Step that received failure.
        step: StepIdx,
        /// Action identifier.
        action: vb_core::ids::ActionId,
        /// Execution attempt number for this action.
        attempt: u16,
        /// Machine-readable failure code (0 = Unknown, 1 = Rejected,
        /// 2 = Timeout, …). The numeric tag is the
        /// [`vb_core::action::ActionFailureCode`] discriminant.
        failure_code: u8,
        /// Retry policy tag (0 = NonRetryable, 1 = Retryable).
        retry_policy_tag: u8,
        /// Taint of the failure.
        taint: Taint,
    },
    /// Action abandoned because the run was cancelled or killed.
    ActionAbandoned {
        /// Owning run.
        run: RunId,
        /// Full ticket that was abandoned.
        ticket: ActionTicket,
    },
    /// Ask scheduled.
    AskScheduled {
        /// Owning run.
        run: RunId,
        /// Step that scheduled the ask.
        step: StepIdx,
    },
    /// Ask answered with full payload.
    AskAnswered {
        /// Owning run.
        run: RunId,
        /// Step that issued the ask.
        ask_step: StepIdx,
        /// Step that consumes the answer slot.
        resume_step: StepIdx,
        /// Slot that received the answer.
        slot: SlotIdx,
        /// Taint of the answer.
        taint: Taint,
        /// Encoded length of the answer payload in bytes.
        encoded_len: u32,
    },
    /// Ask timed out.
    AskTimedOut {
        /// Owning run.
        run: RunId,
        /// Step that timed out.
        step: StepIdx,
    },
    /// Wait scheduled.
    WaitScheduled {
        /// Owning run.
        run: RunId,
        /// Step that scheduled the wait.
        step: StepIdx,
    },
    /// Wait resolved.
    WaitResolved {
        /// Owning run.
        run: RunId,
        /// Step that resolved.
        step: StepIdx,
    },
    /// Timer captured with full authority required to replay a firing.
    TimerCaptured {
        /// Owning run.
        run: RunId,
        /// Step that registered the timer.
        step: StepIdx,
        /// Timer kind.
        kind: PendingTimerKind,
        /// Freshness generation.
        generation: u64,
        /// Wall-clock deadline.
        deadline: Instant,
        /// Logical deadline — the monotonic per-run journal sequence
        /// at the time of capture. Stable across processes, unlike
        /// the wall-clock `deadline`.
        logical_deadline: u64,
    },
    /// Timer fired with full authority.
    TimerFired {
        /// Owning run.
        run: RunId,
        /// Step that registered the timer.
        step: StepIdx,
        /// Timer kind.
        kind: PendingTimerKind,
        /// Freshness generation.
        generation: u64,
        /// Wall-clock deadline authority.
        deadline: Instant,
    },
}

impl BoundaryEvent {
    /// Run identifier carried by this event, regardless of variant.
    #[must_use]
    pub fn run_id(&self) -> RunId {
        match self {
            Self::ActionScheduled { run, .. }
            | Self::ActionScheduledLegacy { run, .. }
            | Self::ActionCompletedModern { run, .. }
            | Self::ActionCompletedLegacy { run, .. }
            | Self::ActionFailed { run, .. }
            | Self::ActionAbandoned { run, .. }
            | Self::AskScheduled { run, .. }
            | Self::AskAnswered { run, .. }
            | Self::AskTimedOut { run, .. }
            | Self::WaitScheduled { run, .. }
            | Self::WaitResolved { run, .. }
            | Self::TimerCaptured { run, .. }
            | Self::TimerFired { run, .. } => *run,
        }
    }

    /// Stable variant name for parity comparisons.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::ActionScheduled { .. } => "ActionScheduled",
            Self::ActionScheduledLegacy { .. } => "ActionScheduledLegacy",
            Self::ActionCompletedModern { .. } => "ActionCompletedModern",
            Self::ActionCompletedLegacy { .. } => "ActionCompletedLegacy",
            Self::ActionFailed { .. } => "ActionFailed",
            Self::ActionAbandoned { .. } => "ActionAbandoned",
            Self::AskScheduled { .. } => "AskScheduled",
            Self::AskAnswered { .. } => "AskAnswered",
            Self::AskTimedOut { .. } => "AskTimedOut",
            Self::WaitScheduled { .. } => "WaitScheduled",
            Self::WaitResolved { .. } => "WaitResolved",
            Self::TimerCaptured { .. } => "TimerCaptured",
            Self::TimerFired { .. } => "TimerFired",
        }
    }
}

/// Monotonic sequence number assigned to each captured boundary event.
pub type TranscriptSeq = u64;
