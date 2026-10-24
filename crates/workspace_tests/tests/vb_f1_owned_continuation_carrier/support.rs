use vb_runtime::run_continuation::{ContinuationAuthority, RunContinuationV1};
use vb_runtime::shard::types::RunState;
use vb_storage::{ContinuationCarrierBudget as Budget, ContinuationCarrierHeader as Header};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const MAX: usize = 1_048_576;
pub(super) const ALLOCATION: usize = 67_108_864;
pub(super) const WORK: usize = 16_777_216;

pub(super) fn budget() -> TestResult<Budget> {
    Ok(Budget::new(MAX, ALLOCATION, WORK)?)
}

pub(super) fn fixture(
    after: bool,
    blob: usize,
) -> TestResult<(RunState, ContinuationAuthority, Vec<u8>, Header)> {
    let (state, authority) = super::component_fixture::carrier_fixture(after, blob)?;
    let ordinary = RunContinuationV1::capture(&state, &authority)?;
    let header = Header::new(
        state.frame.run_id(),
        vb_storage::EventSeq::new(2),
        1,
        vb_storage::EventSeq::new(1),
        ordinary.digest().as_bytes(),
    )?;
    Ok((state, authority, ordinary.to_bytes()?, header))
}

pub(super) fn key(header: Header) -> TestResult<[u8; 17]> {
    Ok(vb_storage::keys::run_event_key(
        header.run(),
        header.own_seq(),
    )?)
}

pub(super) fn envelope(payload: &[u8], seq: u64) -> TestResult<Vec<u8>> {
    let mut bytes = vb_storage::encode_record_header(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunContinuation,
        seq,
        payload,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?
    .to_vec();
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

pub(super) fn expected_payload(header: Header, component: &[u8]) -> TestResult<Vec<u8>> {
    Ok(postcard::to_allocvec(&(
        43_u16,
        1_u16,
        header.run(),
        header.own_seq(),
        header.actual_attempt(),
        header.prior_seq(),
        1_u16,
        header.component_digest(),
        component,
    ))?)
}

pub(super) fn event(attempt: u16) -> TestResult<vb_storage::JournalEvent> {
    let (_, _, component, original) = fixture(false, 3)?;
    let header = Header::new(
        original.run(),
        vb_storage::EventSeq::new(5),
        attempt,
        vb_storage::EventSeq::new(4),
        original.component_digest(),
    )?;
    let encoded = vb_storage::encode_owned_continuation(header, &component, &mut budget()?)?;
    let mut operation = budget()?;
    Ok(vb_storage::decode_owned_continuation(
        &key(header)?,
        header.run(),
        encoded.as_bytes(),
        &mut operation,
    )?
    .into_event(&mut operation)?)
}
