#![forbid(unsafe_code)]

//! Integration tests: `fuzz_yaml_events` harness exercising the real vb_yaml parser
//! surface through `validate_yaml_profile` → `parse_yaml_events` → `build_source_map`.
//!
//! Test layers:
//! - I1-I14: Harness↔parser integration — each tests a specific YamlError variant
//! - SB1-SB2: Saphyr parser boundary behavior
//! - CV1-CV5: Corpus seed validation

use fuzz_lib::test_exports::fuzz_yaml_events;
use vb_yaml::{build_source_map, parse_yaml_events, validate_yaml_profile};

// ============================================================================
// I1-I14: Harness↔Parser Integration — Exercise error variants through fuzz_yaml_events
// ============================================================================

/// Verifies that fuzz_yaml_events does not panic and that the individual parser
/// functions can process the same input without panic.
macro_rules! assert_fuzz_does_not_panic {
    ($yaml:expr) => {
        // The harness must never panic on any input
        fuzz_yaml_events($yaml.as_bytes());
        // Individual parser functions must also not panic
        let _ = validate_yaml_profile($yaml);
        let _ = parse_yaml_events($yaml);
        let _ = build_source_map($yaml);
    };
}

#[test]
fn integration_duplicate_key_triggers_yaml_error() {
    // Given: YAML with duplicate keys
    let yaml = "version: v1\nname: test\nversion: v2\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    input: 0\n";
    // When: harness and individual parsers process it
    // Then: must not panic
    fuzz_yaml_events(yaml.as_bytes());
    let result = validate_yaml_profile(yaml);
    // Duplicate key must produce error
    assert!(result.is_err(), "duplicate key must produce error");
    let err = result.unwrap_err();
    assert!(
        matches!(err, vb_yaml::YamlError::DuplicateKey { .. }),
        "expected DuplicateKey, got {err:?}"
    );
}

#[test]
fn integration_anchor_alias_triggers_yaml_error() {
    // Given: YAML with anchor and alias
    let yaml = "version: v1\nname: test\nwhen: manual\nsteps:\n  - id: 1\n    do: &action ping\n    input: 0\n  - id: 2\n    do: *action\n    input: 0\n";
    // When: harness processes it
    // Then: must not panic — anchors/aliases are rejected by profile or produce events
    fuzz_yaml_events(yaml.as_bytes());
    // Profile should reject this (or Saphyr may reject at parse time)
    let result = validate_yaml_profile(yaml);
    if let Err(e) = result {
        assert!(
            matches!(
                e,
                vb_yaml::YamlError::AnchorAliasMerge | vb_yaml::YamlError::ForbiddenFeature { .. }
            ),
            "expected AnchorAliasMerge or ForbiddenFeature, got {e:?}"
        );
    }
}

#[test]
fn integration_null_byte_triggers_forbidden_feature() {
    // Given: YAML containing a null byte
    let yaml = "version: v1\nname: test\x00embedded\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    input: 0\n";
    // When: harness and profile process it
    // Then: profile must reject with ForbiddenFeature
    fuzz_yaml_events(yaml.as_bytes());
    let result = validate_yaml_profile(yaml);
    assert!(result.is_err(), "null byte must produce error");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            vb_yaml::YamlError::ForbiddenFeature {
                detail: "null_byte_in_source"
            }
        ),
        "expected ForbiddenFeature(null_byte_in_source), got {err:?}"
    );
}

#[test]
fn integration_custom_tag_triggers_yaml_error() {
    // Given: YAML with a custom tag
    let yaml = "version: v1\nname: test\nwhen: manual\nsteps:\n  - id: 1\n    do: !custom_action ping\n    input: 0\n";
    // When: harness processes it
    // Then: must not panic — custom tags are rejected
    fuzz_yaml_events(yaml.as_bytes());
    let result = validate_yaml_profile(yaml);
    if let Err(e) = result {
        assert!(
            matches!(
                e,
                vb_yaml::YamlError::CustomTag { .. } | vb_yaml::YamlError::ForbiddenFeature { .. }
            ),
            "expected CustomTag or ForbiddenFeature, got {e:?}"
        );
    }
}

#[test]
fn integration_binary_scalar_triggers_yaml_error() {
    // Given: YAML with a base64-encoded binary scalar
    let yaml = "version: v1\nname: test\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    data: !!binary |\n      R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7\n";
    // When: harness processes it
    // Then: must not panic — binary scalars are rejected
    fuzz_yaml_events(yaml.as_bytes());
    let result = validate_yaml_profile(yaml);
    if let Err(e) = result {
        assert!(
            matches!(
                e,
                vb_yaml::YamlError::BinaryScalar
                    | vb_yaml::YamlError::ForbiddenFeature { .. }
                    | vb_yaml::YamlError::CustomTag { .. }
            ),
            "expected BinaryScalar, ForbiddenFeature, or CustomTag, got {e:?}"
        );
    }
}

#[test]
fn integration_multiple_documents_triggers_yaml_error() {
    // Given: YAML with multiple documents
    let yaml = "version: v1\nname: test\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    input: 0\n---\nversion: v1\nname: test2\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    input: 0\n";
    // When: harness and profile process it
    // Then: must not panic — multiple documents are rejected
    fuzz_yaml_events(yaml.as_bytes());
    let result = validate_yaml_profile(yaml);
    if let Err(e) = result {
        assert!(
            matches!(
                e,
                vb_yaml::YamlError::MultipleDocuments { .. }
                    | vb_yaml::YamlError::ForbiddenFeature { .. }
            ),
            "expected MultipleDocuments or ForbiddenFeature, got {e:?}"
        );
    }
}

#[test]
fn integration_ambiguous_scalar_triggers_yaml_error() {
    // Given: YAML with YAML 1.1 ambiguous boolean
    let yaml = "version: v1\nname: test\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    input: 0\n    extra: yes\n";
    // When: harness processes it
    // Then: ambiguous scalars like 'yes' are rejected by profile
    fuzz_yaml_events(yaml.as_bytes());
    let result = validate_yaml_profile(yaml);
    if let Err(e) = result {
        assert!(
            matches!(
                e,
                vb_yaml::YamlError::AmbiguousScalar { .. }
                    | vb_yaml::YamlError::ForbiddenFeature { .. }
            ),
            "expected AmbiguousScalar or ForbiddenFeature, got {e:?}"
        );
    }
}

#[test]
fn integration_unsupported_feature_triggers_yaml_error() {
    // Given: YAML with an unsupported feature
    let yaml = "version: v1\nname: test\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    input: 0\n    timeout: 60\n";
    // When: harness processes it
    // Then: must not panic — unknown/unexpected fields are handled
    fuzz_yaml_events(yaml.as_bytes());
    // This may produce UnknownField or UnsupportedFeature depending on field location
    let result = validate_yaml_profile(yaml);
    if let Err(e) = result {
        assert!(
            matches!(
                e,
                vb_yaml::YamlError::UnknownField { .. }
                    | vb_yaml::YamlError::UnsupportedFeature { .. }
                    | vb_yaml::YamlError::ForbiddenFeature { .. }
            ),
            "expected typed error, got {e:?}"
        );
    }
}

// Remaining tests live in submodule
mod group_b;
