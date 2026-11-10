use super::owned::Case;
use super::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Body {
    pub(super) version: u16,
    pub(super) run: RunId,
    pub(super) artifact: WorkflowDigest,
    pub(super) workflow: WorkflowDigest,
    pub(super) source: WorkflowDigest,
    pub(super) slots: u16,
    pub(super) inputs: Box<[vb_core::CompiledInputSlot]>,
    pub(super) resources: vb_core::ResourceContract,
    pub(super) roots: Box<[(vb_core::SlotIdx, SlotValue, vb_core::Taint)]>,
    pub(super) checkpoint: vb_core::value_store::ValueStoreCheckpointV1,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Wire {
    pub(super) body: Body,
    pub(super) digest: WorkflowDigest,
}

pub(super) fn replace(case: &Case, keyspace: &str, key: &[u8], bytes: &[u8]) -> TestResult {
    let database = fjall::Database::builder(case.directory.path())
        .cache_size(1 << 20)
        .open()?;
    let space = database.keyspace(keyspace, fjall::KeyspaceCreateOptions::default)?;
    space.insert(key, bytes)?;
    database.persist(fjall::PersistMode::SyncAll)?;
    drop(space);
    drop(database);
    Ok(())
}

pub(super) fn tamper(
    case: &Case,
    change: impl FnOnce(&mut Wire) -> TestResult,
    redigest: bool,
) -> TestResult {
    // Hostile bytes only. Positive cases use canonical RunSubmission and real bundle validation.
    let mut wire: Wire = postcard::from_bytes(&case.record_bytes)?;
    change(&mut wire)?;
    if redigest {
        wire.digest = WorkflowDigest::from_bytes(blake3::derive_key(
            "velvet-ballistics/owned-run-inputs/v1",
            &postcard::to_allocvec(&wire.body)?,
        ));
    }
    if redigest {
        admission_digest(case, wire.digest)?;
    }
    let payload = postcard::to_allocvec(&wire)?;
    let header = vb_storage::encode_record_header(
        vb_storage::constants::MAGIC_INDEX_RECORD,
        vb_storage::RecordKind::RunOwnedInputs,
        RUN.get(),
        &payload,
        vb_storage::constants::MAX_RUN_INPUT_BYTES,
    )?;
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(&payload);
    replace(
        case,
        vb_storage::constants::KEYSPACE_RUN_INPUTS,
        &vb_storage::keys::run_header_key(RUN)?,
        &bytes,
    )
}

pub(super) fn admission_digest(case: &Case, digest: WorkflowDigest) -> TestResult {
    let mut admission = {
        let journal = case.open()?;
        journal
            .events_for_run_full(RUN)?
            .get(1)
            .cloned()
            .ok_or("admission")?
    };
    let vb_storage::JournalEvent::RunAdmission { input_digest, .. } = &mut admission else {
        return Err("admission variant".into());
    };
    *input_digest = vb_storage::RunInputBinding::Digest(digest);
    let bytes = vb_storage::encode_record(
        vb_storage::constants::MAGIC_JOURNAL_EVENT,
        admission.record_kind(),
        1,
        &admission,
        vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    replace(
        case,
        vb_storage::constants::KEYSPACE_RUN_EVENT,
        &vb_storage::keys::run_event_key(RUN, vb_storage::EventSeq::new(1))?,
        &bytes,
    )
}
