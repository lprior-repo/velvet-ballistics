use super::support::{Fixture, TestResult, record};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::workflow::{CompiledInputSlot, ResourceContract};
use vb_core::{RunId, SlotIdx, SlotValue, Taint, WorkflowDigest};
use vb_storage::constants::{MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES};
use vb_storage::{OwnedRunInputsRecordV1, RecordKind};

// Negative-wire fixtures mirror the payload, never manufacture a workflow or authority.
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Body {
    pub(super) version: u16,
    pub(super) run: RunId,
    pub(super) artifact: WorkflowDigest,
    pub(super) workflow: WorkflowDigest,
    pub(super) source: WorkflowDigest,
    pub(super) slots: u16,
    pub(super) inputs: Box<[CompiledInputSlot]>,
    pub(super) resources: ResourceContract,
    pub(super) roots: Box<[(SlotIdx, SlotValue, Taint)]>,
    pub(super) checkpoint: ValueStoreCheckpointV1,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Wire {
    pub(super) body: Body,
    pub(super) digest: WorkflowDigest,
}

pub(super) fn fixture_wire(f: &Fixture) -> TestResult<Wire> {
    Ok(postcard::from_bytes(&postcard::to_allocvec(&record(f)?)?)?)
}

pub(super) fn redigest(wire: &mut Wire) -> TestResult {
    wire.digest = WorkflowDigest::from_bytes(blake3::derive_key(
        "velvet-ballistics/owned-run-inputs/v1",
        &postcard::to_allocvec(&wire.body)?,
    ));
    Ok(())
}

pub(super) fn wrap(payload: &[u8], kind: RecordKind, run: RunId) -> TestResult<Vec<u8>> {
    let header = vb_storage::encode_record_header(
        MAGIC_INDEX_RECORD,
        kind,
        run.get(),
        payload,
        MAX_RUN_INPUT_BYTES,
    )?;
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

pub(super) fn encode(wire: &Wire, kind: RecordKind) -> TestResult<Vec<u8>> {
    wrap(&postcard::to_allocvec(wire)?, kind, wire.body.run)
}

pub(super) fn decode(
    f: &Fixture,
    bytes: &[u8],
) -> Result<OwnedRunInputsRecordV1, vb_storage::JournalError> {
    OwnedRunInputsRecordV1::decode(
        bytes,
        f.run,
        f.artifact.digest,
        f.source.digest,
        &f.workflow,
    )
}
