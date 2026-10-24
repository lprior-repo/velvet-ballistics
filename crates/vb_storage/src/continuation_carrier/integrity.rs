use super::CarrierError;

pub(super) fn component(bytes: &[u8], digest: [u8; 32]) -> Result<(), CarrierError> {
    if bytes.len() <= 32 {
        return Err(CarrierError::Malformed("component length"));
    }
    if bytes.first() != Some(&1) {
        return Err(CarrierError::Version {
            field: "inner component",
            version: u16::from(
                *bytes
                    .first()
                    .ok_or(CarrierError::Malformed("component version"))?,
            ),
        });
    }
    let end = bytes
        .len()
        .checked_sub(32)
        .ok_or(CarrierError::Malformed("component length"))?;
    let body = bytes
        .get(..end)
        .ok_or(CarrierError::Malformed("component body"))?;
    let tail = bytes
        .get(end..)
        .ok_or(CarrierError::Malformed("component digest"))?;
    if tail != digest || blake3::derive_key("velvet-ballistics/run-continuation/v1", body) != digest
    {
        return Err(CarrierError::Digest);
    }
    Ok(())
}
