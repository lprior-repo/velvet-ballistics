#![forbid(unsafe_code)]

//! Behavior tests for `fuzz_yaml_events` harness logic and `assert_typed_yaml_error`.
//!
//! Test layers:
//! - U1-U6: Harness logic (public behavior of fuzz_yaml_events)
//! - BT7-BT10: Harness boundary behavior
//! - E1-E22: Error variant classification (assert_typed_yaml_error exhaustiveness)
//! - BT1-BT6: Boundary tests (fuzz gate: UTF-8, empty, non-UTF-8, etc.)

use fuzz_lib::test_exports::{assert_typed_yaml_error, fuzz_yaml_events, MAX_FUZZ_PAYLOAD};
use vb_yaml::YamlError;

// ============================================================================
// U1-U6: Harness Logic Tests — fuzz_yaml_events public behavior
// ============================================================================

#[test]
fn harness_does_not_panic_on_valid_minimal_workflow() {
    // Given: a valid, minimal velvet-ballistics workflow
    let yaml = "version: v1\nname: test\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    input: 0\n";
    // When: fuzz_yaml_events is called with valid UTF-8
    // Then: must not panic (the function returns (), no return value to check)
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_single_line_scalar() {
    // Given: a YAML scalar on its own line
    let yaml = "hello\n";
    // When: fuzz_yaml_events is called
    // Then: no panic — empty profiles are acceptable
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_empty_string() {
    // Given: empty input
    let yaml = "";
    // When: fuzz_yaml_events is called
    // Then: no panic — empty input is valid UTF-8
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_returns_early_on_non_utf8_input() {
    // Given: invalid UTF-8 bytes
    let data: &[u8] = &[0xFF, 0xFE, 0xFD];
    // When: fuzz_yaml_events is called
    // Then: must return early without panic (non-UTF-8 is silently skipped)
    fuzz_yaml_events(data);
}

#[test]
fn harness_handles_maximum_valid_payload_boundary() {
    // Given: input at exactly MAX_FUZZ_PAYLOAD bytes
    let payload = "a".repeat(MAX_FUZZ_PAYLOAD as usize);
    // When: fuzz_yaml_events is called
    // Then: no panic — large payloads must not OOM or overflow
    fuzz_yaml_events(payload.as_bytes());
}

#[test]
fn harness_handles_yaml_with_only_whitespace() {
    // Given: whitespace-only input
    let yaml = "   \n  \t  \n   ";
    // When: fuzz_yaml_events is called
    // Then: no panic — whitespace is valid UTF-8, profile may or may not reject it
    fuzz_yaml_events(yaml.as_bytes());
}

// ============================================================================
// BT7-BT10: Harness Boundary Behavior Tests
// ============================================================================

#[test]
fn harness_does_not_panic_on_deeply_nested_yaml() {
    // Given: deeply nested YAML (100 levels)
    let mut yaml = String::new();
    for i in 0..100 {
        for _ in 0..i {
            yaml.push(' ');
        }
        yaml.push_str("key: value\n");
    }
    // When: fuzz_yaml_events is called
    // Then: no panic — nesting errors are returned as typed YamlError
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_source_map_with_max_entries() {
    // Given: YAML that creates many source map entries but stays under MAX_FUZZ_PAYLOAD
    let mut yaml = String::from("items:\n");
    for i in 0..500 {
        yaml.push_str(&format!("  - item_{i}: {i}\n"));
    }
    // When: fuzz_yaml_events is called
    // Then: no panic — event count must stay below MAX_FUZZ_PAYLOAD (4096)
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_very_long_scalar_value() {
    // Given: a scalar value at 64KB
    let scalar_content = "x".repeat(65536);
    let yaml = format!("key: {scalar_content}\n");
    // When: fuzz_yaml_events is called
    // Then: no panic — scalar length limit is enforced as typed error
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_very_long_key_name() {
    // Given: a key name at 64KB
    let key_content = "x".repeat(65536);
    let yaml = format!("{key_content}: value\n");
    // When: fuzz_yaml_events is called
    // Then: no panic
    fuzz_yaml_events(yaml.as_bytes());
}

// ============================================================================
// E1-E22: Error Classification Tests — assert_typed_yaml_error exhaustiveness
// ============================================================================

/// Helper: constructs a YamlError variant and asserts it is recognized by
/// assert_typed_yaml_error.  Every variant must be matched without panicking.
macro_rules! assert_classifies {
    ($expr:expr) => {
        assert_typed_yaml_error($expr);
    };
}

#[test]
fn classify_unsupported_trigger_variant() {
    // Given: UnsupportedTrigger variant
    let err = YamlError::UnsupportedTrigger { trigger: "cron" };
    // When: assert_typed_yaml_error is called
    // Then: must match without panic (or triggering the wildcard arm)
    assert_classifies!(err);
}

#[test]
fn classify_unsupported_feature_variant() {
    let err = YamlError::UnsupportedFeature {
        feature: "merge_keys",
    };
    assert_classifies!(err);
}

#[test]
fn classify_duplicate_key_variant() {
    let err = YamlError::DuplicateKey {
        key: "version".into(),
    };
    assert_classifies!(err);
}

#[test]
fn classify_anchor_alias_merge_variant() {
    let err = YamlError::AnchorAliasMerge;
    assert_classifies!(err);
}

#[test]
fn classify_custom_tag_variant() {
    let err = YamlError::CustomTag {
        tag: "!mytag".into(),
    };
    assert_classifies!(err);
}

#[test]
fn classify_binary_scalar_variant() {
    let err = YamlError::BinaryScalar;
    assert_classifies!(err);
}


// Remaining tests live in submodule
mod group_b;
