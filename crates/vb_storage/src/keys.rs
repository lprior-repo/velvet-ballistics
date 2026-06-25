#![forbid(unsafe_code)]
//! Key encoding functions for Fjall keyspaces.
//!
//! Each key variant uses a specific binary format with a type prefix
//! followed by the payload fields in big-endian byte order.

use arrayvec::ArrayVec;
use std::ops::Range;
use vb_core::{ActionId, RunId, WorkflowId};

use crate::{
    JournalError,
    constants::{
        DIGEST_KEY_BYTES, INDEX_ACTION_KEY_BYTES, INDEX_STATUS_KEY_BYTES, INDEX_WORKFLOW_KEY_BYTES,
        JOURNAL_KEY_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION,
        PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT, PREFIX_RUN_HEADER,
        PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RUN_ONLY_KEY_BYTES,
    },
    error::KeyDecodeError,
    types::{EventSeq, IndexStatusState, StorageKey},
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

/// Storage key prefix classification for filter-by-kind operations.
///
/// Each variant corresponds to one of the nine known key prefixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyPrefix {
    /// Workflow source by digest (`0x01`).
    WorkflowSource,
    /// Compiled IR by digest (`0x02`).
    CompiledIr,
    /// Run header (`0x10`).
    RunHeader,
    /// Run event journal (`0x11`).
    RunEvent,
    /// Run snapshot (`0x12`).
    RunSnapshot,
    /// Large blob by digest (`0x20`).
    Blob,
    /// Status index (`0x30`).
    IndexStatus,
    /// Workflow index (`0x31`).
    IndexWorkflow,
    /// Action index (`0x32`).
    IndexAction,
}

impl KeyPrefix {
    /// Returns the raw prefix byte for this variant.
    #[must_use]
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::WorkflowSource => PREFIX_WORKFLOW_SOURCE,
            Self::CompiledIr => PREFIX_COMPILED_IR,
            Self::RunHeader => PREFIX_RUN_HEADER,
            Self::RunEvent => PREFIX_RUN_EVENT,
            Self::RunSnapshot => PREFIX_RUN_SNAPSHOT,
            Self::Blob => PREFIX_BLOB,
            Self::IndexStatus => PREFIX_INDEX_STATUS,
            Self::IndexWorkflow => PREFIX_INDEX_WORKFLOW,
            Self::IndexAction => PREFIX_INDEX_ACTION,
        }
    }

    /// Returns the expected total key length (in bytes) for this prefix variant.
    #[must_use]
    pub const fn expected_key_len(self) -> usize {
        match self {
            Self::WorkflowSource | Self::CompiledIr | Self::Blob => DIGEST_KEY_BYTES,
            Self::RunHeader => RUN_ONLY_KEY_BYTES,
            Self::RunEvent | Self::RunSnapshot => JOURNAL_KEY_BYTES,
            Self::IndexStatus => INDEX_STATUS_KEY_BYTES,
            Self::IndexWorkflow | Self::IndexAction => INDEX_WORKFLOW_KEY_BYTES,
        }
    }
}

/// Classifies the first byte of a storage key into a `KeyPrefix`.
///
/// Returns `Err(KeyDecodeError::EmptyKey)` for an empty slice and
/// `Err(KeyDecodeError::UnknownPrefix)` for an unrecognised prefix byte.
///
/// # Examples
///
/// ```
/// use vb_storage::keys::{try_key_prefix, KeyPrefix};
///
/// assert!(matches!(try_key_prefix(&[0x01]), Ok(KeyPrefix::WorkflowSource)));
/// assert!(try_key_prefix(&[]).is_err());
/// assert!(try_key_prefix(&[0xFF]).is_err());
/// ```
pub fn try_key_prefix(bytes: &[u8]) -> Result<KeyPrefix, KeyDecodeError> {
    let &prefix = bytes.first().ok_or(KeyDecodeError::EmptyKey)?;
    match prefix {
        PREFIX_WORKFLOW_SOURCE => Ok(KeyPrefix::WorkflowSource),
        PREFIX_COMPILED_IR => Ok(KeyPrefix::CompiledIr),
        PREFIX_RUN_HEADER => Ok(KeyPrefix::RunHeader),
        PREFIX_RUN_EVENT => Ok(KeyPrefix::RunEvent),
        PREFIX_RUN_SNAPSHOT => Ok(KeyPrefix::RunSnapshot),
        PREFIX_BLOB => Ok(KeyPrefix::Blob),
        PREFIX_INDEX_STATUS => Ok(KeyPrefix::IndexStatus),
        PREFIX_INDEX_WORKFLOW => Ok(KeyPrefix::IndexWorkflow),
        PREFIX_INDEX_ACTION => Ok(KeyPrefix::IndexAction),
        unknown => Err(KeyDecodeError::UnknownPrefix { prefix: unknown }),
    }
}

fn key_length_mismatch(prefix: KeyPrefix, actual: usize) -> KeyDecodeError {
    KeyDecodeError::KeyLengthMismatch {
        prefix: prefix.to_u8(),
        expected: prefix.expected_key_len(),
        actual,
    }
}

fn key_array<const N: usize>(
    bytes: &[u8],
    prefix: KeyPrefix,
    range: Range<usize>,
) -> Result<[u8; N], KeyDecodeError> {
    let slice = bytes
        .get(range)
        .ok_or_else(|| key_length_mismatch(prefix, bytes.len()))?;
    <[u8; N]>::try_from(slice).map_err(|_| key_length_mismatch(prefix, bytes.len()))
}

