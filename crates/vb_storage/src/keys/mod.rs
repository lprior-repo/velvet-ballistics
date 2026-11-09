#![forbid(unsafe_code)]
//! Key encoding functions for Fjall keyspaces.
//!
//! Each key variant uses a specific binary format with a type prefix
//! followed by the payload fields in big-endian byte order.

use arrayvec::ArrayVec;
use vb_core::{ActionId, RunId, WorkflowId};

use crate::{
    JournalError,
    constants::{
        DIGEST_KEY_BYTES, INDEX_ACTION_KEY_BYTES, INDEX_STATUS_KEY_BYTES, INDEX_WORKFLOW_KEY_BYTES,
        JOURNAL_KEY_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION,
        PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT,
        PREFIX_WORKFLOW_SOURCE, RUN_ONLY_KEY_BYTES,
    },
    types::{EventSeq, StorageKey},
};

/// Policy that decides how a keyspace iteration handles rows whose key
/// cannot be parsed back into a typed `StorageKey` (truncated bytes,
/// unknown prefix, or shape mismatch).
///
/// Picking a single policy per call site is the unifying rule for CC-002:
/// production paths default to [`Self::FailClosed`]; diagnostic/doctor
/// callers that must keep scanning across partial corruption opt in to
/// [`Self::SkipMalformed`]. Never use `SkipMalformed` on a production
/// read path that is supposed to surface evidence of corruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyspaceScanPolicy {
    /// Surface the first malformed row as a typed
    /// [`crate::JournalError::MalformedKeyspaceRow`] error and abort the
    /// scan. This is the default for production paths.
    #[default]
    FailClosed,
    /// Skip malformed rows silently and continue scanning. Reserved for
    /// doctor/diagnostic tooling that must produce a partial view of a
    /// partially-corrupt keyspace. The caller is responsible for
    /// reporting the skipped count to the operator; the policy itself
    /// does not log.
    SkipMalformed,
}

impl KeyspaceScanPolicy {
    /// Returns the policy used by production scan APIs by default.
    pub const fn default_production() -> Self {
        Self::FailClosed
    }

    /// Returns the policy used by doctor/diagnostic scan APIs by
    /// default. Doctor paths must tolerate partial corruption so the
    /// operator can see whatever is still well-formed.
    pub const fn default_doctor() -> Self {
        Self::SkipMalformed
    }
}

/// Encodes `[0x01][workflow_digest_32]`.
pub fn workflow_source_key(
    digest: [u8; crate::constants::DIGEST_BYTES],
) -> Result<[u8; DIGEST_KEY_BYTES], JournalError> {
    digest_key(PREFIX_WORKFLOW_SOURCE, digest)
}

/// Encodes `[0x02][compiled_digest_32]`.
pub fn compiled_ir_key(
    digest: [u8; crate::constants::DIGEST_BYTES],
) -> Result<[u8; DIGEST_KEY_BYTES], JournalError> {
    digest_key(PREFIX_COMPILED_IR, digest)
}

/// Encodes `[0x10][run_id_u64_be]`.
pub fn run_header_key(run: RunId) -> Result<[u8; RUN_ONLY_KEY_BYTES], JournalError> {
    run_only_key(PREFIX_RUN_HEADER, run)
}

/// Encodes `[0x11][run_id_u64_be][seq_u64_be]`.
pub fn run_event_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    journal_key(run, seq)
}

/// Encodes `[0x12][run_id_u64_be][seq_u64_be]`.
pub fn run_snapshot_key(
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RUN_SNAPSHOT, run, seq)
}

/// Encodes `[0x20][blob_digest_32]`.
pub fn blob_key(
    digest: [u8; crate::constants::DIGEST_BYTES],
) -> Result<[u8; DIGEST_KEY_BYTES], JournalError> {
    digest_key(PREFIX_BLOB, digest)
}

