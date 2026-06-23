#![forbid(unsafe_code)]
//! Key decoding functions for Fjall keyspaces.
//!
//! Provides:
//! - `KeyPrefix` enum for classification
//! - `try_key_prefix` for prefix classification
//! - `decode_storage_key` for full key decoding

use std::ops::Range;

use vb_core::{ActionId, RunId, WorkflowId};

use crate::{
    error::KeyDecodeError,
    types::{EventSeq, IndexStatusState, StorageKey},
};

use crate::constants::{
    DIGEST_BYTES, DIGEST_KEY_BYTES, INDEX_STATUS_KEY_BYTES, INDEX_WORKFLOW_KEY_BYTES,
    JOURNAL_KEY_BYTES, RECOVERY_STAMP_KEY_BYTES, RUN_ONLY_KEY_BYTES,
};

// ============================================================================
// KeyPrefix enum
// ============================================================================

/// Storage key prefix classification for filter-by-kind operations.
///
/// Each variant corresponds to one of the known key prefixes.
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
    /// Recovery-stamp progress marker (`0x40`).
    RecoveryStamp,
}

impl KeyPrefix {
    /// Returns the raw prefix byte for this variant.
    #[must_use]
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::WorkflowSource => crate::constants::PREFIX_WORKFLOW_SOURCE,
            Self::CompiledIr => crate::constants::PREFIX_COMPILED_IR,
            Self::RunHeader => crate::constants::PREFIX_RUN_HEADER,
            Self::RunEvent => crate::constants::PREFIX_RUN_EVENT,
            Self::RunSnapshot => crate::constants::PREFIX_RUN_SNAPSHOT,
            Self::Blob => crate::constants::PREFIX_BLOB,
            Self::IndexStatus => crate::constants::PREFIX_INDEX_STATUS,
            Self::IndexWorkflow => crate::constants::PREFIX_INDEX_WORKFLOW,
            Self::IndexAction => crate::constants::PREFIX_INDEX_ACTION,
            Self::RecoveryStamp => crate::constants::PREFIX_RECOVERY_STAMP,
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
            Self::RecoveryStamp => RECOVERY_STAMP_KEY_BYTES,
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
/// assert_eq!(try_key_prefix(&[0x01]).unwrap(), KeyPrefix::WorkflowSource);
/// assert!(matches!(try_key_prefix(&[]), Err(KeyDecodeError::EmptyKey)));
/// assert!(matches!(try_key_prefix(&[0xFF]), Err(KeyDecodeError::UnknownPrefix)));
/// ```
pub fn try_key_prefix(bytes: &[u8]) -> Result<KeyPrefix, KeyDecodeError> {
    let &prefix = bytes.first().ok_or(KeyDecodeError::EmptyKey)?;
    match prefix {
        crate::constants::PREFIX_WORKFLOW_SOURCE => Ok(KeyPrefix::WorkflowSource),
        crate::constants::PREFIX_COMPILED_IR => Ok(KeyPrefix::CompiledIr),
        crate::constants::PREFIX_RUN_HEADER => Ok(KeyPrefix::RunHeader),
        crate::constants::PREFIX_RUN_EVENT => Ok(KeyPrefix::RunEvent),
        crate::constants::PREFIX_RUN_SNAPSHOT => Ok(KeyPrefix::RunSnapshot),
        crate::constants::PREFIX_BLOB => Ok(KeyPrefix::Blob),
        crate::constants::PREFIX_INDEX_STATUS => Ok(KeyPrefix::IndexStatus),
        crate::constants::PREFIX_INDEX_WORKFLOW => Ok(KeyPrefix::IndexWorkflow),
        crate::constants::PREFIX_INDEX_ACTION => Ok(KeyPrefix::IndexAction),
        crate::constants::PREFIX_RECOVERY_STAMP => Ok(KeyPrefix::RecoveryStamp),
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

// ============================================================================
// Full key decoding
// ============================================================================

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
/// let key = decode_storage_key(&bytes).unwrap();
/// let expected = StorageKey::RunHeader { run: RunId::new(42) };
/// assert_eq!(key, expected);
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
            let digest = key_array::<DIGEST_BYTES>(bytes, prefix, 1..DIGEST_KEY_BYTES)?;
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
        KeyPrefix::RecoveryStamp => {
            let run_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 1..9)?);
            if run_val == 0 {
                return Err(KeyDecodeError::InvalidRunId);
            }
            let seq_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 9..17)?);
            if seq_val == u64::MAX {
                return Err(KeyDecodeError::ReservedSeqSentinel);
            }
            Ok(StorageKey::RecoveryStamp {
                run: RunId::new(run_val),
                seq: EventSeq::new(seq_val),
            })
        }
    }
}
