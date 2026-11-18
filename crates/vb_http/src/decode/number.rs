use super::limits::MAX_INPUT_BYTES;
use super::parser::Parser;
use super::{DecodeError, IntegerField};

impl Parser<'_, '_> {
    pub(super) fn number(&mut self) -> Result<IntegerField, DecodeError> {
        let negative = self.eat_byte(b'-')?;
        let magnitude = match self.peek_byte() {
            Some(b'0') => {
                self.take_byte()?;
                if matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    return Err(DecodeError::InvalidJson);
                }
                Some(0)
            }
            Some(b'1'..=b'9') => self.digits()?,
            _ => return Err(DecodeError::InvalidJson),
        };
        let mut integer = true;
        if self.eat_byte(b'.')? {
            integer = false;
            let _fraction = self.digits()?;
        }
        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            integer = false;
            self.take_byte()?;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.take_byte()?;
            }
            let _exponent = self.digits()?;
        }
        if integer {
            Ok(IntegerField::Integer {
                negative,
                magnitude,
            })
        } else {
            Ok(IntegerField::InvalidType)
        }
    }

    fn digits(&mut self) -> Result<Option<u64>, DecodeError> {
        let start = self.position;
        let mut magnitude = Some(0_u64);
        for _ in 0..MAX_INPUT_BYTES {
            let Some(byte @ b'0'..=b'9') = self.peek_byte() else {
                break;
            };
            self.take_byte()?;
            let digit = u64::from(byte.checked_sub(b'0').ok_or(DecodeError::InvalidJson)?);
            magnitude = magnitude
                .and_then(|value| value.checked_mul(10))
                .and_then(|value| value.checked_add(digit));
        }
        if self.position == start {
            Err(DecodeError::InvalidJson)
        } else {
            Ok(magnitude)
        }
    }
}
