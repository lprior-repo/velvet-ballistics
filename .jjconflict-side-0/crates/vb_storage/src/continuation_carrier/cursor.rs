use super::CarrierError;

pub(super) struct Cursor<'a> {
    bytes: &'a [u8],
}

impl<'a> Cursor<'a> {
    pub(super) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8], CarrierError> {
        let bytes = self
            .bytes
            .get(..n)
            .ok_or(CarrierError::Malformed("truncated"))?;
        self.bytes = self
            .bytes
            .get(n..)
            .ok_or(CarrierError::Malformed("truncated"))?;
        Ok(bytes)
    }

    pub(super) fn integer(&mut self) -> Result<u64, CarrierError> {
        let mut value = 0_u64;
        for shift in (0..70).step_by(7) {
            let byte = *self
                .take(1)?
                .first()
                .ok_or(CarrierError::Malformed("integer"))?;
            if shift == 63 && byte > 1 {
                return Err(CarrierError::Malformed("integer overflow"));
            }
            let part = u64::from(byte & 127)
                .checked_shl(shift)
                .ok_or(CarrierError::Malformed("integer overflow"))?;
            value |= part;
            if byte & 128 == 0 {
                if shift != 0 && byte == 0 {
                    return Err(CarrierError::Malformed("noncanonical integer"));
                }
                return Ok(value);
            }
        }
        Err(CarrierError::Malformed("integer overflow"))
    }

    pub(super) fn u16(&mut self) -> Result<u16, CarrierError> {
        u16::try_from(self.integer()?).map_err(|_| CarrierError::Malformed("u16 overflow"))
    }

    pub(super) fn end(&self) -> Result<(), CarrierError> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(CarrierError::Malformed("trailing bytes"))
        }
    }
}
