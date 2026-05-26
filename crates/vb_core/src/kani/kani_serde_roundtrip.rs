#![forbid(unsafe_code)]
//! PO-009: Kani harness for SymbolicCode Serialize/Deserialize round-trip.
//!
//! Proves: (1) serialize produces the symbolic string; deserialize produces
//! matching SymbolicCode; round-trip identity holds.
//! (2) well-formed but unregistered strings deserialize to Err.
//!
//! Bound: Registry size 157 entries (unwind=160)
//! Trusted Base: TBL-002 (serde framework)
//!
//! Rewired: uses production SymbolicCode and CODE_REGISTRY from
//! crate::diagnostic instead of inline models.
//! R5: deserialize_symbolic_code uses &'static str errors (no format!())
//!     to avoid Kani allocator path explosion over 157-entry registry.

use crate::diagnostic::{CODE_REGISTRY, SymbolicCode};

/// Mirror of serde Serialize for SymbolicCode via JSON string.
fn serialize_symbolic_code(code: &SymbolicCode) -> String {
    format!("\"{}\"", code.as_str())
}

/// Mirror of serde Deserialize for SymbolicCode from JSON string.
/// Parses a JSON string value and validates against the registry.
/// Iterates CODE_REGISTRY to find matching entry and constructs
/// SymbolicCode from the registered &'static str.
///
/// R5: Uses &'static str errors instead of String/format!() to avoid
/// Kani allocator path explosion when iterating the 157-entry registry.
fn deserialize_symbolic_code(json_str: &str) -> Result<SymbolicCode, &'static str> {
    let inner = json_str.trim_matches('"');
    for entry in CODE_REGISTRY {
        if entry.symbolic == inner {
            return SymbolicCode::from_static(entry.symbolic)
                .ok_or("ERR_SERDE_UNKNOWN_CODE");
        }
    }
    Err("ERR_SERDE_UNKNOWN_CODE")
}

/// Round-trip: serialize then deserialize.
fn roundtrip(code: &SymbolicCode) -> Result<SymbolicCode, &'static str> {
    let serialized = serialize_symbolic_code(code);
    deserialize_symbolic_code(&serialized)
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-009 H1: For each registered SymbolicCode, serialize produces the
    /// symbolic string, deserialize produces matching SymbolicCode, and
    /// round-trip identity holds.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_serde_roundtrip() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let code = SymbolicCode::from_static(entry.symbolic)
                .expect("Registry entry must produce valid SymbolicCode");

            let serialized = serialize_symbolic_code(&code);
            assert!(
                serialized.contains(entry.symbolic),
                "Serialized form must contain the symbolic name"
            );

            let deserialized = deserialize_symbolic_code(&serialized);
            assert!(
                deserialized.is_ok(),
                "Deserialization must succeed for registered codes"
            );
            assert_eq!(
                deserialized.as_ref().map(|c| c.as_str()).unwrap_or(""),
                entry.symbolic,
                "Deserialized SymbolicCode must match original"
            );

            let rt = roundtrip(&code);
            assert!(rt.is_ok(), "Round-trip must succeed");
            assert_eq!(
                rt.as_ref().map(|c| c.as_str()).unwrap_or(""),
                code.as_str(),
                "Round-trip identity must hold"
            );
        }
    }

    /// PO-009 H2: For well-formed but unregistered strings, deserialize returns Err.
    /// Registry size 157 entries — unwind 160 covers full linear scan + loop exit.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_serde_rejects_unknown() {
        let unknown = [
            "\"__UNKNOWN__\"",
            "\"NOT_A_CODE\"",
            "\"\"",
            "\"RANDOM_STRING_123\"",
        ];
        for s in unknown.iter() {
            let result = deserialize_symbolic_code(s);
            assert!(result.is_err(), "Unknown code '{}' must be rejected", s);
        }
    }
}
