#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Canonical BLAKE3 encoding for every observation variant.
//!
//! Each `encode_*` helper binds one observation variant into the
//! running hasher using a fixed-width byte layout so the BLAKE3 input
//! is byte-deterministic for a given observation. Sub-tags and
//! fixed-width integer fields guarantee that no two observations
//! collapse to the same hash by accident.
//!
//! The dispatcher [`encode_observation_into`] is invoked by
//! [`super::digest::observation_digest`] for every observation in
//! event order. The ask and action encoder sub-modules
//! ([`super::encode_ask`], [`super::encode_action`]) keep this file
//! under the source-length cap.

use blake3::Hasher;

use super::ask::SlotObservation;
use super::encode_action::encode_action;
use super::encode_ask::encode_ask;
use super::lifecycle::{
    LifecycleObservation, StepObservation, TerminalObservation, TimerObservation, WaitObservation,
};
use super::signature::JournalObservation;
use super::subject::DigestObservation;

/// Bind a single observation into the running hasher.
///
/// Dispatched from [`super::digest::observation_digest`]. Each arm
/// appends a kind tag plus variant-specific bytes via a focused
/// per-variant encoder.
pub(crate) fn encode_observation_into(observation: &JournalObservation, hasher: &mut Hasher) {
    hasher.update(&[observation.kind_tag()]);
    match observation {
        JournalObservation::Lifecycle(lifecycle) => encode_lifecycle(lifecycle, hasher),
        JournalObservation::Step(step) => encode_step(step, hasher),
        JournalObservation::Slot(slot) => encode_slot(slot, hasher),
        JournalObservation::Action(action) => encode_action(action, hasher),
        JournalObservation::Ask(ask) => encode_ask(ask, hasher),
        JournalObservation::Wait(wait) => encode_wait(wait, hasher),
        JournalObservation::Timer(timer) => encode_timer(timer, hasher),
        JournalObservation::Terminal(terminal) => encode_terminal(terminal, hasher),
        JournalObservation::Digest(digest) => encode_digest(digest, hasher),
    }
}

fn encode_lifecycle(lifecycle: &LifecycleObservation, hasher: &mut Hasher) {
    match lifecycle {
        LifecycleObservation::Accepted { workflow } => {
            hasher.update(&[1u8]);
            hasher.update(&workflow.bytes);
        }
        LifecycleObservation::Admitted {
            artifact,
            capabilities,
            capability_count,
            policy_tag: policy,
        } => encode_lifecycle_admitted(hasher, artifact, capabilities, *capability_count, *policy),
        LifecycleObservation::Resumed => {
            hasher.update(&[3u8]);
        }
        LifecycleObservation::Retried => {
            hasher.update(&[4u8]);
        }
    }
}

fn encode_lifecycle_admitted(
    hasher: &mut Hasher,
    artifact: &DigestObservation,
    capabilities: &DigestObservation,
    capability_count: u32,
    policy: u8,
) {
    hasher.update(&[2u8]);
    hasher.update(&artifact.bytes);
    hasher.update(&capabilities.bytes);
    hasher.update(&encode_u32(capability_count));
    hasher.update(&[policy]);
}

fn encode_step(step: &StepObservation, hasher: &mut Hasher) {
    match step {
        StepObservation::Started { step, attempt } => {
            hasher.update(&[1u8]);
            hasher.update(&encode_u16(step.get()));
            hasher.update(&encode_u16(*attempt));
        }
        StepObservation::Succeeded { step, output } => {
            hasher.update(&[2u8]);
            hasher.update(&encode_u16(step.get()));
            hasher.update(&encode_u16(output.get()));
        }
    }
}

fn encode_slot(slot: &SlotObservation, hasher: &mut Hasher) {
    hasher.update(&encode_u16(slot.slot.get()));
    hasher.update(&encode_u16(slot.attempt));
    match &slot.value_digest {
        Some(digest) => {
            hasher.update(&[1u8]);
            hasher.update(&digest.bytes);
        }
        None => {
            hasher.update(&[0u8]);
        }
    }
    match &slot.extra_digest {
        Some(digest) => {
            hasher.update(&[1u8]);
            hasher.update(&digest.bytes);
        }
        None => {
            hasher.update(&[0u8]);
        }
    }
}

fn encode_wait(wait: &WaitObservation, hasher: &mut Hasher) {
    match wait {
        WaitObservation::Scheduled { step, attempt } => {
            hasher.update(&[1u8]);
            hasher.update(&encode_u16(step.get()));
            hasher.update(&encode_u16(*attempt));
        }
        WaitObservation::Resolved { step, attempt } => {
            hasher.update(&[2u8]);
            hasher.update(&encode_u16(step.get()));
            hasher.update(&encode_u16(*attempt));
        }
    }
}

fn encode_timer(timer: &TimerObservation, hasher: &mut Hasher) {
    match timer {
        TimerObservation::RetryScheduled { step, attempt } => {
            hasher.update(&[1u8]);
            hasher.update(&encode_u16(step.get()));
            hasher.update(&encode_u16(*attempt));
        }
        TimerObservation::AskTimedOut { step, attempt } => {
            hasher.update(&[2u8]);
            hasher.update(&encode_u16(step.get()));
            hasher.update(&encode_u16(*attempt));
        }
    }
}

fn encode_terminal(terminal: &TerminalObservation, hasher: &mut Hasher) {
    match terminal {
        TerminalObservation::Finished { result, attempt } => {
            hasher.update(&[1u8]);
            hasher.update(&encode_u16(result.get()));
            hasher.update(&encode_u16(*attempt));
        }
        TerminalObservation::Failed { attempt } => {
            hasher.update(&[2u8]);
            hasher.update(&encode_u16(*attempt));
        }
        TerminalObservation::Cancelled { attempt, reason } => {
            encode_terminal_cancelled(hasher, *attempt, reason);
        }
        TerminalObservation::Killed { attempt } => {
            hasher.update(&[4u8]);
            hasher.update(&encode_u16(*attempt));
        }
    }
}

fn encode_terminal_cancelled(
    hasher: &mut Hasher,
    attempt: u16,
    reason: &Option<DigestObservation>,
) {
    hasher.update(&[3u8]);
    hasher.update(&encode_u16(attempt));
    match reason {
        Some(digest) => {
            hasher.update(&[1u8]);
            hasher.update(&digest.bytes);
        }
        None => {
            hasher.update(&[0u8]);
        }
    }
}

fn encode_digest(digest: &DigestObservation, hasher: &mut Hasher) {
    hasher.update(&[digest.subject.tag()]);
    hasher.update(&digest.bytes);
}

fn encode_u16(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

fn encode_u32(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}
