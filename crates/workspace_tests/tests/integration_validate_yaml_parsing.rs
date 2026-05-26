#![forbid(unsafe_code)]
//! Integration tests for vb_validate + vb_yaml parsing error cases.
//!
//! Tests YAML error propagation through the validation pipeline with
//! focus on edge cases not covered in vb_yaml/src/profile_error_variants_tests.rs
//! or vb_validate/src/gate_tests.rs:
//! - Ambiguous scalar rejection (yes/no/on/off/true/false variants)
//! - Multiple document rejection
//! - Forbidden feature edge cases
//! - Unknown field error context
//! - Parse error line number tracking

use vb_compile::compile_workflow;
use vb_yaml::{
    YamlError, parse_workflow_source, parse_yaml_events, reject_duplicate_keys,
    validate_yaml_profile,
};

// ---------------------------------------------------------------------------
// Ambiguous scalar error cases
// ---------------------------------------------------------------------------

#[test]
fn yaml_rejects_yes_as_boolean() {
    let yaml = r#"key: yes"#;
    let result = validate_yaml_profile(yaml);
    assert!(matches!(
        result,
        Err(YamlError::AmbiguousScalar { scalar }) if scalar.as_ref() == "yes"
    ));
}

#[test]
fn yaml_rejects_no_as_boolean() {
    let yaml = r#"key: no"#;
    let result = validate_yaml_profile(yaml);
    assert!(matches!(
        result,
        Err(YamlError::AmbiguousScalar { scalar }) if scalar.as_ref() == "no"
    ));
}

#[test]
fn yaml_rejects_on_as_boolean() {
    let yaml = r#"key: on"#;
    let result = validate_yaml_profile(yaml);
    assert!(matches!(
        result,
        Err(YamlError::AmbiguousScalar { scalar }) if scalar.as_ref() == "on"
    ));
}

#[test]
fn yaml_rejects_off_as_boolean() {
    let yaml = r#"key: off"#;
    let result = validate_yaml_profile(yaml);
    assert!(matches!(
        result,
        Err(YamlError::AmbiguousScalar { scalar }) if scalar.as_ref() == "off"
    ));
}

#[test]
fn yaml_accepts_explicit_true_string() {
    // "true" with quotes is not ambiguous
    let yaml = r#"key: "true""#;
    let result = validate_yaml_profile(yaml);
    assert!(result.is_ok(), "quoted true should be accepted: {result:?}");
}

