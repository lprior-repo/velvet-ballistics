// Kani proof harness for PS-004: envelope decode boundary (Gate 2a).

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::record::{
    AcceptedArtifactEnvelopeLengthDecision, classify_accepted_artifact_envelope_len,
};

/// PS-004a: zero-length input is rejected by the postcard envelope decoder.
#[kani::proof]
fn ps_004_zero_length_rejected() {
    let decision = classify_accepted_artifact_envelope_len(0);
    kani::assert(
        decision == AcceptedArtifactEnvelopeLengthDecision::TooShort,
        "zero-length input must not decode",
    );
}

/// PS-004d: one-to-four byte inputs are too short for an AcceptedArtifact envelope.
#[kani::proof]
fn ps_004_truncated_header_rejected() {
    let len: u8 = kani::any();
    kani::assume(len >= 1 && len <= 4);
    let decision = classify_accepted_artifact_envelope_len(usize::from(len));
    kani::assert(
        decision == AcceptedArtifactEnvelopeLengthDecision::TooShort,
        "truncated input must not decode",
    );
}
