#![forbid(unsafe_code)]
//! PO-KANI-004: Kani harness verifying that `parse_yaml_events` never
//! panics for valid UTF-8 inputs within bounded size. Returns
//! `Ok(events)` or `Err(YamlError)` — no `unwrap`, `expect`, `panic`,
//! or unreachable branches.
//!
//! Proves: For bounded UTF-8 inputs (≤ 256 bytes for Kani tractability),
//! `parse_yaml_events` completes without panic. All error paths produce
//! typed `YamlError` values.
//!
//! GOD RULE 1 compliance: Input bytes are generated via `kani::any()`,
//! constrained to valid UTF-8 via `std::str::from_utf8`. No hardcoded
//! YAML strings.
//!
//! Bounds:
//! - Input size: ≤ 256 bytes (Kani tractability bound)
//! - Max depth: 64 (from YamlLimits::default)
//! - Max nodes: 100,000 (from YamlLimits::default)

use crate::{parse_yaml_events, validate_yaml_profile};
use vb_core::diagnostic::HasSymbolicCode;

/// Bounded input size for Kani tractability.
const MAX_KANI_INPUT: usize = 256;

/// Generate an arbitrary UTF-8 byte slice up to MAX_KANI_INPUT bytes.
///
/// Kani generates symbolic bytes; we constrain them to valid UTF-8
/// using `std::str::from_utf8`. For this harness, we only test inputs
/// that are actually valid UTF-8 (binary non-UTF-8 inputs are exercised
/// by the cargo-fuzz and proptest lanes).
fn arbitrary_utf8_input() -> Vec<u8> {
    let len: usize = kani::any();
    kani::assume(len <= MAX_KANI_INPUT);

    let mut bytes = Vec::with_capacity(len);
    for _ in 0..len {
        bytes.push(kani::any::<u8>());
    }
    bytes
}

/// PO-KANI-004: `parse_yaml_events` never panics on any valid UTF-8
/// input up to MAX_KANI_INPUT bytes.
#[kani::proof]
#[kani::unwind(256)]
fn check_parse_yaml_events_panic_free() {
    let data = arbitrary_utf8_input();

    // Only test valid UTF-8 inputs.
    if let Ok(text) = std::str::from_utf8(&data) {
        // parse_yaml_events must return a Result, never panic.
        let result = parse_yaml_events(text);

        match result {
            Ok(events) => {
                // On success, events must be a valid Vec.
                // Event count must be bounded (≤ max_nodes from YamlLimits).
                assert!(
                    events.len() <= 100_000,
                    "event count {} exceeds max_nodes bound",
                    events.len()
                );
            }
            Err(error) => {
                // On error, must be a typed YamlError.
                // Verify the error has a registered symbolic code.
                let code = error.symbolic_code();
                let _name = code.as_str();
                // Code must not be the sentinel.
                assert_ne!(
                    code,
                    vb_core::diagnostic::SymbolicCode::INTERNAL_INVARIANT,
                    "YamlError must not use INTERNAL_INVARIANT sentinel"
                );
            }
        }
    }
    // Non-UTF-8 input is simply ignored (not a panic path).
}

/// PO-KANI-004 (extended): `validate_yaml_profile` never panics.
#[kani::proof]
#[kani::unwind(256)]
fn check_validate_yaml_profile_panic_free() {
    let data = arbitrary_utf8_input();

    if let Ok(text) = std::str::from_utf8(&data) {
        let result = validate_yaml_profile(text);

        match result {
            Ok(()) => {
                // Profile validation passed.
            }
            Err(error) => {
                // Must be a registered symbolic code.
                let code = error.symbolic_code();
                assert_ne!(code, vb_core::diagnostic::SymbolicCode::INTERNAL_INVARIANT);
            }
        }
    }
}
