use super::super::support::{Case, TestResult};
use vb_storage::constants::{
    KEYSPACE_RUN_EVENT, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
};
use vb_storage::{EventSeq, JournalEvent};

// Adversarial persisted fixtures only: mutate after canonical commit and close.
// Positive activation always uses the untouched actual atomic submission.
pub(super) fn replace(
    case: &Case,
    keyspace: &str,
    key: &[u8],
    replacement: Option<Vec<u8>>,
) -> TestResult {
    let database = fjall::Database::builder(case.directory.path())
        .cache_size(1 << 20)
        .open()?;
    let space = database.keyspace(keyspace, fjall::KeyspaceCreateOptions::default)?;
    match replacement {
        Some(bytes) => space.insert(key, bytes.as_slice())?,
        None => space.remove(key)?,
    }
    database.persist(fjall::PersistMode::SyncAll)?;
    drop(space);
    drop(database);
    Ok(())
}

pub(super) fn admission(
    case: &Case,
    change: impl FnOnce(&mut JournalEvent) -> TestResult,
) -> TestResult {
    let mut admission = {
        let journal = case.open()?;
        journal
            .events_for_run_full(case.run)?
            .get(1)
            .cloned()
            .ok_or("admission missing")?
    };
    change(&mut admission)?;
    let bytes = vb_storage::encode_record(
        MAGIC_JOURNAL_EVENT,
        admission.record_kind(),
        1,
        &admission,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let key = vb_storage::keys::run_event_key(case.run, EventSeq::new(1))?;
    replace(case, KEYSPACE_RUN_EVENT, &key, Some(bytes))
}

pub(super) fn owned_payload(
    case: &Case,
    change: impl FnOnce(&mut Vec<u8>) -> TestResult,
) -> TestResult {
    let mut payload = {
        let journal = case.open()?;
        let record = journal
            .owned_run_inputs(
                case.run,
                case.artifact.digest,
                case.source.digest,
                &case.workflow,
            )?
            .ok_or("owned record missing")?;
        postcard::to_allocvec(&record)?
    };
    change(&mut payload)?;
    let header = vb_storage::encode_record_header(
        vb_storage::constants::MAGIC_INDEX_RECORD,
        vb_storage::RecordKind::RunOwnedInputs,
        case.run.get(),
        &payload,
        vb_storage::constants::MAX_RUN_INPUT_BYTES,
    )?;
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(&payload);
    let key = vb_storage::keys::run_header_key(case.run)?;
    replace(
        case,
        vb_storage::constants::KEYSPACE_RUN_INPUTS,
        &key,
        Some(bytes),
    )
}
