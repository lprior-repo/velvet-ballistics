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
        PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RECOVERY_STAMP, PREFIX_RUN_EVENT,
        PREFIX_RUN_HEADER, PREFIX_RUN_SEQ_GAP, PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE,
        RECOVERY_STAMP_KEY_BYTES, RUN_ONLY_KEY_BYTES, RUN_SEQ_GAP_KEY_BYTES,
    },
    types::{EventSeq, IndexStatusState},
};

// ============================================================================
// Public encoding functions
// ============================================================================

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

/// Encodes `[0x40][run_id_u64_be][seq_u64_be]`.
pub fn recovery_stamp_key(
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; RECOVERY_STAMP_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RECOVERY_STAMP, run, seq)
}

/// Encodes `[0x13][run_id_u64_be][seq_u64_be]` for the `run_seq_gap` keyspace.
pub fn run_seq_gap_key(
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; RUN_SEQ_GAP_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RUN_SEQ_GAP, run, seq)
}

/// Encodes `[0x30][state_u8][timestamp_u64_be][run_id_u64_be]`.
pub fn index_status_key(
    state: IndexStatusState,
    timestamp: u64,
    run: RunId,
) -> Result<[u8; INDEX_STATUS_KEY_BYTES], JournalError> {
    // SC-001 defense-in-depth: reject Other payloads that would overflow the
    // offset encoding (v + MIN_OTHER_BYTE > u8::MAX). try_new_other already
    // enforces this at construction, but a direct variant construction can
    // still produce an out-of-range payload and we must not let it corrupt
    // the wire format.
    if let IndexStatusState::Other(byte) = state
        && byte > IndexStatusState::MAX_OTHER_BYTE
    {
        return Err(JournalError::IndexStatusStateCollision { byte });
    }
    let mut key = ArrayVec::<u8, INDEX_STATUS_KEY_BYTES>::new();
    key.try_push(PREFIX_INDEX_STATUS)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_push(state.to_u8())
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

/// Encodes any supported storage key using the existing typed key encoders.
pub fn encode_key(key: crate::types::StorageKey) -> Result<Vec<u8>, JournalError> {
    let encoded = match key {
        crate::types::StorageKey::WorkflowSource { digest } => {
            workflow_source_key(digest)?.to_vec()
        }
        crate::types::StorageKey::CompiledIr { digest } => compiled_ir_key(digest)?.to_vec(),
        crate::types::StorageKey::RunHeader { run } => run_header_key(run)?.to_vec(),
        crate::types::StorageKey::RunEvent { run, seq } => run_event_key(run, seq)?.to_vec(),
        crate::types::StorageKey::RunSnapshot { run, seq } => run_snapshot_key(run, seq)?.to_vec(),
        crate::types::StorageKey::Blob { digest } => blob_key(digest)?.to_vec(),
        crate::types::StorageKey::IndexStatus {
            state,
            timestamp,
            run,
        } => index_status_key(state, timestamp, run)?.to_vec(),
        crate::types::StorageKey::IndexWorkflow { workflow, run } => {
            index_workflow_key(workflow, run)?.to_vec()
        }
        crate::types::StorageKey::IndexAction { action, run, step } => {
            index_action_key(action, run, step)?.to_vec()
        }
        crate::types::StorageKey::RecoveryStamp { run, seq } => {
            recovery_stamp_key(run, seq)?.to_vec()
        }
        crate::types::StorageKey::RunSeqGap { run, seq } => run_seq_gap_key(run, seq)?.to_vec(),
    };
    Ok(encoded)
}

// ============================================================================
// Internal helper functions
// ============================================================================

pub fn journal_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RUN_EVENT, run, seq)
}

fn sequenced_run_key(
    prefix: u8,
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    if EventSeq::is_reserved_sentinel(seq.get()) {
        return Err(JournalError::ReservedSeqSentinel);
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
