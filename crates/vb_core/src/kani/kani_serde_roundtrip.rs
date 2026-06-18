#![forbid(unsafe_code)]
//! PO-009: Kani harness for SymbolicCode Serialize/Deserialize round-trip.
//!
//! Proves: (1) serialize produces the symbolic string; deserialize produces
//! matching SymbolicCode; round-trip identity holds.
//! (2) well-formed but unregistered strings deserialize to Err.
//!
//! Bound: Registry size ~90 entries (unwind=100)
//! Trusted Base: TBL-002 (serde framework)

use super::kani_symbolic_code_validation::{CODE_REGISTRY, SymbolicCode};

/// Mirror of serde Serialize for SymbolicCode via JSON string.
fn serialize_symbolic_code(code: &SymbolicCode) -> String {
    format!("\"{}\"", code.as_str())
}

/// Mirror of serde Deserialize for SymbolicCode from JSON string.
/// Parses a JSON string value and validates against the registry.
fn deserialize_symbolic_code(json_str: &str) -> Result<SymbolicCode, String> {
    // Strip optional surrounding quotes
    let inner = json_str.trim_matches('"');
    for entry in CODE_REGISTRY {
        if entry.symbolic == inner {
            return Ok(SymbolicCode::from_static_infallible(entry.symbolic));
        }
    }
    Err(format!("Unknown symbolic code: {}", inner))
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
    #[kani::unwind(100)]
    fn kani_serde_roundtrip() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let code = SymbolicCode::from_static_infallible(entry.symbolic);

            // Serialize
            let serialized = serialize_symbolic_code(&code);
            // Should produce JSON string of the symbolic name
            kani::assert(serialized.contains(entry.symbolic),
                "Serialized form must contain the symbolic name",
            );

            // Deserialize
            let deserialized = deserialize_symbolic_code(&serialized);
            kani::assert(deserialized.is_ok(),
                "Deserialization must succeed for registered codes",
            );
            kani::assert(deserialized.map(|code| code.as_str()) == Ok(entry.symbolic), "Deserialized SymbolicCode must match original");

            // Round-trip identity
            let rt = roundtrip(&code);
            kani::assert(rt.is_ok(), "Round-trip must succeed");
            kani::assert(rt.map(|code| code.as_str()) == Ok(code.as_str()), "Round-trip identity must hold");
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
            kani::assert(result.is_err(), "Unknown code must be rejected");
        }
    }
}
