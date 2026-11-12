#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::*;
use crate::{
    BlobRecord, CompiledIrRecord, EventSeq, IndexStatusState, JournalError, JournalEvent,
    RunHeaderRecord, WorkflowSourceRecord, constants::*, recovery::RunSnapshot,
};
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    }
}

fn make_step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

fn corrupt_magic_preserving_crc(value: &mut [u8]) {
    let magic_bytes = 0xDEAD_BEEFu32.to_le_bytes();
    if let Some(slice) = value.get_mut(0..4) {
        slice.copy_from_slice(&magic_bytes);
    }
    let checksum = crc32c::crc32c(&value[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = value.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }
}

fn accepted_artifact_record_for_source(
    source_digest: WorkflowDigest,
) -> Result<CompiledIrRecord, JournalError> {
    let zero_digest = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);
    let mut artifact = crate::admission::AcceptedArtifact {
        digest: zero_digest,
        source_digest,
        policy_digest: WorkflowDigest::from_bytes([0xA5; DIGEST_BYTES]),
        ir: b"compiled ir payload".to_vec(),
        verification: crate::admission::VerificationProof::new(zero_digest, 15, false),
        accepted_at_seq: EventSeq::new(0),
        required_capabilities: Box::new([]),
        action_contracts: Box::new([]),
        artifact_version: crate::compat::CURRENT_IR_VERSION,
    };
    let digest = crate::admission::accepted_artifact_digest(&artifact)?;
    artifact.digest = digest;
    artifact.verification.digest = digest;
    let ir = postcard::to_allocvec(&artifact).map_err(JournalError::PostcardEncodeFailed)?;
    Ok(CompiledIrRecord { digest, ir })
}

include!("tests/roundtrips_and_read_rejections.rs");
include!("tests/isolation_ordering_and_duplicates.rs");
include!("tests/digests_snapshots_headers_and_indexes.rs");
include!("tests/edge_cases_and_header_validation.rs");
include!("tests/snapshot_tail_replay_and_boundaries.rs");
include!("tests/durability_close_and_replay.rs");
