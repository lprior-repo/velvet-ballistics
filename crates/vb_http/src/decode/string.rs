use super::limits::MAX_INPUT_BYTES;
use super::parser::Parser;
use super::DecodeError;

impl Parser<'_, '_> {
    pub(super) fn string(&mut self) -> Result<bool, DecodeError> {
        self.expect_byte(b'"')?;
        let mut decoded = 0_usize;
        let mut matches = true;
        for _ in 0..MAX_INPUT_BYTES {
            match self.take_byte()? {
                b'"' => return Ok(matches && decoded == self.field.len()),
                b'\\' => {
                    let scalar = self.string_escape()?;
                    let mut scratch = [0_u8; 4];
                    for byte in scalar.encode_utf8(&mut scratch).as_bytes().iter().take(4) {
                        self.string_byte(*byte, &mut decoded, &mut matches)?;
                    }
                }
                0x00..=0x1f => return Err(DecodeError::InvalidJson),
                // The caller validates the complete body as UTF-8 before scanning.
                byte => self.string_byte(byte, &mut decoded, &mut matches)?,
            }
        }
        Err(DecodeError::InvalidJson)
    }

    fn string_byte(
        &self,
        byte: u8,
        decoded: &mut usize,
        matches: &mut bool,
    ) -> Result<(), DecodeError> {
        let next = (*decoded).checked_add(1).ok_or(DecodeError::StringLimit)?;
        if next > self.limits.string_bytes {
            return Err(DecodeError::StringLimit);
        }
        if self.field.as_bytes().get(*decoded).copied() != Some(byte) {
            *matches = false;
        }
        *decoded = next;
        Ok(())
    }

    fn string_escape(&mut self) -> Result<char, DecodeError> {
        match self.take_byte()? {
            b'"' => Ok('"'),
            b'\\' => Ok('\\'),
            b'/' => Ok('/'),
            b'b' => Ok('\u{0008}'),
            b'f' => Ok('\u{000c}'),
            b'n' => Ok('\n'),
            b'r' => Ok('\r'),
            b't' => Ok('\t'),
            b'u' => self.string_unicode_escape(),
            _ => Err(DecodeError::InvalidJson),
        }
    }

    fn string_unicode_escape(&mut self) -> Result<char, DecodeError> {
        let high = self.string_hex_unit()?;
        let scalar = match high {
            0xd800..=0xdbff => {
                self.expect_byte(b'\\')?;
                self.expect_byte(b'u')?;
                let low = self.string_hex_unit()?;
                if !(0xdc00..=0xdfff).contains(&low) {
                    return Err(DecodeError::InvalidJson);
                }
                let prefix = high
                    .checked_sub(0xd800)
                    .and_then(|value| value.checked_mul(0x400))
                    .ok_or(DecodeError::InvalidJson)?;
                let suffix = low.checked_sub(0xdc00).ok_or(DecodeError::InvalidJson)?;
                prefix
                    .checked_add(suffix)
                    .and_then(|value| value.checked_add(0x10000))
                    .ok_or(DecodeError::InvalidJson)?
            }
            0xdc00..=0xdfff => return Err(DecodeError::InvalidJson),
            _ => high,
        };
        char::from_u32(scalar).ok_or(DecodeError::InvalidJson)
    }

    fn string_hex_unit(&mut self) -> Result<u32, DecodeError> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let byte = self.take_byte()?;
            let (base, offset): (u8, u32) = match byte {
                b'0'..=b'9' => (b'0', 0),
                b'a'..=b'f' => (b'a', 10),
                b'A'..=b'F' => (b'A', 10),
                _ => return Err(DecodeError::InvalidJson),
            };
            let digit = u32::from(byte)
                .checked_sub(u32::from(base))
                .and_then(|number| number.checked_add(offset))
                .ok_or(DecodeError::InvalidJson)?;
            value = value
                .checked_mul(16)
                .and_then(|number| number.checked_add(digit))
                .ok_or(DecodeError::InvalidJson)?;
        }
        Ok(value)
    }
}
