use super::cursor::Cursor;
use super::{
    CarrierError, ContinuationCarrierBudget, ContinuationCarrierHeader,
    OwnedContinuationCarrierView,
};
use vb_core::{RunId, value_store::transport::CheckpointTransportError as Error};

/// Bounded keyed route. Declared opaque bytes remain borrowed until explicit copy.
pub fn decode_owned_continuation<'a>(
    key: &[u8],
    expected_run: RunId,
    record: &'a [u8],
    operation: &mut ContinuationCarrierBudget,
) -> Result<OwnedContinuationCarrierView<'a>, CarrierError> {
    operation.wire()?;
    if record.len() > operation.record_limit {
        return Err(Error::Bound("whole_record").into());
    }
    let envelope = crate::codec::decode_record_header(
        record,
        crate::MAGIC_JOURNAL_EVENT,
        crate::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    if envelope.record_kind != 43 {
        return Err(CarrierError::Identity("record kind"));
    }
    let (_, payload) = crate::codec::payload::decode_record_payload(
        record,
        crate::MAGIC_JOURNAL_EVENT,
        crate::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let view = payload_view(payload, operation.record_limit)?;
    let (key_run, key_seq) = crate::keys::decode_run_event_key(key, expected_run)?;
    if view.header.run() != key_run
        || view.header.own_seq() != key_seq
        || view.header.own_seq().get() != envelope.sequence
    {
        return Err(CarrierError::Identity("key, run or envelope sequence"));
    }
    super::integrity::component(view.component, view.header.component_digest())?;
    Ok(view)
}

fn payload_view(
    payload: &[u8],
    limit: usize,
) -> Result<OwnedContinuationCarrierView<'_>, CarrierError> {
    let mut cursor = Cursor::new(payload);
    if cursor.u16()? != 43 {
        return Err(CarrierError::Identity("stable tag"));
    }
    version(&mut cursor, "carrier")?;
    let run = RunId::new(cursor.integer()?);
    let own = crate::EventSeq::new(cursor.integer()?);
    let attempt = cursor.u16()?;
    let prior = crate::EventSeq::new(cursor.integer()?);
    version(&mut cursor, "component")?;
    let digest =
        <[u8; 32]>::try_from(cursor.take(32)?).map_err(|_| CarrierError::Malformed("digest"))?;
    let header = ContinuationCarrierHeader::new(run, own, attempt, prior, digest)?;
    let length = usize::try_from(cursor.integer()?).map_err(|_| Error::Overflow)?;
    if length > limit {
        return Err(Error::Bound("opaque_bytes").into());
    }
    let component = cursor.take(length)?;
    cursor.end()?;
    Ok(OwnedContinuationCarrierView { header, component })
}

fn version(cursor: &mut Cursor<'_>, field: &'static str) -> Result<(), CarrierError> {
    let version = cursor.u16()?;
    if version != 1 {
        return Err(CarrierError::Version { field, version });
    }
    Ok(())
}
