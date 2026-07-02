#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Canonical BLAKE3 encoding for action observations.
//!
//! Splits the action-observation encoder into a header encoder (state
//! tag + optional ABI digest) and a capability+outcome encoder so
//! each sub-function fits within the Farley 25-line limit. Kept in
//! its own module so [`super::encode`] stays under the source-length
//! cap.

use blake3::Hasher;

use super::action_types::{ActionObservation, ActionOutcomeObservation};
use super::subject::DigestObservation;

/// Bind the action-observation header (state tag + optional ABI digest)
/// into the hasher. The header must be encoded before
/// [`encode_action_cap_outcome`] so the byte layout stays fixed-width.
pub(crate) fn encode_action_header(
    hasher: &mut Hasher,
    state: &super::action_types::ActionStateObservation,
    abi_digest: &Option<DigestObservation>,
) {
    hasher.update(&[state.tag()]);
    match abi_digest {
        Some(digest) => {
            hasher.update(&[1u8]);
            hasher.update(&digest.bytes);
        }
        None => {
            hasher.update(&[0u8]);
        }
    }
}

/// Bind the action-completion capacity + outcome (or absence thereof)
/// into the hasher.
pub(crate) fn encode_action_cap_outcome(
    hasher: &mut Hasher,
    capacity: Option<u16>,
    outcome: Option<&ActionOutcomeObservation>,
) {
    encode_action_capacity(hasher, capacity);
    encode_action_outcome(hasher, outcome);
}

/// Bind the action-completion capacity (or absence thereof).
fn encode_action_capacity(hasher: &mut Hasher, capacity: Option<u16>) {
    match capacity {
        Some(capacity) => {
            hasher.update(&[1u8]);
            hasher.update(&encode_u16(capacity));
        }
        None => {
            hasher.update(&[0u8]);
        }
    }
}

/// Bind the action-completion outcome (or absence thereof).
fn encode_action_outcome(hasher: &mut Hasher, outcome: Option<&ActionOutcomeObservation>) {
    match outcome {
        Some(ActionOutcomeObservation::Ready {
            taint_tag,
            value_digest,
        }) => {
            hasher.update(&[1u8]);
            hasher.update(&[*taint_tag]);
            hasher.update(value_digest);
        }
        None => {
            hasher.update(&[0u8]);
        }
    }
}

/// Bind the action observation into the hasher.
pub(crate) fn encode_action(action: &ActionObservation, hasher: &mut Hasher) {
    hasher.update(&encode_u16(action.step.get()));
    hasher.update(&encode_u16(action.action.get()));
    hasher.update(&encode_u16(action.attempt));
    encode_action_header(hasher, &action.state, &action.action_abi_digest);
    encode_action_cap_outcome(hasher, action.capacity, action.outcome.as_ref());
}

fn encode_u16(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}
