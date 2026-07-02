#![forbid(unsafe_code)]
#![deny(unused_must_use)]
//! Direct `record_*` methods on [`BoundaryTranscriptJournal`].
//!
//! These methods push authority-bearing events whose payload the
//! runtime journal cannot preserve. Each method takes a single typed
//! authority newtype ([`TimerAuthority`], [`AskAnswerAuthority`],
//! [`FailureAuthority`]) and `(self, &authority)` keeps the public
//! signature to two parameters regardless of the authority's internal
//! field count.

use crate::boundary_transcript::authority::{AskAnswerAuthority, FailureAuthority, TimerAuthority};
use crate::boundary_transcript::event::{BoundaryEvent, TranscriptSeq};
use crate::boundary_transcript::journal_projection::BoundaryTranscriptJournal;
use crate::boundary_transcript::transcript::BoundaryTranscriptError;

impl BoundaryTranscriptJournal {
    /// Pushes a [`BoundaryEvent::TimerCaptured`] event with the supplied
    /// authority. Used by the runtime to capture timer registration
    /// authority that the journal cannot preserve.
    pub fn record_timer_captured(
        &self,
        authority: &TimerAuthority,
    ) -> Result<Option<TranscriptSeq>, BoundaryTranscriptError> {
        self.transcript.push(BoundaryEvent::TimerCaptured {
            run: authority.run,
            step: authority.step,
            kind: authority.kind,
            generation: authority.generation,
            deadline: authority.deadline,
            logical_deadline: authority.logical_deadline,
        })
    }

    /// Pushes a [`BoundaryEvent::TimerFired`] event with the supplied
    /// authority. Used by the runtime to capture timer fire authority
    /// that the journal cannot preserve.
    pub fn record_timer_fired(
        &self,
        authority: &TimerAuthority,
    ) -> Result<Option<TranscriptSeq>, BoundaryTranscriptError> {
        self.transcript.push(BoundaryEvent::TimerFired {
            run: authority.run,
            step: authority.step,
            kind: authority.kind,
            generation: authority.generation,
            deadline: authority.deadline,
        })
    }

    /// Pushes a [`BoundaryEvent::AskAnswered`] event with the full
    /// payload fields. Used by the runtime to capture ask answer
    /// authority that the journal cannot preserve.
    pub fn record_ask_answered(
        &self,
        authority: &AskAnswerAuthority,
    ) -> Result<Option<TranscriptSeq>, BoundaryTranscriptError> {
        self.transcript.push(BoundaryEvent::AskAnswered {
            run: authority.run,
            ask_step: authority.ask_step,
            resume_step: authority.resume_step,
            slot: authority.slot,
            taint: authority.taint,
            encoded_len: authority.encoded_len,
        })
    }

    /// Pushes a [`BoundaryEvent::ActionFailed`] event with the full
    /// failure payload. Used by the runtime to capture failure authority
    /// that the journal cannot preserve.
    pub fn record_action_failed(
        &self,
        authority: &FailureAuthority,
    ) -> Result<Option<TranscriptSeq>, BoundaryTranscriptError> {
        let failure_code: u8 = authority.failure_code.into();
        let retry_policy_tag: u8 = authority.retry_policy_tag.into();
        self.transcript.push(BoundaryEvent::ActionFailed {
            run: authority.run,
            step: authority.step,
            action: authority.action,
            attempt: authority.attempt,
            failure_code,
            retry_policy_tag,
            taint: authority.taint,
        })
    }
}
