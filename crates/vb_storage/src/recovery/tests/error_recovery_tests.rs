#![forbid(unsafe_code)]
#![cfg(test)]
//! Error recovery tests for fuzz-malformed journal records.
//!
//! Each test constructs a valid encoded journal record, mutates one byte (or
//! a small targeted region) to simulate a specific fuzz-class corruption, and
//! asserts that the decode/replay pipeline returns the typed `JournalError`
//! variant that the storage contract promises for that mutation class.

#[path = "error_recovery_tests/decode_tests.rs"]
mod decode_tests;
#[path = "error_recovery_tests/replay_tests.rs"]
mod replay_tests;
#[path = "error_recovery_tests/sanity_tests.rs"]
mod sanity_tests;

use crate::JournalEvent;
use crate::codec::encode_journal_event_record;
use vb_core::RunId;

/// Build a minimal valid journal event (RunAccepted at seq=0).
fn sample_event() -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: crate::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x11; 32]),
    }
}

/// Encode a valid record and return the bytes for mutation.
fn encoded_record() -> Vec<u8> {
    encode_journal_event_record(&sample_event()).expect("valid event must encode cleanly")
}

/// Mutate one byte at `offset` (wraps via XOR with 0xFF for a deterministic but
/// content-changing flip).
fn flip_byte(bytes: &mut [u8], offset: usize) {
    if let Some(b) = bytes.get_mut(offset) {
        *b ^= 0xFF;
    }
}

/// Mutate 4 bytes at `offset` to a sentinel that won't match any legitimate
/// header field.
fn scribble_u32(bytes: &mut [u8], offset: usize) {
    let sentinel = 0xDE_AD_BE_EF_u32.to_le_bytes();
    for (i, slot) in bytes.iter_mut().enumerate().skip(offset).take(4) {
        *slot = sentinel[i - offset];
    }
}
