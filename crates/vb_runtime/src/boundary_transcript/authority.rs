#![forbid(unsafe_code)]
#![deny(unused_must_use)]
//! Newtype authority structs passed to the `record_*` methods on
//! [`BoundaryTranscriptJournal`].
//!
//! Each newtype bundles the full authority fields required to push a
//! boundary event whose payload the runtime journal cannot preserve.
//! Reducing `record_*` from 6-7 params to `(self, &Authority)` makes the
//! call sites self-documenting and gives the type system something to
//! hand to callers.
//!
//! The newtypes are **plain data** — no validation, no panic paths. The
//! constructors take already-validated fields from the runtime; the
//! `record_*` methods perform the final typed write into the transcript.

use std::time::Instant;

use vb_core::action::ActionFailureCode;
use vb_core::action::RetryPolicy;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::value::Taint;

use crate::shard::PendingTimerKind;

/// Typed tag for [`vb_core::action::ActionFailureCode`] used by the
/// [`FailureAuthority`] newtype.
///
/// The on-wire numeric code is the same `u8` discriminant already present
/// in [`crate::boundary_transcript::event::BoundaryEvent::ActionFailed`].
/// This wrapper exists so the authority builder can take a typed
/// [`ActionFailureCode`] without losing the `u8` boundary-transcript
/// encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureCodeTag(pub u8);

impl From<ActionFailureCode> for FailureCodeTag {
    fn from(code: ActionFailureCode) -> Self {
        // `ActionFailureCode` is `#[repr(u8)]` with an explicit `Unknown =
        // 255` discriminant, so this is a total bijection — no `as`-cast,
        // no lossy conversion.
        FailureCodeTag(code as u8)
    }
}

impl From<FailureCodeTag> for u8 {
    fn from(tag: FailureCodeTag) -> Self {
        tag.0
    }
}

/// Typed tag for [`vb_core::action::contract::RetryPolicy`] used by the
/// [`FailureAuthority`] newtype.
///
/// The on-wire numeric code is 0 for `NonRetryable` and 1 for `Retryable`
/// (matches the existing [`crate::boundary_transcript::event::BoundaryEvent::ActionFailed`]
/// `retry_policy_tag` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicyTag(pub u8);

impl From<RetryPolicy> for RetryPolicyTag {
    fn from(policy: RetryPolicy) -> Self {
        // The `BoundaryEvent::ActionFailed::retry_policy_tag` field uses
        // 0 = NonRetryable and 1 = Retryable (the on-wire encoding
        // documented at the field definition). The `RetryPolicy` enum
        // uses the opposite numeric discriminant, so we flip the
        // mapping here.
        let tag = match policy {
            RetryPolicy::NonRetryable => 0,
            RetryPolicy::Retryable => 1,
            _ => 0,
        };
        RetryPolicyTag(tag)
    }
}

impl From<RetryPolicyTag> for u8 {
    fn from(tag: RetryPolicyTag) -> Self {
        tag.0
    }
}

/// Authority bundle for [`crate::boundary_transcript::event::BoundaryEvent::TimerCaptured`].
///
/// Carries the wall-clock `deadline` plus a `logical_deadline` derived
/// from the per-run journal sequence at capture time. Replay uses the
/// `logical_deadline` for cross-process determinism; the wall-clock
/// `deadline` is preserved for the live runtime path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerAuthority {
    /// Owning run.
    pub run: RunId,
    /// Step that registered the timer.
    pub step: StepIdx,
    /// Timer kind.
    pub kind: PendingTimerKind,
    /// Freshness generation.
    pub generation: u64,
    /// Wall-clock deadline authority.
    pub deadline: Instant,
    /// Logical deadline (per-run journal sequence at capture time).
    pub logical_deadline: u64,
}

impl TimerAuthority {
    /// Builds a new [`TimerAuthority`] from validated fields. No
    /// validation is performed — the runtime is responsible for handing
    /// in a consistent `(generation, deadline, kind)` triple.
    #[must_use]
    pub const fn new(
        run: RunId,
        step: StepIdx,
        kind: PendingTimerKind,
        generation: u64,
        deadline: Instant,
        logical_deadline: u64,
    ) -> Self {
        Self {
            run,
            step,
            kind,
            generation,
            deadline,
            logical_deadline,
        }
    }
}

/// Authority bundle for [`crate::boundary_transcript::event::BoundaryEvent::AskAnswered`].
///
/// Captures the full ask-answer payload that the runtime journal cannot
/// preserve (`taint`, `encoded_len`, `resume_step`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskAnswerAuthority {
    /// Owning run.
    pub run: RunId,
    /// Step that issued the ask.
    pub ask_step: StepIdx,
    /// Step that consumes the answer slot.
    pub resume_step: StepIdx,
    /// Slot that received the answer.
    pub slot: SlotIdx,
    /// Taint of the answer.
    pub taint: Taint,
    /// Encoded length of the answer payload in bytes.
    pub encoded_len: u32,
}

impl AskAnswerAuthority {
    /// Builds a new [`AskAnswerAuthority`] from validated fields. No
    /// validation is performed — the runtime is responsible for handing
    /// in a consistent `(ask_step, resume_step, slot, taint)` quadruple.
    #[must_use]
    pub const fn new(
        run: RunId,
        ask_step: StepIdx,
        resume_step: StepIdx,
        slot: SlotIdx,
        taint: Taint,
        encoded_len: u32,
    ) -> Self {
        Self {
            run,
            ask_step,
            resume_step,
            slot,
            taint,
            encoded_len,
        }
    }
}

/// Authority bundle for [`crate::boundary_transcript::event::BoundaryEvent::ActionFailed`].
///
/// Captures the full failure payload (`failure_code`, `retry_policy_tag`,
/// `taint`) that the runtime journal cannot preserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureAuthority {
    /// Owning run.
    pub run: RunId,
    /// Step that received failure.
    pub step: StepIdx,
    /// Action identifier.
    pub action: ActionId,
    /// Execution attempt number for this action.
    pub attempt: u16,
    /// Failure code tag (typed view of `ActionFailureCode`).
    pub failure_code: FailureCodeTag,
    /// Retry policy tag.
    pub retry_policy_tag: RetryPolicyTag,
    /// Taint of the failure.
    pub taint: Taint,
}

impl FailureAuthority {
    /// Builds a new [`FailureAuthority`] from validated fields.
    #[must_use]
    pub const fn new(
        run: RunId,
        step: StepIdx,
        action: ActionId,
        attempt: u16,
        failure_code: FailureCodeTag,
        retry_policy_tag: RetryPolicyTag,
        taint: Taint,
    ) -> Self {
        Self {
            run,
            step,
            action,
            attempt,
            failure_code,
            retry_policy_tag,
            taint,
        }
    }
}
