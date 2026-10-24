use super::{CarrierError, ContinuationCarrierHeader};
use arrayvec::ArrayVec;

pub(super) fn prefix(
    header: ContinuationCarrierHeader,
    length: usize,
) -> Result<ArrayVec<u8, 96>, CarrierError> {
    let mut out = ArrayVec::new();
    for n in [
        43,
        1,
        header.run().get(),
        header.own_seq().get(),
        u64::from(header.actual_attempt()),
        header.prior_seq().get(),
        1,
    ] {
        integer(&mut out, n)?;
    }
    for byte in header.component_digest() {
        out.try_push(byte)
            .map_err(|_| CarrierError::Malformed("prefix capacity"))?;
    }
    integer(
        &mut out,
        u64::try_from(length).map_err(|_| CarrierError::Malformed("length"))?,
    )?;
    Ok(out)
}

fn integer(out: &mut ArrayVec<u8, 96>, mut value: u64) -> Result<(), CarrierError> {
    for _ in 0..10 {
        let mut byte = u8::try_from(value & 127).map_err(|_| CarrierError::Malformed("integer"))?;
        value >>= 7;
        if value != 0 {
            byte |= 128;
        }
        out.try_push(byte)
            .map_err(|_| CarrierError::Malformed("prefix capacity"))?;
        if value == 0 {
            return Ok(());
        }
    }
    Err(CarrierError::Malformed("integer"))
}