fn key_byte(bytes: &[u8], prefix: KeyPrefix, index: usize) -> Result<u8, KeyDecodeError> {
    bytes
        .get(index)
        .copied()
        .ok_or_else(|| key_length_mismatch(prefix, bytes.len()))
}

/// Decodes a raw byte slice into a typed `StorageKey`.
///
/// 1. Classifies the prefix byte via `try_key_prefix`.
/// 2. Validates the key length matches the expected byte count for that prefix.
/// 3. Decodes numeric fields from big-endian bytes.
/// 4. Validates domain rules (RunId must be non-zero; EventSeq must not be MAX).
///
/// # Errors
///
/// Returns a `KeyDecodeError` variant for empty input, unknown prefix,
/// length mismatch, invalid RunId (zero), or reserved seq sentinel (MAX).
///
/// # Examples
///
/// ```
/// use vb_storage::keys::decode_storage_key;
/// use vb_storage::StorageKey;
/// use vb_core::RunId;
///
/// let bytes = [0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2A];
/// let expected = StorageKey::RunHeader { run: RunId::new(42) };
/// assert!(decode_storage_key(&bytes).is_ok_and(|k| k == expected));
/// ```
pub fn decode_storage_key(bytes: &[u8]) -> Result<StorageKey, KeyDecodeError> {
    let prefix = try_key_prefix(bytes)?;
    let expected_len = prefix.expected_key_len();
    if bytes.len() != expected_len {
        return Err(KeyDecodeError::KeyLengthMismatch {
            prefix: prefix.to_u8(),
            expected: expected_len,
            actual: bytes.len(),
        });
    }

    match prefix {
        KeyPrefix::WorkflowSource | KeyPrefix::CompiledIr | KeyPrefix::Blob => {
            let digest = key_array::<{ crate::constants::DIGEST_BYTES }>(
                bytes,
                prefix,
                1..DIGEST_KEY_BYTES,
            )?;
            match prefix {
                KeyPrefix::WorkflowSource => Ok(StorageKey::WorkflowSource { digest }),
                KeyPrefix::CompiledIr => Ok(StorageKey::CompiledIr { digest }),
                _ => Ok(StorageKey::Blob { digest }),
            }
        }
        KeyPrefix::RunHeader => {
            let run_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 1..9)?);
            if run_val == 0 {
                return Err(KeyDecodeError::InvalidRunId);
            }
            Ok(StorageKey::RunHeader {
                run: RunId::new(run_val),
            })
        }
        KeyPrefix::RunEvent | KeyPrefix::RunSnapshot => {
            let run_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 1..9)?);
            if run_val == 0 {
                return Err(KeyDecodeError::InvalidRunId);
            }
            let seq_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 9..17)?);
            if seq_val == u64::MAX {
                return Err(KeyDecodeError::ReservedSeqSentinel);
            }
            let run = RunId::new(run_val);
            let seq = EventSeq::new(seq_val);
            match prefix {
                KeyPrefix::RunEvent => Ok(StorageKey::RunEvent { run, seq }),
                _ => Ok(StorageKey::RunSnapshot { run, seq }),
            }
        }
        KeyPrefix::IndexStatus => {
            let state_byte = key_byte(bytes, prefix, 1)?;
            let state = IndexStatusState::from_u8(state_byte);
            let timestamp = u64::from_be_bytes(key_array::<8>(bytes, prefix, 2..10)?);
            let run_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 10..18)?);
            if run_val == 0 {
                return Err(KeyDecodeError::InvalidRunId);
            }
            Ok(StorageKey::IndexStatus {
                state,
                timestamp,
                run: RunId::new(run_val),
            })
        }
        KeyPrefix::IndexWorkflow => {
            let workflow_val = u32::from_be_bytes(key_array::<4>(bytes, prefix, 1..5)?);
            let run_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 5..13)?);
            if run_val == 0 {
                return Err(KeyDecodeError::InvalidRunId);
            }
            Ok(StorageKey::IndexWorkflow {
                workflow: WorkflowId::new(workflow_val),
                run: RunId::new(run_val),
            })
        }
        KeyPrefix::IndexAction => {
            let action_val = u16::from_be_bytes(key_array::<2>(bytes, prefix, 1..3)?);
            let run_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 3..11)?);
            if run_val == 0 {
                return Err(KeyDecodeError::InvalidRunId);
            }
            let step_val = u16::from_be_bytes(key_array::<2>(bytes, prefix, 11..13)?);
            Ok(StorageKey::IndexAction {
                action: ActionId::new(action_val),
                run: RunId::new(run_val),
                step: vb_core::StepIdx::new(step_val),
            })
        }
    }
}

pub fn journal_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RUN_EVENT, run, seq)
}

fn sequenced_run_key(
    prefix: u8,
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
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

fn digest_key(
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

fn run_only_key(prefix: u8, run: RunId) -> Result<[u8; RUN_ONLY_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, RUN_ONLY_KEY_BYTES>::new();
    key.try_push(prefix)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

// Re-export run_prefix for use by FjallJournal
pub(crate) fn run_prefix_key(run: RunId) -> Result<[u8; 9], JournalError> {
    run_prefix(run)
}

#[cfg(test)]
#[path = "keys/tests.rs"]
mod tests;
