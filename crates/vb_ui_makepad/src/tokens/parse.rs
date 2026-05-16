#![forbid(unsafe_code)]

use crate::error::Error;

pub const TOKENS_TOML: &str = include_str!("../../../../design/tokens/velvet_ui_tokens.toml");

pub fn parse_hex(hex: &str) -> Result<[f32; 4], Error> {
    let hex = hex.trim();
    let hex = match hex.strip_prefix('#') {
        Some(stripped) => stripped,
        None => hex,
    };

    fn nybble(b: u8) -> Result<u8, Error> {
        match b {
            b'0' => Ok(0),
            b'1' => Ok(1),
            b'2' => Ok(2),
            b'3' => Ok(3),
            b'4' => Ok(4),
            b'5' => Ok(5),
            b'6' => Ok(6),
            b'7' => Ok(7),
            b'8' => Ok(8),
            b'9' => Ok(9),
            b'A' | b'a' => Ok(10),
            b'B' | b'b' => Ok(11),
            b'C' | b'c' => Ok(12),
            b'D' | b'd' => Ok(13),
            b'E' | b'e' => Ok(14),
            b'F' | b'f' => Ok(15),
            _ => Err(Error::TokenParseError("invalid hex char".into())),
        }
    }

    let bytes = hex.as_bytes();
    let len = bytes.len();

    fn parse_pair(b0: u8, b1: u8) -> Result<u8, Error> {
        let hi = nybble(b0)?;
        let lo = nybble(b1)?;
        hi.checked_mul(16)
            .and_then(|scaled| scaled.checked_add(lo))
            .ok_or_else(|| Error::TokenParseError("invalid hex pair".into()))
    }

    fn parse_pair_at(bytes: &[u8], offset: usize, label: &str) -> Result<u8, Error> {
        let next_offset = offset.saturating_add(1);
        match (bytes.get(offset), bytes.get(next_offset)) {
            (Some(first), Some(second)) => parse_pair(*first, *second)
                .map_err(|_| Error::TokenParseError(format!("invalid hex {label}"))),
            _ => Err(Error::TokenParseError("hex too short".into())),
        }
    }

    if len == 6 {
        let r = parse_pair_at(bytes, 0, "r")?;
        let g = parse_pair_at(bytes, 2, "g")?;
        let b = parse_pair_at(bytes, 4, "b")?;
        Ok([
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        ])
    } else if len == 8 {
        let r = parse_pair_at(bytes, 0, "r")?;
        let g = parse_pair_at(bytes, 2, "g")?;
        let b = parse_pair_at(bytes, 4, "b")?;
        let a = parse_pair_at(bytes, 6, "a")?;
        Ok([
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        ])
    } else {
        Err(Error::TokenParseError(format!("invalid hex length: {len}")))
    }
}

pub struct Tokens;

impl Tokens {
    pub fn parse() -> Result<super::sections::ParsedTokens, Error> {
        super::sections::ParsedTokens::from_toml(TOKENS_TOML)
    }
}
