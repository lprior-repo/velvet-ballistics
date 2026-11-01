#![forbid(unsafe_code)]
//! PO-009: Kani harness for SymbolicCode Serialize/Deserialize round-trip.
//!
//! Binds to the production `SymbolicCode` type, the production
//! registry, and production `SymbolicCode::from_str` — which applies
//! the same registry validation as the production `Deserialize`
//! impl. The previous hand-rolled registry scan inside the test
//! deserializer is deleted; only the JSON string framing remains,
//! because `serde_json` is a dev-dependency and is not linked into
//! the kani build. Full serde-derive paths are covered by the
//! runtime unification tests (`serde_json` round-trip).
//!
//! Bound: Production registry size (unwind=300)
//! Trusted Base: TBL-002 (serde framework)

use core::str::FromStr as _;

use crate::diagnostic::{CODE_REGISTRY, SymbolicCode};

/// Mirror of serde Serialize for SymbolicCode via JSON string.
fn serialize_symbolic_code(code: &SymbolicCode) -> String {
    format!("\"{}\"", code.as_str())
}

/// Validate a JSON string value against the production registry via
/// the production `FromStr` impl (same registry validation as
/// production `Deserialize`).
fn deserialize_symbolic_code(json_str: &str) -> Result<SymbolicCode, String> {
    // Strip optional surrounding quotes
    let inner = json_str.trim_matches('"');
    SymbolicCode::from_str(inner).map_err(|_| format!("Unknown symbolic code: {inner}"))
}

/// Round-trip: serialize then deserialize.
fn roundtrip(code: &SymbolicCode) -> Result<SymbolicCode, String> {
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
    #[kani::unwind(300)]
    fn kani_serde_roundtrip() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            if let Some(code) = SymbolicCode::from_static(entry.symbolic) {
                // Serialize
                let serialized = serialize_symbolic_code(&code);
                // Should produce JSON string of the symbolic name
                assert!(
                    serialized.contains(entry.symbolic),
                    "Serialized form must contain the symbolic name"
                );

                // Deserialize
                let deserialized = deserialize_symbolic_code(&serialized);
                assert!(
                    deserialized.is_ok(),
                    "Deserialization must succeed for registered codes"
                );
                assert_eq!(
                    deserialized.map(|code| code.as_str()),
                    Ok(entry.symbolic),
                    "Deserialized SymbolicCode must match original"
                );

                // Round-trip identity
                let rt = roundtrip(&code);
                assert!(rt.is_ok(), "Round-trip must succeed");
                assert_eq!(
                    rt.map(|code| code.as_str()),
                    Ok(code.as_str()),
                    "Round-trip identity must hold"
                );
            } else {
                kani::assert(false, "registered symbolic must construct");
            }
        }
    }

    /// PO-009 H2: For well-formed but unregistered strings, deserialize returns Err.
    #[kani::proof]
    #[kani::unwind(50)]
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
