#![forbid(unsafe_code)]
//! PO-008: Kani harness for DiagnosticCode::from_str backward compatibility.
//!
//! Binds directly to the production parser
//! (`core::str::FromStr for crate::diagnostic::DiagnosticCode`) and the
//! production `CODE_REGISTRY`. No mirrored parser, error type, or range
//! table lives here: the harnesses execute the production `from_str`
//! (char prefix check, hex packing with checked arithmetic, registry
//! gate) so a passing proof constrains production behavior.
//!
//! Bound: Production registry size (unwind=300); full-u16 symbolic
//! property (unwind=300).

use core::str::FromStr as _;

use crate::diagnostic::{
    CODE_REGISTRY, DiagnosticCode, DiagnosticCodeParseError, is_registered_numeric,
};

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

    /// PO-008 H1: Every production registry numeric code parses via
    /// production `from_str` and round-trips to the same value.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_from_str_backward_compat() {
        for i in 0..CODE_REGISTRY.len() {
            let code = CODE_REGISTRY[i].numeric;
            let e_str = format_e_code(code);
            let result = DiagnosticCode::from_str(&e_str);
            match result {
                Ok(parsed) => {
                    assert_eq!(
                        parsed.code(),
                        code,
                        "Parsed code must match the registry numeric value"
                    );
                }
                Err(_) => {
                    // If the code is registered, production from_str must succeed.
                    if is_registered_numeric(code) {
                        kani::assert(false, "registered code rejected by production from_str");
                    }
                }
            }
        }
    }

    /// PO-008 H2 (binding): over all `u16` inputs, production `from_str`
    /// succeeds exactly for registered codes, and the parsed value equals
    /// the input. This binds the whole production parse path — prefix
    /// check, hex digit packing, and registry gate — to the registry.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_from_str_agrees_with_registry() {
        let raw: u16 = kani::any();
        let e_str = format_e_code(raw);
        let result = DiagnosticCode::from_str(&e_str);
        if is_registered_numeric(raw) {
            match result {
                Ok(parsed) => {
                    assert_eq!(parsed.code(), raw, "Registered code must parse to itself");
                }
                Err(_) => {
                    kani::assert(false, "registered code rejected by production from_str");
                }
            }
        } else {
            assert_eq!(
                result,
                Err(DiagnosticCodeParseError::UnsupportedCode),
                "Unregistered code {:04X} must be UnsupportedCode",
                raw
            );
        }
    }

    /// PO-008 H3: Verified gap anchors stay rejected with the production
    /// `UnsupportedCode` error. Each value below is confirmed absent from
    /// the production registry (see bead contract); H2 above proves the
    /// general case over all `u16`.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_from_str_rejects_unsupported() {
        let unsupported = [
            0x0000u16, 0x0100, 0x010C, 0x0200, 0x0300, 0x030A, 0x0400, 0x040D, 0x0500, 0x0600,
            0x0604, 0x0900, 0x0F00, 0x1000, 0x1007, 0x1010, 0x1016, 0x1100, 0x1106, 0x1200, 0x1204,
            0x1300, 0x130E, 0x1310, 0x1400, 0x2000, 0x3000, 0x300E, 0x4000, 0xFFFF,
        ];
        for code in unsupported.iter() {
            // Each anchor must genuinely be unregistered, or the anchor
            // itself has drifted and must be refreshed.
            assert!(
                !is_registered_numeric(*code),
                "Anchor {:04X} became registered; refresh the anchor list",
                code
            );
            let e_str = format_e_code(*code);
            let result = DiagnosticCode::from_str(&e_str);
            assert_eq!(
                result,
                Err(DiagnosticCodeParseError::UnsupportedCode),
                "Code {:04X} should be unsupported",
                code
            );
        }
    }
}
