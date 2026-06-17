#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::panic
)]
//! Property test: Section 16 master contract parity against CODE_REGISTRY.
//!
//! Verifies that all 36 Section 16 symbolic codes from the master contract
//! appear in CODE_REGISTRY with correct symbolic names and expected numeric ranges.

use vb_core::diagnostic::CODE_REGISTRY;

/// Golden data: the 36 Section 16 symbolic codes with their numeric ranges.
struct Section16Entry {
    symbolic: &'static str,
    /// Expected high byte of the numeric code (e.g., 0x01 for Schema).
    expected_high_byte: u8,
}

fn section16_entries() -> Vec<Section16Entry> {
    vec![
        // Schema: E01xx
        Section16Entry {
            symbolic: "DUPLICATE_KEY",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "FORBIDDEN_YAML_FEATURE",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "UNKNOWN_TOP_LEVEL_FIELD",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "UNKNOWN_STEP_FIELD",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "MISSING_REQUIRED_FIELD",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "INVALID_VERSION",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "INVALID_ID",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "RESERVED_ID",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "DUPLICATE_ID",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "MULTIPLE_STEP_PRIMITIVES",
            expected_high_byte: 0x01,
        },
        Section16Entry {
            symbolic: "MISSING_STEP_PRIMITIVE",
            expected_high_byte: 0x01,
        },
        // Reference: E02xx
        Section16Entry {
            symbolic: "UNKNOWN_REFERENCE",
            expected_high_byte: 0x02,
        },
        Section16Entry {
            symbolic: "FUTURE_REFERENCE",
            expected_high_byte: 0x02,
        },
        Section16Entry {
            symbolic: "SECRET_NOT_DECLARED",
            expected_high_byte: 0x02,
        },
        Section16Entry {
            symbolic: "DIRECT_RUNTIME_REFERENCE",
            expected_high_byte: 0x02,
        },
        // Control Flow: E03xx
        Section16Entry {
            symbolic: "INVALID_THEN_TARGET",
            expected_high_byte: 0x03,
        },
        Section16Entry {
            symbolic: "CONTROL_FLOW_CYCLE",
            expected_high_byte: 0x03,
        },
        Section16Entry {
            symbolic: "UNREACHABLE_STEP",
            expected_high_byte: 0x03,
        },
        Section16Entry {
            symbolic: "INVALID_CHOOSE",
            expected_high_byte: 0x03,
        },
        Section16Entry {
            symbolic: "INVALID_FOR_EACH",
            expected_high_byte: 0x03,
        },
        Section16Entry {
            symbolic: "INVALID_TOGETHER",
            expected_high_byte: 0x03,
        },
        Section16Entry {
            symbolic: "INVALID_COLLECT",
            expected_high_byte: 0x03,
        },
        Section16Entry {
            symbolic: "INVALID_REDUCE",
            expected_high_byte: 0x03,
        },
        Section16Entry {
            symbolic: "INVALID_REPEAT",
            expected_high_byte: 0x03,
        },
        // Type/Taint: E04xx
        Section16Entry {
            symbolic: "INVALID_WAIT",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "INVALID_ASK",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "INVALID_FINISH",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "INVALID_RETRY",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "INVALID_ON_ERROR",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "SECRET_RESULT_LEAK",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "TYPE_MISMATCH",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "PAYLOAD_TOO_LARGE",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "LIMIT_REQUIRED",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "LIMIT_EXCEEDED",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "UNSUPPORTED_TRIGGER",
            expected_high_byte: 0x04,
        },
        Section16Entry {
            symbolic: "HTTP_TRIGGER_OUT_OF_CORE",
            expected_high_byte: 0x04,
        },
    ]
}

#[test]
fn section16_all_36_entries_present_in_code_registry() {
    let entries = section16_entries();
    assert_eq!(
        entries.len(),
        36,
        "golden data must contain 36 Section 16 entries"
    );

    for entry in &entries {
        let found = CODE_REGISTRY.iter().any(|e| e.symbolic == entry.symbolic);
        assert!(
            found,
            "CODE_REGISTRY must contain Section 16 code: '{}'",
            entry.symbolic
        );
    }
}

#[test]
fn section16_entries_have_correct_high_byte() {
    for entry in &section16_entries() {
        let registry_entries: Vec<_> = CODE_REGISTRY
            .iter()
            .filter(|e| e.symbolic == entry.symbolic)
            .collect();
        assert!(
            !registry_entries.is_empty(),
            "CODE_REGISTRY must have entry for '{}'",
            entry.symbolic
        );

        // At least one entry for this symbolic name must have the expected high byte.
        let has_correct_high = registry_entries
            .iter()
            .any(|e| (e.numeric >> 8) & 0xFF == entry.expected_high_byte as u16);
        assert!(
            has_correct_high,
            "Section 16 code '{}' must have an entry with high byte 0x{:02X}",
            entry.symbolic, entry.expected_high_byte
        );
    }
}
