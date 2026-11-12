use super::CheckpointTransportError as Error;

/// No-allocation postcard cursor for checkpoint structural preflight.
pub struct CheckpointWireCursor<'a> {
    remaining: &'a [u8],
}

impl<'a> CheckpointWireCursor<'a> {
    /// Borrows bytes; no untrusted containers are deserialized.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    /// Remaining input, also used to bind complete private runtime witnesses.
    #[must_use]
    pub const fn remaining(&self) -> &'a [u8] {
        self.remaining
    }

    /// Takes an exact checked byte range.
    pub fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let value = self
            .remaining
            .get(..count)
            .ok_or(Error::Malformed("truncated"))?;
        self.remaining = self
            .remaining
            .get(count..)
            .ok_or(Error::Malformed("truncated"))?;
        Ok(value)
    }

    /// Reads a canonical unsigned postcard integer with its exact bit width.
    pub fn unsigned(&mut self, bits: u32) -> Result<u64, Error> {
        if bits == 0 || bits > 64 {
            return Err(Error::InvalidLimit);
        }
        let mut value = 0_u64;
        for index in 0_u32..10 {
            let byte = *self.take(1)?.first().ok_or(Error::Malformed("integer"))?;
            let shift = index.checked_mul(7).ok_or(Error::Overflow)?;
            let payload = u64::from(byte & 127);
            let shifted = payload.checked_shl(shift).ok_or(Error::Overflow)?;
            if shift >= bits || (shift == 63 && payload > 1) {
                return Err(Error::Malformed("integer_overflow"));
            }
            value |= shifted;
            if byte & 128 == 0 {
                if index > 0 && payload == 0 {
                    return Err(Error::Malformed("noncanonical_integer"));
                }
                if bits < 64 && value >= 1_u64.checked_shl(bits).ok_or(Error::Overflow)? {
                    return Err(Error::Malformed("integer_width"));
                }
                return Ok(value);
            }
        }
        Err(Error::Malformed("integer_overflow"))
    }

    /// Reads a bounded sequence length before any reservation.
    pub fn length(&mut self, limit: usize, field: &'static str) -> Result<usize, Error> {
        let count = usize::try_from(self.unsigned(64)?).map_err(|_| Error::Overflow)?;
        if count > limit {
            return Err(Error::Bound(field));
        }
        Ok(count)
    }

    /// Checks an exact known canonical sequence, without decoding its fields.
    pub fn matching(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self.take(bytes.len())? != bytes {
            return Err(Error::Malformed("authority_bytes"));
        }
        Ok(())
    }

    /// Checks a taint discriminant without allocating an error string.
    pub fn taint(&mut self) -> Result<(), Error> {
        if self.unsigned(32)? > 4 {
            return Err(Error::Malformed("taint"));
        }
        Ok(())
    }

    /// Checks the complete scalar/handle schema; values never own nested data.
    pub fn value(&mut self) -> Result<(), Error> {
        match self.unsigned(32)? {
            0 => Ok(()),
            1 => {
                if self.take(1)? > [1_u8].as_slice() {
                    return Err(Error::Malformed("bool"));
                }
                Ok(())
            }
            2 | 7 => self.unsigned(64).map(|_| ()),
            3 => {
                let bytes: [u8; 8] = self
                    .take(8)?
                    .try_into()
                    .map_err(|_| Error::Malformed("f64"))?;
                if !f64::from_le_bytes(bytes).is_finite() {
                    return Err(Error::Malformed("f64"));
                }
                Ok(())
            }
            4..=6 => self.unsigned(32).map(|_| ()),
            _ => Err(Error::Malformed("slot_value")),
        }
    }
}
