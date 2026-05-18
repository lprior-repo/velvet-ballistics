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
        PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT, PREFIX_RUN_HEADER,
        PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RUN_ONLY_KEY_BYTES,
    },
    types::{EventSeq, StorageKey},
};

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
pub fn encode_key(key: StorageKey) -> Result<Vec<u8>, JournalError> {
    let encoded = match key {
        StorageKey::WorkflowSource { digest } => workflow_source_key(digest)?.to_vec(),
        StorageKey::CompiledIr { digest } => compiled_ir_key(digest)?.to_vec(),
        StorageKey::RunHeader { run } => run_header_key(run)?.to_vec(),
        StorageKey::RunEvent { run, seq } => run_event_key(run, seq)?.to_vec(),
        StorageKey::RunSnapshot { run, seq } => run_snapshot_key(run, seq)?.to_vec(),
        StorageKey::Blob { digest } => blob_key(digest)?.to_vec(),
        StorageKey::IndexStatus {
            state,
            timestamp,
            run,
        } => index_status_key(state, timestamp, run)?.to_vec(),
        StorageKey::IndexWorkflow { workflow, run } => index_workflow_key(workflow, run)?.to_vec(),
        StorageKey::IndexAction { action, run, step } => {
            index_action_key(action, run, step)?.to_vec()
        }
    };
    Ok(encoded)
}

pub fn journal_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RUN_EVENT, run, seq)
}

