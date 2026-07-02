#![forbid(unsafe_code)]
#![deny(unused_must_use)]
//! Cold-path boundary transcript capture for deterministic replay.
//!
//! Records action, ask, and timer boundary events with stable ordering and
//! full payload so scripted boundary outcomes can be replayed deterministically.
//!
//! # Layering
//!
//! The module is decomposed into focused submodules:
//!
//! 1. [`event`]: [`BoundaryEvent`] enum and the monotonic [`TranscriptSeq`]
//!    sequence-number type.
//! 2. [`transcript`]: [`BoundaryTranscript`] / [`SharedBoundaryTranscript`]
//!    bounded FIFO capture with typed [`BoundaryTranscriptError`].
//! 3. [`authority`]: Newtype structs carrying typed authority for the
//!    events the journal cannot preserve ([`TimerAuthority`],
//!    [`AskAnswerAuthority`], [`FailureAuthority`]).
//! 4. [`journal_projection`]: [`BoundaryTranscriptJournal`] projection of
//!    runtime journal events into boundary events (per-variant helpers).
//! 5. [`record`]: `record_*` methods on [`BoundaryTranscriptJournal`] that
//!    accept the authority newtypes (≤2 params each).
//!
//! Public API surface (preserved across the split):
//! [`BoundaryEvent`], [`BoundaryTranscript`], [`SharedBoundaryTranscript`],
//! [`BoundaryTranscriptError`], [`BoundaryTranscriptJournal`],
//! [`BoundaryTranscriptEntry`], [`TranscriptSeq`].

mod authority;
mod event;
mod journal_projection;
mod record;
mod shared_transcript;
#[cfg(test)]
mod tests;
mod transcript;

pub use authority::{
    AskAnswerAuthority, FailureAuthority, FailureCodeTag, RetryPolicyTag, TimerAuthority,
};
pub use event::{BoundaryEvent, TranscriptSeq};
pub use journal_projection::BoundaryTranscriptJournal;
pub use shared_transcript::SharedBoundaryTranscript;
pub use transcript::{BoundaryTranscript, BoundaryTranscriptEntry, BoundaryTranscriptError};
