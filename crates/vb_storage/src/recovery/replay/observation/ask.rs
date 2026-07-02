#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Ask, answer, and slot observation types.
//!
//! Defines the public-facing observation variants for ask-style
//! external-input events and slot writes. Kept in a dedicated module
//! because ask events carry a richer [`ConstAnswerObservation`]
//! payload that mirrors `vb_core::ConstValue` without depending on its
//! `Debug` formatting.

use vb_core::{ConstValue, SlotIdx, StepIdx};

use super::subject::DigestObservation;

/// Ask observation covering scheduled, answered, recorded, and timed-out.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum AskObservation {
    /// Ask was scheduled.
    Scheduled {
        /// Step index.
        step: StepIdx,
        /// Attempt number.
        attempt: u16,
    },
    /// Ask was answered by an external caller.
    Answered {
        /// Step index.
        step: StepIdx,
        /// Attempt number.
        attempt: u16,
    },
    /// An answer was recorded into a slot.
    AnswerRecorded {
        /// Slot receiving the answer.
        slot: SlotIdx,
        /// Recorded answer value (constant-typed).
        answer: ConstAnswerObservation,
    },
    /// Ask timed out without an answer.
    TimedOut {
        /// Step index.
        step: StepIdx,
        /// Attempt number.
        attempt: u16,
    },
}

/// Reduced representation of `ConstValue` for ask-answer observations.
///
/// `ConstValue` already excludes runtime-allocated handles (List, Object,
/// Blob). We mirror its variants so the observation is fully self-contained
/// without depending on `vb_core::ConstValue`'s `Debug` formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum ConstAnswerObservation {
    /// Explicit null.
    Null,
    /// Boolean.
    Bool(bool),
    /// Signed integer.
    I64(i64),
    /// Boolean discriminant of `FiniteF64`.
    F64Tag,
    /// Symbol handle raw value.
    Symbol(u32),
}

impl ConstAnswerObservation {
    /// Reduce a `ConstValue` into a stable observation variant.
    #[must_use]
    pub(crate) fn from_const(value: ConstValue) -> Self {
        match value {
            ConstValue::Null => Self::Null,
            ConstValue::Bool(v) => Self::Bool(v),
            ConstValue::I64(v) => Self::I64(v),
            ConstValue::F64(_) => Self::F64Tag,
            ConstValue::Symbol(v) => Self::Symbol(v.get()),
            // `ConstValue` is `#[non_exhaustive]`; preserve the observation
            // for any future variant by tagging it via the symbol channel
            // with a sentinel value. ConstValue does not currently have any
            // such variants, but this avoids forcing a future schema bump
            // just to add one.
            _ => Self::Symbol(u32::MAX),
        }
    }
}

/// Slot-write observation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SlotObservation {
    /// Slot index written.
    pub(crate) slot: SlotIdx,
    /// Attempt number.
    pub(crate) attempt: u16,
    /// Digest of the captured slot value bytes (None when payload was absent).
    pub(crate) value_digest: Option<DigestObservation>,
    /// Digest of the encoded slot-write extra envelope (None when absent).
    pub(crate) extra_digest: Option<DigestObservation>,
}