fn sequenced_run_key(
    prefix: u8,
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
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
mod tests {
    use super::*;
    use crate::IndexStatusState;
    use crate::constants::{
        DIGEST_KEY_BYTES, INDEX_ACTION_KEY_BYTES, INDEX_STATUS_KEY_BYTES, INDEX_WORKFLOW_KEY_BYTES,
        JOURNAL_KEY_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION,
        PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT, PREFIX_RUN_HEADER,
        PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RUN_ONLY_KEY_BYTES,
    };

    // =========================================================================
    // Key construction: workflow_source_key
    // =========================================================================

    #[test]
    fn workflow_source_key_has_correct_prefix() -> Result<(), JournalError> {
        let digest = [0xAB_u8; crate::constants::DIGEST_BYTES];
        let key = workflow_source_key(digest)?;
        assert_eq!(key[0], PREFIX_WORKFLOW_SOURCE, "prefix byte must be 0x01");
        Ok(())
    }

    #[test]
    fn workflow_source_key_embeds_digest() -> Result<(), JournalError> {
        let digest = [0xCD_u8; crate::constants::DIGEST_BYTES];
        let key = workflow_source_key(digest)?;
        assert_eq!(
            &key[1..],
            &digest[..],
            "bytes after prefix must match the input digest"
        );
        Ok(())
    }

    #[test]
    fn workflow_source_key_length() -> Result<(), JournalError> {
        let digest = [0u8; crate::constants::DIGEST_BYTES];
        let key = workflow_source_key(digest)?;
        assert_eq!(key.len(), DIGEST_KEY_BYTES);
        Ok(())
    }

    // =========================================================================
    // Key construction: compiled_ir_key
    // =========================================================================

    #[test]
    fn compiled_ir_key_has_correct_prefix() -> Result<(), JournalError> {
        let digest = [0u8; crate::constants::DIGEST_BYTES];
        let key = compiled_ir_key(digest)?;
        assert_eq!(key[0], PREFIX_COMPILED_IR, "prefix byte must be 0x02");
        Ok(())
    }

    #[test]
    fn compiled_ir_key_preserves_digest_bytes() -> Result<(), JournalError> {
        let digest = [0xEF_u8; crate::constants::DIGEST_BYTES];
        let key = compiled_ir_key(digest)?;
        assert_eq!(&key[1..], &digest[..]);
        Ok(())
    }

    // =========================================================================
    // Key construction: run_header_key
    // =========================================================================

    #[test]
    fn run_header_key_has_correct_prefix() -> Result<(), JournalError> {
        let run = RunId::new(0);
        let key = run_header_key(run)?;
        assert_eq!(key[0], PREFIX_RUN_HEADER, "prefix byte must be 0x10");
        Ok(())
    }

    #[test]
    fn run_header_key_encodes_run_id_big_endian() -> Result<(), JournalError> {
        let run = RunId::new(0x0102_0304_0506_0708);
        let key = run_header_key(run)?;
        let expected = run.get().to_be_bytes();
        assert_eq!(
            &key[1..9],
            &expected,
            "run id must be big-endian after prefix"
        );
        Ok(())
    }

    #[test]
    fn run_header_key_length() -> Result<(), JournalError> {
        let key = run_header_key(RunId::new(42))?;
        assert_eq!(key.len(), RUN_ONLY_KEY_BYTES);
        Ok(())
    }

    // =========================================================================
    // Key construction: run_event_key / journal_key
    // =========================================================================

    #[test]
    fn run_event_key_has_correct_prefix() -> Result<(), JournalError> {
        let run = RunId::new(1);
        let seq = EventSeq::new(0);
        let key = run_event_key(run, seq)?;
        assert_eq!(key[0], PREFIX_RUN_EVENT, "prefix byte must be 0x11");
        Ok(())
    }

    #[test]
    fn run_event_key_encodes_run_and_seq_big_endian() -> Result<(), JournalError> {
        let run = RunId::new(0xAABBCCDD_EEFF0011);
        let seq = EventSeq::new(0x1122_3344_5566_7788);
        let key = run_event_key(run, seq)?;
        assert_eq!(&key[1..9], &run.get().to_be_bytes(), "run id portion");
        assert_eq!(&key[9..17], &seq.get().to_be_bytes(), "seq portion");
        Ok(())
    }

    #[test]
    fn run_event_key_length() -> Result<(), JournalError> {
        let key = run_event_key(RunId::new(0), EventSeq::new(0))?;
        assert_eq!(key.len(), JOURNAL_KEY_BYTES);
        Ok(())
    }

    #[test]
    fn journal_key_matches_run_event_key() -> Result<(), JournalError> {
        let run = RunId::new(99);
        let seq = EventSeq::new(7);
        let a = run_event_key(run, seq)?;
        let b = journal_key(run, seq)?;
        assert_eq!(
            a, b,
            "journal_key and run_event_key must produce identical bytes"
        );
        Ok(())
    }

    // =========================================================================
    // Key construction: run_snapshot_key
    // =========================================================================

    #[test]
    fn run_snapshot_key_has_correct_prefix() -> Result<(), JournalError> {
        let run = RunId::new(5);
        let seq = EventSeq::new(3);
        let key = run_snapshot_key(run, seq)?;
        assert_eq!(key[0], PREFIX_RUN_SNAPSHOT, "prefix byte must be 0x12");
        Ok(())
    }

    #[test]
    fn run_snapshot_key_encodes_run_and_seq() -> Result<(), JournalError> {
        let run = RunId::new(0x1234_5678_9ABC_DEF0);
        let seq = EventSeq::new(0xDEAD_BEEF_CAFE_BABE);
        let key = run_snapshot_key(run, seq)?;
        assert_eq!(&key[1..9], &run.get().to_be_bytes());
        assert_eq!(&key[9..17], &seq.get().to_be_bytes());
        Ok(())
    }

    // =========================================================================
    // Key construction: blob_key
    // =========================================================================

    #[test]
    fn blob_key_has_correct_prefix() -> Result<(), JournalError> {
        let digest = [0u8; crate::constants::DIGEST_BYTES];
        let key = blob_key(digest)?;
        assert_eq!(key[0], PREFIX_BLOB, "prefix byte must be 0x20");
        Ok(())
    }

    #[test]
    fn blob_key_preserves_digest_bytes() -> Result<(), JournalError> {
        let digest = [0x42_u8; crate::constants::DIGEST_BYTES];
        let key = blob_key(digest)?;
        assert_eq!(&key[1..], &digest[..]);
        Ok(())
    }

    // =========================================================================
    // Key construction: index_status_key
    // =========================================================================

    #[test]
    fn index_status_key_has_correct_prefix() -> Result<(), JournalError> {
        let key = index_status_key(IndexStatusState::Submitted, 0, RunId::new(0))?;
        assert_eq!(key[0], PREFIX_INDEX_STATUS, "prefix byte must be 0x30");
        Ok(())
    }

    #[test]
    fn index_status_key_encodes_state_timestamp_run() -> Result<(), JournalError> {
        let state = IndexStatusState::Other(0x05);
        let timestamp: u64 = 0x0102_0304_0506_0708;
        let run = RunId::new(0xAABB_CCDD_EEFF_0011);
        let key = index_status_key(state, timestamp, run)?;
        assert_eq!(key[0], PREFIX_INDEX_STATUS, "prefix");
        assert_eq!(key[1], state.to_u8(), "state byte");
        assert_eq!(
            &key[2..10],
            &timestamp.to_be_bytes(),
            "timestamp big-endian"
        );
        assert_eq!(&key[10..18], &run.get().to_be_bytes(), "run id big-endian");
        Ok(())
    }

    #[test]
    fn index_status_key_length() -> Result<(), JournalError> {
        let key = index_status_key(IndexStatusState::Submitted, 0, RunId::new(0))?;
        assert_eq!(key.len(), INDEX_STATUS_KEY_BYTES);
        Ok(())
    }

    // =========================================================================
    // Key construction: index_workflow_key
    // =========================================================================

    #[test]
    fn index_workflow_key_has_correct_prefix() -> Result<(), JournalError> {
        let key = index_workflow_key(WorkflowId::new(1), RunId::new(1))?;
        assert_eq!(key[0], PREFIX_INDEX_WORKFLOW, "prefix byte must be 0x31");
        Ok(())
    }

    #[test]
    fn index_workflow_key_encodes_workflow_and_run() -> Result<(), JournalError> {
        let workflow = WorkflowId::new(0x12345678);
        let run = RunId::new(0xAABBCCDD_EEFF0011);
        let key = index_workflow_key(workflow, run)?;
        assert_eq!(
            &key[1..5],
            &workflow.get().to_be_bytes(),
            "workflow u32 big-endian"
        );
        assert_eq!(&key[5..13], &run.get().to_be_bytes(), "run u64 big-endian");
        Ok(())
    }

    #[test]
    fn index_workflow_key_length() -> Result<(), JournalError> {
        let key = index_workflow_key(WorkflowId::new(0), RunId::new(0))?;
        assert_eq!(key.len(), INDEX_WORKFLOW_KEY_BYTES);
        Ok(())
    }

    // =========================================================================
    // Key construction: index_action_key
    // =========================================================================

    #[test]
    fn index_action_key_has_correct_prefix() -> Result<(), JournalError> {
        let key = index_action_key(ActionId::new(1), RunId::new(1), vb_core::StepIdx::new(1))?;
        assert_eq!(key[0], PREFIX_INDEX_ACTION, "prefix byte must be 0x32");
        Ok(())
    }

    #[test]
    fn index_action_key_encodes_action_run_step() -> Result<(), JournalError> {
        let action = ActionId::new(0x1234);
        let run = RunId::new(0xDEAD_BEEF_CAFE_BABE);
        let step = vb_core::StepIdx::new(0x5678);
        let key = index_action_key(action, run, step)?;
        assert_eq!(
            &key[1..3],
            &action.get().to_be_bytes(),
            "action u16 big-endian"
        );
        assert_eq!(&key[3..11], &run.get().to_be_bytes(), "run u64 big-endian");
        assert_eq!(
            &key[11..13],
            &step.get().to_be_bytes(),
            "step u16 big-endian"
        );
        Ok(())
    }

    #[test]
    fn index_action_key_length() -> Result<(), JournalError> {
        let key = index_action_key(ActionId::new(0), RunId::new(0), vb_core::StepIdx::new(0))?;
        assert_eq!(key.len(), INDEX_ACTION_KEY_BYTES);
        Ok(())
    }

    // =========================================================================
    // Key construction: encode_key (polymorphic)
    // =========================================================================

    #[test]
    fn encode_key_workflow_source_matches_typed_encoder() -> Result<(), JournalError> {
        let digest = [0x11_u8; crate::constants::DIGEST_BYTES];
        let typed = workflow_source_key(digest)?;
        let generic = encode_key(StorageKey::WorkflowSource { digest })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    #[test]
    fn encode_key_compiled_ir_matches_typed_encoder() -> Result<(), JournalError> {
        let digest = [0x22_u8; crate::constants::DIGEST_BYTES];
        let typed = compiled_ir_key(digest)?;
        let generic = encode_key(StorageKey::CompiledIr { digest })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    #[test]
    fn encode_key_run_header_matches_typed_encoder() -> Result<(), JournalError> {
        let run = RunId::new(42);
        let typed = run_header_key(run)?;
        let generic = encode_key(StorageKey::RunHeader { run })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    #[test]
    fn encode_key_run_event_matches_typed_encoder() -> Result<(), JournalError> {
        let run = RunId::new(7);
        let seq = EventSeq::new(3);
        let typed = run_event_key(run, seq)?;
        let generic = encode_key(StorageKey::RunEvent { run, seq })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    #[test]
    fn encode_key_run_snapshot_matches_typed_encoder() -> Result<(), JournalError> {
        let run = RunId::new(99);
        let seq = EventSeq::new(1);
        let typed = run_snapshot_key(run, seq)?;
        let generic = encode_key(StorageKey::RunSnapshot { run, seq })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    #[test]
    fn encode_key_blob_matches_typed_encoder() -> Result<(), JournalError> {
        let digest = [0x33_u8; crate::constants::DIGEST_BYTES];
        let typed = blob_key(digest)?;
        let generic = encode_key(StorageKey::Blob { digest })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    #[test]
    fn encode_key_index_status_matches_typed_encoder() -> Result<(), JournalError> {
        let state = IndexStatusState::Completed;
        let timestamp = 12345u64;
        let run = RunId::new(67);
        let typed = index_status_key(state, timestamp, run)?;
        let generic = encode_key(StorageKey::IndexStatus {
            state,
            timestamp,
            run,
        })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    #[test]
    fn encode_key_index_workflow_matches_typed_encoder() -> Result<(), JournalError> {
        let workflow = WorkflowId::new(10);
        let run = RunId::new(20);
        let typed = index_workflow_key(workflow, run)?;
        let generic = encode_key(StorageKey::IndexWorkflow { workflow, run })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    #[test]
    fn encode_key_index_action_matches_typed_encoder() -> Result<(), JournalError> {
        let action = ActionId::new(5);
        let run = RunId::new(30);
        let step = vb_core::StepIdx::new(2);
        let typed = index_action_key(action, run, step)?;
        let generic = encode_key(StorageKey::IndexAction { action, run, step })?;
        assert_eq!(generic, typed.to_vec());
        Ok(())
    }

    // =========================================================================
    // Prefix uniqueness: each key type must produce a distinct first byte
    // =========================================================================

    #[test]
    fn all_digest_key_prefixes_are_distinct() -> Result<(), JournalError> {
        let digest = [0u8; crate::constants::DIGEST_BYTES];
        let ws = workflow_source_key(digest)?;
        let ci = compiled_ir_key(digest)?;
        let bl = blob_key(digest)?;
        // All three digest-key types must have different prefixes.
        assert_ne!(
            ws[0], ci[0],
            "workflow_source and compiled_ir prefixes must differ"
        );
        assert_ne!(
            ws[0], bl[0],
            "workflow_source and blob prefixes must differ"
        );
        assert_ne!(ci[0], bl[0], "compiled_ir and blob prefixes must differ");
        Ok(())
    }

    #[test]
    fn all_run_key_prefixes_are_distinct() -> Result<(), JournalError> {
        let run = RunId::new(1);
        let seq = EventSeq::new(1);
        let header = run_header_key(run)?;
        let event = run_event_key(run, seq)?;
        let snapshot = run_snapshot_key(run, seq)?;
        assert_ne!(
            header[0], event[0],
            "run_header and run_event prefixes must differ"
        );
        assert_ne!(
            header[0], snapshot[0],
            "run_header and run_snapshot prefixes must differ"
        );
        assert_ne!(
            event[0], snapshot[0],
            "run_event and run_snapshot prefixes must differ"
        );
        Ok(())
    }

    // =========================================================================
    // Deterministic: same inputs produce same keys
    // =========================================================================

    #[test]
    fn key_encoding_is_deterministic() -> Result<(), JournalError> {
        let digest = [0xFF_u8; crate::constants::DIGEST_BYTES];
        let run = RunId::new(12345);
        let seq = EventSeq::new(67890);

        // Call each encoder twice and verify the results are identical.
        assert_eq!(workflow_source_key(digest)?, workflow_source_key(digest)?);
        assert_eq!(compiled_ir_key(digest)?, compiled_ir_key(digest)?);
        assert_eq!(run_header_key(run)?, run_header_key(run)?);
        assert_eq!(run_event_key(run, seq)?, run_event_key(run, seq)?);
        assert_eq!(run_snapshot_key(run, seq)?, run_snapshot_key(run, seq)?);
        assert_eq!(blob_key(digest)?, blob_key(digest)?);
        assert_eq!(
            index_status_key(IndexStatusState::Submitted, 999, run)?,
            index_status_key(IndexStatusState::Submitted, 999, run)?
        );
        assert_eq!(
            index_workflow_key(WorkflowId::new(42), run)?,
            index_workflow_key(WorkflowId::new(42), run)?
        );
        assert_eq!(
            index_action_key(ActionId::new(3), run, vb_core::StepIdx::new(4))?,
            index_action_key(ActionId::new(3), run, vb_core::StepIdx::new(4))?
        );
        Ok(())
    }

    // =========================================================================
    // Boundary: zero and max values
    // =========================================================================

    #[test]
    fn run_header_key_with_zero_run_id() -> Result<(), JournalError> {
        let key = run_header_key(RunId::new(0))?;
        assert_eq!(key[0], PREFIX_RUN_HEADER);
        assert_eq!(&key[1..], &0u64.to_be_bytes());
        Ok(())
    }

    #[test]
    fn run_header_key_with_max_run_id() -> Result<(), JournalError> {
        let run = RunId::new(u64::MAX);
        let key = run_header_key(run)?;
        assert_eq!(&key[1..], &u64::MAX.to_be_bytes());
        Ok(())
    }

    #[test]
    fn run_event_key_with_zero_seq() -> Result<(), JournalError> {
        let key = run_event_key(RunId::new(1), EventSeq::new(0))?;
        assert_eq!(&key[9..], &0u64.to_be_bytes());
        Ok(())
    }

    #[test]
    fn run_event_key_with_max_values() -> Result<(), JournalError> {
        let key = run_event_key(RunId::new(u64::MAX), EventSeq::new(u64::MAX))?;
        assert_eq!(&key[1..9], &u64::MAX.to_be_bytes());
        assert_eq!(&key[9..], &u64::MAX.to_be_bytes());
        Ok(())
    }

    #[test]
    fn index_status_key_with_zero_values() -> Result<(), JournalError> {
        let key = index_status_key(IndexStatusState::Submitted, 0, RunId::new(0))?;
        assert_eq!(key[1], 0);
        assert_eq!(&key[2..10], &0u64.to_be_bytes());
        assert_eq!(&key[10..18], &0u64.to_be_bytes());
        Ok(())
    }

    #[test]
    fn index_action_key_with_max_values() -> Result<(), JournalError> {
        let key = index_action_key(
            ActionId::new(u16::MAX),
            RunId::new(u64::MAX),
            vb_core::StepIdx::new(u16::MAX),
        )?;
        assert_eq!(&key[1..3], &u16::MAX.to_be_bytes());
        assert_eq!(&key[3..11], &u64::MAX.to_be_bytes());
        assert_eq!(&key[11..13], &u16::MAX.to_be_bytes());
        Ok(())
    }

    // =========================================================================
    // Different inputs produce different keys
    // =========================================================================

    #[test]
    fn different_digests_produce_different_keys() -> Result<(), JournalError> {
        let mut digest_a = [0u8; crate::constants::DIGEST_BYTES];
        digest_a[0] = 1;
        let mut digest_b = [0u8; crate::constants::DIGEST_BYTES];
        digest_b[0] = 2;
        assert_ne!(
            workflow_source_key(digest_a)?,
            workflow_source_key(digest_b)?,
            "different digests must produce different keys"
        );
        Ok(())
    }

    #[test]
    fn different_run_ids_produce_different_keys() -> Result<(), JournalError> {
        let a = run_header_key(RunId::new(1))?;
        let b = run_header_key(RunId::new(2))?;
        assert_ne!(a, b, "different run ids must produce different keys");
        Ok(())
    }

    #[test]
    fn different_sequences_produce_different_keys() -> Result<(), JournalError> {
        let run = RunId::new(10);
        let a = run_event_key(run, EventSeq::new(0))?;
        let b = run_event_key(run, EventSeq::new(1))?;
        assert_ne!(a, b, "different sequences must produce different keys");
        Ok(())
    }

    // =========================================================================
    // run_prefix_key
    // =========================================================================

    #[test]
    fn run_prefix_key_has_run_event_prefix() -> Result<(), JournalError> {
        let run = RunId::new(42);
        let prefix = run_prefix_key(run)?;
        assert_eq!(
            prefix[0], PREFIX_RUN_EVENT,
            "run_prefix must use PREFIX_RUN_EVENT"
        );
        Ok(())
    }

    #[test]
    fn run_prefix_key_encodes_run_id() -> Result<(), JournalError> {
        let run = RunId::new(0x0102_0304_0506_0708);
        let prefix = run_prefix_key(run)?;
        assert_eq!(&prefix[1..9], &run.get().to_be_bytes());
        Ok(())
    }

    #[test]
    fn run_prefix_key_is_9_bytes() -> Result<(), JournalError> {
        let prefix = run_prefix_key(RunId::new(0))?;
        assert_eq!(prefix.len(), RUN_ONLY_KEY_BYTES);
        Ok(())
    }
}