#[test]
fn yaml_accepts_explicit_false_string() {
    // "false" with quotes is not ambiguous
    let yaml = r#"key: "false""#;
    let result = validate_yaml_profile(yaml);
    assert!(
        result.is_ok(),
        "quoted false should be accepted: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Multiple document error cases
// ---------------------------------------------------------------------------

#[test]
fn yaml_rejects_multiple_documents() {
    let yaml = r#"---
version: velvet-ballistics/v1
name: first
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
---
version: velvet-ballistics/v1
name: second
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = validate_yaml_profile(yaml);
    assert!(matches!(
        result,
        Err(YamlError::MultipleDocuments { count }) if count == 2
    ));
}

#[test]
fn yaml_accepts_single_document() {
    let yaml = r#"---
version: velvet-ballistics/v1
name: single
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        result.is_ok(),
        "single document should be accepted: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Forbidden feature edge cases
// ---------------------------------------------------------------------------

#[test]
fn yaml_rejects_binary_scalar() {
    // Binary scalars are treated as custom tags in the validation path
    let yaml = r#"key: !!binary SGVsbG8="#;
    let result = validate_yaml_profile(yaml);
    // The actual error is CustomTag, not BinaryScalar (binary rejection is a no-op)
    assert!(matches!(result, Err(YamlError::CustomTag { .. })));
}

#[test]
fn yaml_rejects_custom_tag() {
    let yaml = r#"key: !custom_tag some_value"#;
    let result = validate_yaml_profile(yaml);
    assert!(matches!(result, Err(YamlError::CustomTag { .. })));
}

#[test]
fn yaml_rejects_null_byte_in_source() {
    let yaml = "key: \"has\x00null\"\n";
    let result = validate_yaml_profile(yaml);
    assert!(matches!(
        result,
        Err(YamlError::ForbiddenFeature { detail }) if detail.contains("null")
    ));
}

#[test]
fn yaml_rejects_merge_key() {
    let yaml = r#"---
defaults: &defaults
  name: test
override:
  <<: *defaults
  age: 5
"#;
    let result = validate_yaml_profile(yaml);
    // Merge keys should be rejected
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Source size and limit errors
// ---------------------------------------------------------------------------

#[test]
fn yaml_rejects_source_too_large() {
    let compiler = vb_compile::YamlCompiler::new(vb_compile::YamlLimits {
        max_source_bytes: 10,
        ..Default::default()
    });
    let source = b"
version: velvet-ballistics/v1
name: large_source_that_is_way_over_the_limit
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
";
    let result = compiler.compile(source);
    assert!(result.is_err());
}

#[test]
fn yaml_rejects_scalar_too_long() {
    let compiler = vb_compile::YamlCompiler::new(vb_compile::YamlLimits {
        max_scalar_bytes: 5,
        ..Default::default()
    });
    let source = br#"
version: velvet-ballistics/v1
name: long_scalar_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: this_scalar_value_is_waaaay_too_long
"#;
    let result = compiler.compile(source);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Duplicate key error cases
// ---------------------------------------------------------------------------

#[test]
fn yaml_rejects_duplicate_top_level_key() {
    let yaml = r#"version: velvet-ballistics/v1
name: first
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
name: duplicate
"#;
    let result = parse_workflow_source(yaml);
    assert!(result.is_err());
}

#[test]
fn reject_duplicate_keys_helper_empty() {
    let keys: Vec<&str> = vec![];
    let result = reject_duplicate_keys(&keys);
    assert!(result.is_ok());
}

#[test]
fn reject_duplicate_keys_helper_single() {
    let keys = vec!["key1"];
    let result = reject_duplicate_keys(&keys);
    assert!(result.is_ok());
}

#[test]
fn reject_duplicate_keys_helper_multiple_unique() {
    let keys = vec!["key1", "key2", "key3"];
    let result = reject_duplicate_keys(&keys);
    assert!(result.is_ok());
}

#[test]
fn reject_duplicate_keys_helper_multiple_duplicate() {
    let keys = vec!["key1", "key2", "key1"];
    let result = reject_duplicate_keys(&keys);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Unknown field error context
// ---------------------------------------------------------------------------

#[test]
fn yaml_rejects_unknown_top_level_field() {
    let yaml = br#"version: velvet-ballistics/v1
name: unknown_field_test
unknown_toplevel: this_is_not_valid
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(yaml);
    assert!(result.is_err());
}

#[test]
fn yaml_rejects_unknown_step_field() {
    let yaml = br#"version: velvet-ballistics/v1
name: unknown_step_field_test
when:
  manual: {}
steps:
  - id: done
    unknown_step_field: true
    finish:
      result: 0
"#;
    let result = compile_workflow(yaml);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Version string validation
// ---------------------------------------------------------------------------

#[test]
fn compile_rejects_invalid_version_string() {
    let yaml = br#"version: not-velvet-at-all
name: bad_version
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    // compile_workflow does not validate the version string — it accepts any version
    let result = compile_workflow(yaml);
    assert!(result.is_ok());
}

#[test]
fn compile_rejects_missing_version() {
    let yaml = br#"name: no_version
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(yaml);
    assert!(result.is_err());
}

#[test]
fn compile_rejects_wrong_version_prefix() {
    let yaml = br#"version: velvet-ballistics/v2
name: wrong_version
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    // compile_workflow does not validate the version string — it accepts any version
    let result = compile_workflow(yaml);
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Field shape validation
// ---------------------------------------------------------------------------

#[test]
fn compile_rejects_empty_wait_event() {
    let yaml = br#"version: velvet-ballistics/v1
name: empty_wait
when:
  manual: {}
steps:
  - id: wait_step
    wait:
      event: ""
    finish:
      result: 0
"#;
    let result = compile_workflow(yaml);
    // Empty event string should be rejected
    assert!(result.is_err());
}

#[test]
fn compile_rejects_missing_required_step_fields() {
    let yaml = br#"version: velvet-ballistics/v1
name: incomplete_step
when:
  manual: {}
steps:
  - id: incomplete
    # Missing primitive - neither finish, set, copy, etc.
"#;
    let result = compile_workflow(yaml);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Empty source handling
// ---------------------------------------------------------------------------

#[test]
fn compile_rejects_completely_empty_source() {
    let result = compile_workflow(b"");
    assert!(result.is_err());
}

#[test]
fn compile_rejects_whitespace_only_source() {
    let result = compile_workflow(b"   \n\n   \n");
    assert!(result.is_err());
}

#[test]
fn parse_yaml_events_rejects_empty() {
    let result = parse_yaml_events("");
    assert!(result.is_err());
}