/// Encodes `[0x30][state_u8][timestamp_u64_be][run_id_u64_be]`.
pub fn index_status_key(
    state: crate::types::IndexStatusState,
    timestamp: u64,
    run: RunId,
) -> Result<[u8; INDEX_STATUS_KEY_BYTES], JournalError> {
    // PO-cn2v4-001: a zero `RunId` is reserved as "no run" per the
    // storage contract. Reject it before any byte writing so the typed
    // error surfaces exactly once at the encoder boundary (the
    // decoder in `decode_storage_key` mirrors this check). This is
    // checked BEFORE the IndexStatusState collision check so the
    // encoder ordering matches what the decoder reports when both
    // conditions hold.
    require_non_zero_run(run)?;
    // VB-NOORE (wildcard elimination): use `to_u8_checked` so an
    // `Other(v)` whose byte is in the collision range
    // `0..MIN_OTHER_STATUS_BYTE` is rejected with a typed
    // `JournalError::IndexStatusStateCollision` instead of silently
    // emitting a collision byte (SC-001 / vb-f1xkn).
    let state_byte = state.to_u8_checked()?;
    let mut key = ArrayVec::<u8, INDEX_STATUS_KEY_BYTES>::new();
    key.try_push(PREFIX_INDEX_STATUS)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_push(state_byte)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&timestamp.to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

/// Encodes `[0x31][workflow_id_u32_be][run_id_u64_be]`.
pub fn index_workflow_key(
    workflow: WorkflowId,
    run: RunId,
) -> Result<[u8; INDEX_WORKFLOW_KEY_BYTES], JournalError> {
    // PO-cn2v4-001: mirror the zero-RunId rejection at every public
    // encoder (see `index_status_key` rationale).
    require_non_zero_run(run)?;
    let mut key = ArrayVec::<u8, INDEX_WORKFLOW_KEY_BYTES>::new();
    key.try_push(PREFIX_INDEX_WORKFLOW)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&workflow.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

/// Encodes `[0x32][action_id_u16_be][run_id_u64_be][step_u16_be]`.
pub fn index_action_key(
    action: ActionId,
    run: RunId,
    step: vb_core::StepIdx,
) -> Result<[u8; INDEX_ACTION_KEY_BYTES], JournalError> {
    // PO-cn2v4-001: mirror the zero-RunId rejection at every public
    // encoder (see `index_status_key` rationale).
    require_non_zero_run(run)?;
    let mut key = ArrayVec::<u8, INDEX_ACTION_KEY_BYTES>::new();
    key.try_push(PREFIX_INDEX_ACTION)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&action.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&step.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

/// Encodes any supported storage key into the provided buffer.
///
/// `out` is cleared before writing. This avoids per-call allocation when
/// the caller owns a reusable scratch `Vec<u8>` (see scan paths in
/// `trimming/logic.rs` and the doctor preview loop).
pub fn encode_key_into(key: &StorageKey, out: &mut Vec<u8>) -> Result<(), JournalError> {
    out.clear();
    match key {
        StorageKey::WorkflowSource { digest } => {
            out.extend_from_slice(&workflow_source_key(*digest)?);
        }
        StorageKey::CompiledIr { digest } => {
            out.extend_from_slice(&compiled_ir_key(*digest)?);
        }
        StorageKey::RunHeader { run } => {
            out.extend_from_slice(&run_header_key(*run)?);
        }
        StorageKey::RunEvent { run, seq } => {
            out.extend_from_slice(&run_event_key(*run, *seq)?);
        }
        StorageKey::RunSnapshot { run, seq } => {
            out.extend_from_slice(&run_snapshot_key(*run, *seq)?);
        }
        StorageKey::Blob { digest } => {
            out.extend_from_slice(&blob_key(*digest)?);
        }
        StorageKey::IndexStatus {
            state,
            timestamp,
            run,
        } => {
            out.extend_from_slice(&index_status_key(*state, *timestamp, *run)?);
        }
        StorageKey::IndexWorkflow { workflow, run } => {
            out.extend_from_slice(&index_workflow_key(*workflow, *run)?);
        }
        StorageKey::IndexAction { action, run, step } => {
            out.extend_from_slice(&index_action_key(*action, *run, *step)?);
        }
    }
    Ok(())
}

/// Encodes any supported storage key using the existing typed key encoders.
///
/// Thin wrapper around [`encode_key_into`] that returns an owned `Vec<u8>`.
/// Use [`encode_key_into`] directly in hot scan paths to reuse a single
/// scratch buffer across iterations.
pub fn encode_key(key: StorageKey) -> Result<Vec<u8>, JournalError> {
    let mut buf = Vec::with_capacity(32);
    encode_key_into(&key, &mut buf)?;
    Ok(buf)
}

mod decode;
mod shape;

pub use decode::{KeyPrefix, decode_run_event_key, decode_storage_key, try_key_prefix};
pub use shape::journal_key;
pub(crate) use shape::{
    digest_key, require_non_zero_run, run_only_key, run_prefix_key, sequenced_run_key,
};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
