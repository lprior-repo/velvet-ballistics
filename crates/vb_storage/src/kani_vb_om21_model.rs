#![cfg(kani)]
#![forbid(unsafe_code)]

// Shared bounded model for PO-vb-om21-*-kani harnesses.
// Trust boundary: this mirrors the documented local key contract
// `[0x11][run_id_u64_be][seq_u64_be]` using only fixed arrays and scalar
// conversions because the production ArrayVec encoder currently drives Kani
// into verifier-internal UNDETERMINED memory checks before the vb-om21
// property can be reached.

use crate::constants::{JOURNAL_KEY_BYTES, PREFIX_RUN_EVENT};

#[derive(Clone, Copy)]
pub enum Mode {
    QueryAllowsEmpty,
    RecoveryRequiresJournal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Metadata {
    Missing,
    Equal,
    Above,
    Below,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ok { tail: u64 },
    MissingJournal,
    TailMismatch,
    TailOverflow,
}

pub fn encode_run_event_key(run: u64, seq: u64) -> [u8; JOURNAL_KEY_BYTES] {
    let run_bytes = run.to_be_bytes();
    let seq_bytes = seq.to_be_bytes();
    [
        PREFIX_RUN_EVENT,
        run_bytes[0],
        run_bytes[1],
        run_bytes[2],
        run_bytes[3],
        run_bytes[4],
        run_bytes[5],
        run_bytes[6],
        run_bytes[7],
        seq_bytes[0],
        seq_bytes[1],
        seq_bytes[2],
        seq_bytes[3],
        seq_bytes[4],
        seq_bytes[5],
        seq_bytes[6],
        seq_bytes[7],
    ]
}

pub fn has_prefix(key: &[u8; JOURNAL_KEY_BYTES], run: u64) -> bool {
    let run_bytes = run.to_be_bytes();
    key[0] == PREFIX_RUN_EVENT
        && key[1] == run_bytes[0]
        && key[2] == run_bytes[1]
        && key[3] == run_bytes[2]
        && key[4] == run_bytes[3]
        && key[5] == run_bytes[4]
        && key[6] == run_bytes[5]
        && key[7] == run_bytes[6]
        && key[8] == run_bytes[7]
}

pub fn parse_seq_if_prefix(key: &[u8; JOURNAL_KEY_BYTES], run: u64) -> Option<u64> {
    if has_prefix(key, run) {
        Some(u64::from_be_bytes([
            key[9], key[10], key[11], key[12], key[13], key[14], key[15], key[16],
        ]))
    } else {
        None
    }
}

pub fn lex_le_run_event(a: &[u8; JOURNAL_KEY_BYTES], b: &[u8; JOURNAL_KEY_BYTES]) -> bool {
    let mut idx = 0usize;
    while idx < JOURNAL_KEY_BYTES {
        if a[idx] < b[idx] {
            return true;
        }
        if a[idx] > b[idx] {
            return false;
        }
        idx += 1;
    }
    true
}

pub fn tail_after_max(max_seq: u64) -> Outcome {
    if max_seq == u64::MAX {
        Outcome::TailOverflow
    } else {
        Outcome::Ok { tail: max_seq + 1 }
    }
}

pub fn classify(mode: Mode, seen: bool, max_seq: u64, metadata: Metadata) -> Outcome {
    if matches!(mode, Mode::RecoveryRequiresJournal) && !seen {
        Outcome::MissingJournal
    } else if max_seq == u64::MAX {
        Outcome::TailOverflow
    } else if metadata == Metadata::Below {
        Outcome::TailMismatch
    } else {
        Outcome::Ok { tail: max_seq + 1 }
    }
}

pub fn any_mode(raw: bool) -> Mode {
    if raw {
        Mode::RecoveryRequiresJournal
    } else {
        Mode::QueryAllowsEmpty
    }
}

pub fn any_metadata(raw: u8) -> Metadata {
    match raw % 4 {
        0 => Metadata::Missing,
        1 => Metadata::Equal,
        2 => Metadata::Above,
        _ => Metadata::Below,
    }
}
