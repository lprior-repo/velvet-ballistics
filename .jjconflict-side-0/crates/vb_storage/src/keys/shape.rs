#![forbid(unsafe_code)]
//! Key-shape builders shared by the key encoders.

use arrayvec::ArrayVec;
use vb_core::RunId;

use crate::{
    JournalError,
    constants::{DIGEST_KEY_BYTES, JOURNAL_KEY_BYTES, PREFIX_RUN_EVENT, RUN_ONLY_KEY_BYTES},
    types::EventSeq,
};

pub fn journal_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RUN_EVENT, run, seq)
}

pub(crate) fn sequenced_run_key(
    prefix: u8,
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    // PO-cn2v4-001: a zero `RunId` is reserved as "no run"; reject
    // it before any byte writing. The order matters: run validation
    // runs first so the encoder matches the decoder ordering for
    // the (run=0, seq=MAX) case (this surfaces `SequenceOverflow` is
    // NOT possible for zero runs, only `InvalidRunId`).
    require_non_zero_run(run)?;
    if seq.get() == u64::MAX {
        return Err(JournalError::SequenceOverflow);
    }
    let mut key = ArrayVec::<u8, JOURNAL_KEY_BYTES>::new();
    key.try_push(prefix)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&seq.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

fn run_prefix(run: RunId) -> Result<[u8; 9], JournalError> {
    run_only_key(PREFIX_RUN_EVENT, run)
}

pub(crate) fn digest_key(
    prefix: u8,
    digest: [u8; crate::constants::DIGEST_BYTES],
) -> Result<[u8; DIGEST_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, DIGEST_KEY_BYTES>::new();
    key.try_push(prefix)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&digest)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

pub(crate) fn run_only_key(
    prefix: u8,
    run: RunId,
) -> Result<[u8; RUN_ONLY_KEY_BYTES], JournalError> {
    // PO-cn2v4-001: a zero `RunId` is reserved as "no run"; reject
    // it before any byte writing. Covers `run_header_key`,
    // `run_prefix_key`, and the `run_prefix` scan helper used by
    // journal replay/trimming paths.
    require_non_zero_run(run)?;
    let mut key = ArrayVec::<u8, RUN_ONLY_KEY_BYTES>::new();
    key.try_push(prefix)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

/// Validates that `run` is non-zero.
///
/// Zero is reserved as the "no run" sentinel and must not be encoded
/// into any storage key. The decoder in `decode_storage_key` already
/// rejects this on the read path; the encoders mirror the same rule
/// so storage never silently accepts a zero run id at write time
/// and a later decode would surface a different key shape.
///
/// PO-cn2v4-001: zero `RunId` rejection at every storage-key encoder
/// boundary; defence-in-depth against accidental all-zero run
/// encoding that would collide with no-row sentinel keys.
#[inline]
pub(crate) fn require_non_zero_run(run: RunId) -> Result<(), JournalError> {
    if run.get() == 0 {
        return Err(JournalError::InvalidRunId { run });
    }
    Ok(())
}

// Re-export run_prefix for use by FjallJournal
pub(crate) fn run_prefix_key(run: RunId) -> Result<[u8; 9], JournalError> {
    run_prefix(run)
}
