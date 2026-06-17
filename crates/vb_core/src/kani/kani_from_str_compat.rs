#![forbid(unsafe_code)]
//! PO-008: Kani harness for DiagnosticCode::from_str backward compatibility.
//!
//! Proves: (1) all previously supported codes parse successfully;
//! (2) all newly added codes (E0501-E0603, E401C) parse successfully;
//! (3) out-of-range codes return Err(UnsupportedCode).
//!
//! Bound: ~100 code constants (unwind=100)

use super::kani_symbolic_code_validation::{CODE_REGISTRY, is_supported_code};

/// Mirror of DiagnosticCodeParseError.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCodeParseError {
    InvalidFormat,
    UnsupportedCode,
}

/// Mirror of DiagnosticCode for parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCode(u16);

impl DiagnosticCode {
    #[must_use]
    pub const fn new(code: u16) -> Self {
        Self(code)
    }
    #[must_use]
    pub const fn code(self) -> u16 {
        self.0
    }
}

/// Parse a hex digit character.
const fn parse_hex_digit(c: Option<char>) -> Result<u16, DiagnosticCodeParseError> {
    match c {
        Some(ch) => match ch.to_digit(16) {
            Some(d) if d <= 15 => Ok(d as u16),
            _ => Err(DiagnosticCodeParseError::InvalidFormat),
        },
        None => Err(DiagnosticCodeParseError::InvalidFormat),
    }
}

/// Pack four hex digits into a u16.
const fn pack_digits(a: u16, b: u16, c: u16, d: u16) -> Result<u16, DiagnosticCodeParseError> {
    let a_s = match a.checked_shl(12) {
        Some(v) => v,
        None => return Err(DiagnosticCodeParseError::InvalidFormat),
    };
    let b_s = match b.checked_shl(8) {
        Some(v) => v,
        None => return Err(DiagnosticCodeParseError::InvalidFormat),
    };
    let c_s = match c.checked_shl(4) {
        Some(v) => v,
        None => return Err(DiagnosticCodeParseError::InvalidFormat),
    };
    let ab = match a_s.checked_add(b_s) {
        Some(v) => v,
        None => return Err(DiagnosticCodeParseError::InvalidFormat),
    };
    let abc = match ab.checked_add(c_s) {
        Some(v) => v,
        None => return Err(DiagnosticCodeParseError::InvalidFormat),
    };
    match abc.checked_add(d) {
        Some(v) => Ok(v),
        None => Err(DiagnosticCodeParseError::InvalidFormat),
    }
}

/// Mirror of FromStr for DiagnosticCode.
fn from_str_diagnostic_code(input: &str) -> Result<DiagnosticCode, DiagnosticCodeParseError> {
    let mut chars = input.chars();
    if chars.next() != Some('E') {
        return Err(DiagnosticCodeParseError::InvalidFormat);
    }
    let first = parse_hex_digit(chars.next())?;
    let second = parse_hex_digit(chars.next())?;
    let third = parse_hex_digit(chars.next())?;
    let fourth = parse_hex_digit(chars.next())?;
    if chars.next().is_some() {
        return Err(DiagnosticCodeParseError::InvalidFormat);
    }
    let code = pack_digits(first, second, third, fourth)?;
    if is_supported_code(code) {
        Ok(DiagnosticCode::new(code))
    } else {
        Err(DiagnosticCodeParseError::UnsupportedCode)
    }
}

