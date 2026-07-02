#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Canonical BLAKE3 encoding for ask, answer-recorded, and timer
//! observations.
//!
//! Each helper binds one observation variant into the running hasher
//! using a fixed-width byte layout so the BLAKE3 input is
//! byte-deterministic for a given observation. Kept in its own module
//! so [`super::encode`] stays under the source-length cap.

use blake3::Hasher;

use super::ask::{AskObservation, ConstAnswerObservation};

/// Bind the ask observation variant into the hasher.
pub(crate) fn encode_ask(ask: &AskObservation, hasher: &mut Hasher) {
    match ask {
        AskObservation::Scheduled { step, attempt } => {
            encode_ask_scheduled(hasher, *step, *attempt);
        }
        AskObservation::Answered { step, attempt } => {
            encode_ask_answered(hasher, *step, *attempt);
        }
        AskObservation::AnswerRecorded { slot, answer } => {
            encode_ask_answer_recorded(hasher, *slot, answer);
        }
        AskObservation::TimedOut { step, attempt } => {
            encode_ask_timed_out(hasher, *step, *attempt);
        }
    }
}

/// Bind a scheduled ask into the hasher.
fn encode_ask_scheduled(hasher: &mut Hasher, step: vb_core::StepIdx, attempt: u16) {
    hasher.update(&[1u8]);
    hasher.update(&encode_u16(step.get()));
    hasher.update(&encode_u16(attempt));
}

/// Bind an answered ask into the hasher.
fn encode_ask_answered(hasher: &mut Hasher, step: vb_core::StepIdx, attempt: u16) {
    hasher.update(&[2u8]);
    hasher.update(&encode_u16(step.get()));
    hasher.update(&encode_u16(attempt));
}

/// Bind an answer-recorded event into the hasher.
fn encode_ask_answer_recorded(
    hasher: &mut Hasher,
    slot: vb_core::SlotIdx,
    answer: &ConstAnswerObservation,
) {
    hasher.update(&[3u8]);
    hasher.update(&encode_u16(slot.get()));
    encode_const_answer(answer, hasher);
}

/// Bind a timed-out ask into the hasher.
fn encode_ask_timed_out(hasher: &mut Hasher, step: vb_core::StepIdx, attempt: u16) {
    hasher.update(&[4u8]);
    hasher.update(&encode_u16(step.get()));
    hasher.update(&encode_u16(attempt));
}

/// Bind a `ConstAnswerObservation` variant into the hasher.
pub(crate) fn encode_const_answer(answer: &ConstAnswerObservation, hasher: &mut Hasher) {
    match answer {
        ConstAnswerObservation::Null => {
            hasher.update(&[0u8]);
        }
        ConstAnswerObservation::Bool(v) => {
            hasher.update(&[1u8]);
            hasher.update(&[u8::from(*v)]);
        }
        ConstAnswerObservation::I64(v) => {
            hasher.update(&[2u8]);
            hasher.update(&encode_i64(*v));
        }
        ConstAnswerObservation::F64Tag => {
            hasher.update(&[3u8]);
        }
        ConstAnswerObservation::Symbol(v) => {
            hasher.update(&[4u8]);
            hasher.update(&encode_u32(*v));
        }
    }
}

fn encode_u16(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

fn encode_u32(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

fn encode_i64(value: i64) -> [u8; 8] {
    value.to_le_bytes()
}
