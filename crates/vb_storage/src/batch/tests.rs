#![forbid(unsafe_code)]
pub(super) use crate::batch::*;
pub(super) use crate::{
    BlobRecord, CompiledIrRecord, EventSeq, IndexStatusState, JournalError, JournalEvent,
    RunHeaderRecord, WorkflowSourceRecord,
    codec::encode_record,
    constants::DIGEST_BYTES,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES},
    records::RecordKind,
    recovery::RunSnapshot,
};
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use tempfile::TempDir;
pub(super) use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

pub(super) fn temp_journal() -> (TempDir, crate::FjallJournal) {
    let temp = TempDir::new().expect("tempdir creation should succeed");
    let journal =
        crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

pub(super) fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    }
}

pub(super) fn make_run_header(run: RunId) -> RunHeaderRecord {
    RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
        status: 1,
        accepted_at_ms: 1000,
    }
}

mod t_append_event;
mod t_byte_accounting_part1;
mod t_byte_accounting_part2;
mod t_byte_accounting_part3;
mod t_byte_accounting_part4;
mod t_construction;
mod t_putters_a;
mod t_putters_b;
mod t_strict;