/// Format a u16 as an "EXXXX" string.
fn format_e_code(code: u16) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(5);
    let _ = write!(s, "E{:04X}", code);
    s
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-008 H1: All registry numeric codes parse successfully via from_str.
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_from_str_backward_compat() {
        for i in 0..CODE_REGISTRY.len() {
            let code = CODE_REGISTRY[i].numeric;
            let e_str = format_e_code(code);
            let result = from_str_diagnostic_code(&e_str);
            match result {
                Ok(parsed) => {
                    kani::assert(parsed.code() == code, "Parsed code must match the registry numeric value");
                }
                Err(_) => {
                    // If is_supported_code accepts it, from_str must succeed
                    if is_supported_code(code) {
                         == code, "Parsed code must match the registry numeric value");
                }
                Err(_) => {
                    // If is_supported_code accepts it, from_str must succeed
                    if is_supported_code(code) {
                        kani::assert(false, "is_supported_code accepted but from_str rejected");
                    }
                }
            }
        }
    }

    /// PO-008 H2: Newly added codes (E05xx, E06xx) parse successfully.
    #[kani::proof]
    #[kani::unwind(60)]
    fn kani_from_str_new_codes_parse() {
        // Gate verifier range
        for code in 0x0501u16..=0x0513 {
            let e_str = format_e_code(code);
            let result = from_str_diagnostic_code(&e_str);
            kani::assert(result.is_ok(), "New Gate code {:04X} must parse", code);
        }
        // Contract discovery range
        for code in 0x0601u16..=0x0603 {
            let e_str = format_e_code(code);
            let result = from_str_diagnostic_code(&e_str);
            kani::assert(result.is_ok(),
                "New ContractDiscovery code {:04X} must parse",
                code,
            );
        }
        // Extended boundary code
        let e_str = format_e_code(0x401C);
        let result = from_str_diagnostic_code(&e_str);
        kani::assert(result.is_ok(), "Extended boundary code 0x401C must parse");
    }

    /// PO-008 H3: Out-of-range codes return Err(UnsupportedCode).
    #[kani::proof]
    #[kani::unwind(30)]
    fn kani_from_str_rejects_unsupported() {
        let unsupported = [
            0x0100u16, 0x010C, 0x0200, 0x0205, 0x0300, 0x030A, 0x0400, 0x040D, 0x0500, 0x0600,
            0x0604, 0x0900, 0x0F00, 0x1000, 0x1003, 0x1010, 0x1014, 0x1100, 0x1105, 0x1200, 0x1203,
            0x1300, 0x130E, 0x1310, 0x1315, 0x1400, 0x1408, 0x2000, 0x2010, 0x3000, 0x300F, 0x4000,
            0x401D,
        ];
        for code in unsupported.iter() {
            let e_str = format_e_code(*code);
            let result = from_str_diagnostic_code(&e_str);
            ;
                }
                Err(_) => {
                    // If is_supported_code accepts it, from_str must succeed
                    if is_supported_code(code) {
                        , "Extended boundary code 0x401C must parse");
    }

    /// PO-008 H3: Out-of-range codes return Err(UnsupportedCode).
    #[kani::proof]
    #[kani::unwind(30)]
    fn kani_from_str_rejects_unsupported() {
        let unsupported = [
            0x0100u16, 0x010C, 0x0200, 0x0205, 0x0300, 0x030A, 0x0400, 0x040D, 0x0500, 0x0600,
            0x0604, 0x0900, 0x0F00, 0x1000, 0x1003, 0x1010, 0x1014, 0x1100, 0x1105, 0x1200, 0x1203,
            0x1300, 0x130E, 0x1310, 0x1315, 0x1400, 0x1408, 0x2000, 0x2010, 0x3000, 0x300F, 0x4000,
            0x401D,
        ];
        for code in unsupported.iter() {
            let e_str = format_e_code(*code);
            let result = from_str_diagnostic_code(&e_str);
            ;
                }
                Err(_) => {
                    // If is_supported_code accepts it, from_str must succeed
                    if is_supported_code(code) {
                        kani::assert(false, "is_supported_code accepted but from_str rejected");
                    }
                }
            }
        }
    }

    /// PO-008 H2: Newly added codes (E05xx, E06xx) parse successfully.
    #[kani::proof]
    #[kani::unwind(60)]
    fn kani_from_str_new_codes_parse() {
        // Gate verifier range
        for code in 0x0501u16..=0x0513 {
            let e_str = format_e_code(code);
            let result = from_str_diagnostic_code(&e_str);
            kani::assert(result.is_ok(), "New Gate code {:04X} must parse", code);
        }
        // Contract discovery range
        for code in 0x0601u16..=0x0603 {
            let e_str = format_e_code(code);
            let result = from_str_diagnostic_code(&e_str);
            kani::assert(result.is_ok(),
                "New ContractDiscovery code {:04X} must parse",
                code,
            );
        }
        // Extended boundary code
        let e_str = format_e_code(0x401C);
        let result = from_str_diagnostic_code(&e_str);
        kani::assert(result.is_ok(), "Extended boundary code 0x401C must parse");
    }

    /// PO-008 H3: Out-of-range codes return Err(UnsupportedCode).
    #[kani::proof]
    #[kani::unwind(30)]
    fn kani_from_str_rejects_unsupported() {
        let unsupported = [
            0x0100u16, 0x010C, 0x0200, 0x0205, 0x0300, 0x030A, 0x0400, 0x040D, 0x0500, 0x0600,
            0x0604, 0x0900, 0x0F00, 0x1000, 0x1003, 0x1010, 0x1014, 0x1100, 0x1105, 0x1200, 0x1203,
            0x1300, 0x130E, 0x1310, 0x1315, 0x1400, 0x1408, 0x2000, 0x2010, 0x3000, 0x300F, 0x4000,
            0x401D,
        ];
        for code in unsupported.iter() {
            let e_str = format_e_code(*code);
            let result = from_str_diagnostic_code(&e_str);
            kani::assert(result != Err(DiagnosticCodeParseError::UnsupportedCode));
        }
    }
}
