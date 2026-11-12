#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use crate::keys::{
        blob_key, compiled_ir_key, encode_key, index_action_key, index_status_key,
        index_workflow_key, journal_key, run_event_key, run_header_key, run_snapshot_key,
        workflow_source_key,
    };
    use crate::queue::BatchBuilder;
    use crate::recovery::{ActionReplayTracker, RunSnapshot};
    use crate::{
        BlobRecord, CURRENT_SCHEMA_VERSION, CompiledIrRecord, DIGEST_BYTES, DurableActionOutcome,
        EventSeq, FjallJournal, IndexStatusState, JournalError, JournalEvent, JournalWriterQueue,
        KeyspaceProfile, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_IPC_FRAME,
        MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES,
        MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES,
        MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR,
        PREFIX_INDEX_ACTION, PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT,
        PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES,
        RECORD_HEADER_LEN, RecordKind, RunHeaderRecord, StorageKey, StorageLimits,
        WorkflowSourceRecord, append_journal_event, decode_record, decode_record_header,
        encode_record, encode_record_header, flush_profile, init_keyspaces, keyspace_options_for,
        open_store, put_blob, put_compiled_ir, put_run_header, put_workflow_source, read_blob,
        read_run_events, replay_journal, verify_digest_match, write_snapshot,
    };
    use vb_core::{
        ActionId, ActionTicket, CapabilitySet, ConstValue, DiagnosticCode, RunId, RuntimePolicy,
        SeqNo, SlotIdx, StepIdx, Taint, WorkflowDigest, WorkflowId,
    };

    fn compiled_ir_digest(ir: &[u8]) -> WorkflowDigest {
        WorkflowDigest::from_bytes(blake3::hash(ir).into())
    }

    fn compiled_ir_record(ir: impl Into<Vec<u8>>) -> CompiledIrRecord {
        let ir = ir.into();
        let digest = compiled_ir_digest(&ir);
        CompiledIrRecord { digest, ir }
    }

    include!("tests/keys_and_records.rs");

    include!("tests/error_variants_and_bdd.rs");

    include!("tests/lifecycle_and_roundtrips.rs");

    include!("tests/run_header_and_adversarial_decode.rs");

    include!("tests/adversarial_keys_queues.rs");

    include!("tests/batch_write_through.rs");

    include!("tests/envelope_atomicity_locks.rs");
}
