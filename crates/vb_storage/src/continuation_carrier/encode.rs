use super::{
    BoundedContinuationRecord, CarrierError, ContinuationCarrierBudget, ContinuationCarrierHeader,
};
use vb_core::value_store::transport::CheckpointTransportError as Error;

/// Dedicated manual writer; ordinary Serde routes intentionally cannot emit43.
pub fn encode_owned_continuation(
    header: ContinuationCarrierHeader,
    component: &[u8],
    operation: &mut ContinuationCarrierBudget,
) -> Result<BoundedContinuationRecord, CarrierError> {
    operation.wire()?;
    if component.len() > operation.record_limit {
        return Err(Error::Bound("whole_record").into());
    }
    let prefix = super::prefix::prefix(header, component.len())?;
    let length = crate::RECORD_HEADER_BYTES
        .checked_add(prefix.len())
        .and_then(|n| n.checked_add(component.len()))
        .ok_or(Error::Overflow)?;
    if length > operation.record_limit {
        return Err(Error::Bound("whole_record").into());
    }
    super::integrity::component(component, header.component_digest())?;
    write_record(header, component, &prefix, length)
}

fn write_record(
    header: ContinuationCarrierHeader,
    component: &[u8],
    prefix: &[u8],
    length: usize,
) -> Result<BoundedContinuationRecord, CarrierError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| Error::Allocation)?;
    bytes.resize(crate::RECORD_HEADER_BYTES, 0);
    bytes.extend_from_slice(prefix);
    bytes.extend_from_slice(component);
    let payload = bytes
        .get(crate::RECORD_HEADER_BYTES..)
        .ok_or(Error::Overflow)?;
    let envelope = crate::codec::encode_record_header(
        crate::MAGIC_JOURNAL_EVENT,
        crate::RecordKind::RunContinuation,
        header.own_seq().get(),
        payload,
        crate::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    bytes
        .get_mut(..crate::RECORD_HEADER_BYTES)
        .ok_or(Error::Overflow)?
        .copy_from_slice(&envelope);
    Ok(BoundedContinuationRecord { bytes })
}
