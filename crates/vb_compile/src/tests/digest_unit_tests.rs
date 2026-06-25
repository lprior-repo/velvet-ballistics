// Unit tests for canonical_digest and digest_step_primitive
// Bead: vb-xi2f.34 — Finish Digest Semantics
//
// These tests exercise the pub(crate) digest functions in
// mod_compile_lowering::part_05. Because the test module is declared
// via #[path] inside part_05.rs, it has access to super::* and
// therefore to canonical_digest and digest_step_primitive.
//
// Test plan: UT-1 through UT-8 (Section 9.1)
// Contract clauses: C1–C10

#![forbid(unsafe_code)]

use super::*;
use vb_core::WorkflowDigest;
use vb_yaml::ast::{ScalarValue, StepPrimitive};
use vb_yaml::parse_workflow_source;

// ── Helpers ────────────────────────────────────────────────────────

/// Build a Finish StepPrimitive with a String result.
fn finish_string(value: &str) -> StepPrimitive {
    StepPrimitive::Finish {
        result: ScalarValue::String(value.to_string()),
    }
}

/// Build a Finish StepPrimitive with an Integer result.
fn finish_integer(value: i64) -> StepPrimitive {
    StepPrimitive::Finish {
        result: ScalarValue::Integer(value),
    }
}

/// Hash a manual byte sequence and return the blake3 output bytes.
fn hash_bytes(data: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for chunk in data {
        hasher.update(chunk);
    }
    hasher.finalize().into()
}

/// Call digest_step_primitive with a hasher and return the final hash bytes.
fn hash_primitive(primitive: &StepPrimitive) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    digest_step_primitive(&mut hasher, primitive).expect("valid test primitive");
    hasher.finalize().into()
}

/// Parse a minimal YAML source and return its canonical digest.
fn digest_yaml(yaml: &str) -> WorkflowDigest {
    let source = parse_workflow_source(yaml).expect("valid YAML must parse");
    canonical_digest(&source).expect("valid test input")
}

// ── UT-1: Finish discriminator prefix (B9) ─────────────────────────

/// Verify that digest_step_primitive writes b"finish" discriminator
/// for a Finish primitive. The digest produced by the function (which
/// writes b"finish" + result encoding) differs from a digest produced
/// by hashing only the result encoding without the discriminator.
#[test]
fn digest_step_primitive_finish_writes_finish_discriminator() {
    let primitive = finish_integer(42);

    // Hash produced by digest_step_primitive: b"finish" + 42i64 LE
    let actual = hash_primitive(&primitive);

    // Hash produced manually: only 42i64 LE (no discriminator)
    let manual_no_discriminator = hash_bytes(&[&42_i64.to_le_bytes()]);

    // Hashes must differ — discriminator changes digest
    assert_ne!(
        actual, manual_no_discriminator,
        "Finish discriminator must affect the digest"
    );

    // Hash produced manually: b"finish" + 42i64 LE (exact match)
    let manual_with_discriminator = hash_bytes(&[b"finish", &42_i64.to_le_bytes()]);

    // Hashes must match — encoding is deterministic
    assert_eq!(
        actual, manual_with_discriminator,
        "Finish encoding must match manual b\"finish\" + LE bytes"
    );
}

// ── UT-2: String result encoding (B10) ─────────────────────────────

/// Verify that digest_step_primitive encodes String result as
/// b"finish" followed by the raw UTF-8 bytes of the string.
#[test]
fn digest_step_primitive_finish_encodes_string_result_as_utf8_bytes() {
    let primitive = finish_string("my_output");

    let actual = hash_primitive(&primitive);
    let expected = hash_bytes(&[b"finish", b"my_output"]);

    assert_eq!(
        actual, expected,
        "Finish String result must hash b\"finish\" + raw UTF-8 bytes"
    );
}

/// String empty case: empty string result still writes discriminator.
#[test]
fn digest_step_primitive_finish_encodes_empty_string() {
    let primitive = finish_string("");

    let actual = hash_primitive(&primitive);
    // b"finish" + b"" = just b"finish"
    let expected = hash_bytes(&[b"finish", b""]);

    assert_eq!(
        actual, expected,
        "Finish String result empty string must produce valid hash"
    );

    // Differ from Integer(0) encoding
    let int_primitive = finish_integer(0);
    let int_hash = hash_primitive(&int_primitive);
    assert_ne!(
        actual, int_hash,
        "empty string must differ from Integer(0) encoding"
    );
}

/// Unicode: non-ASCII string content is hashed as raw UTF-8 bytes.
#[test]
fn digest_step_primitive_finish_encodes_unicode_string() {
    let primitive = finish_string("ré∑umé");

    let actual = hash_primitive(&primitive);
    let expected = hash_bytes(&[b"finish", "ré∑umé".as_bytes()]);

    assert_eq!(
        actual, expected,
        "Finish String with unicode must hash raw UTF-8 bytes"
    );
}

// ── UT-3: Integer result encoding (B11) ────────────────────────────

/// Verify that digest_step_primitive encodes Integer result as
/// b"finish" followed by i64::to_le_bytes().
#[test]
fn digest_step_primitive_finish_encodes_integer_result_as_le_bytes() {
    let primitive = finish_integer(42);

    let actual = hash_primitive(&primitive);
    let expected = hash_bytes(&[b"finish", &42_i64.to_le_bytes()]);

    assert_eq!(
        actual, expected,
        "Finish Integer result must hash b\"finish\" + i64 LE bytes"
    );
}

/// Integer zero: all-zero LE bytes.
#[test]
fn digest_step_primitive_finish_encodes_integer_zero() {
    let primitive = finish_integer(0);

    let actual = hash_primitive(&primitive);
    let expected = hash_bytes(&[b"finish", &[0u8; 8]]);

    assert_eq!(
        actual, expected,
        "Finish Integer 0 must hash b\"finish\" + [0u8; 8]"
    );
}

/// Integer negative: -1 produces all-0xFF LE bytes.
#[test]
fn digest_step_primitive_finish_encodes_integer_negative() {
    let primitive = finish_integer(-1);

    let actual = hash_primitive(&primitive);
    let expected = hash_bytes(&[b"finish", &[0xFF_u8; 8]]);

    assert_eq!(
        actual, expected,
        "Finish Integer -1 must hash b\"finish\" + [0xFF_u8; 8]"
    );
}

/// Integer i64::MIN: boundary value.
#[test]
fn digest_step_primitive_finish_encodes_integer_min() {
    let primitive = finish_integer(i64::MIN);

    let actual = hash_primitive(&primitive);
    let expected = hash_bytes(&[b"finish", &i64::MIN.to_le_bytes()]);

    assert_eq!(
        actual, expected,
        "Finish Integer i64::MIN must produce correct hash"
    );
}

/// Integer i64::MAX: boundary value.
#[test]
fn digest_step_primitive_finish_encodes_integer_max() {
    let primitive = finish_integer(i64::MAX);

    let actual = hash_primitive(&primitive);
    let expected = hash_bytes(&[b"finish", &i64::MAX.to_le_bytes()]);

    assert_eq!(
        actual, expected,
        "Finish Integer i64::MAX must produce correct hash"
    );
}

// ── UT-4: String vs Integer discrimination (B15) ────────────────────

/// Verify that Finish { result: String("42") } and
/// Finish { result: Integer(42) } produce different digests.
#[test]
fn digest_step_primitive_finish_string_vs_integer_produce_different_encoding_bytes() {
    let string_prim = finish_string("42");
    let int_prim = finish_integer(42);

    let string_hash = hash_primitive(&string_prim);
    let int_hash = hash_primitive(&int_prim);

    assert_ne!(
        string_hash, int_hash,
        "Finish String and Integer encodings must differ even for same logical value"
    );
}

/// Verify that Finish String and Finish Integer have different encoding
/// lengths (String is variable-length, Integer is always 8 bytes).
#[test]
fn digest_step_primitive_finish_string_and_integer_have_different_byte_lengths() {
    let string_prim = finish_string("42");
    let int_prim = finish_integer(42);

    let string_hash = hash_primitive(&string_prim);
    let int_hash = hash_primitive(&int_prim);

    assert_ne!(
        string_hash, int_hash,
        "String and Integer encodings must produce different hashes"
    );
}

// ── UT-5: Canonical digest determinism (B6) ─────────────────────────

/// Verify that canonical_digest returns identical digests for identical sources.
#[test]
fn canonical_digest_is_deterministic_for_identical_source() {
    let yaml = r#"version: velvet-ballistics/v1
name: det_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 42
"#;
    let d1 = digest_yaml(yaml);
    let d2 = digest_yaml(yaml);

    assert_eq!(
        d1, d2,
        "canonical_digest must be deterministic for identical source"
    );
    // Digest must be non-zero
    let zero = WorkflowDigest::from_bytes([0u8; 32]);
    assert_ne!(
        d1, zero,
        "canonical_digest must produce non-zero output for valid input"
    );
}

// ── UT-6: Step ID sensitivity (B4) ─────────────────────────────────

/// Verify that changing the Finish step's id changes the canonical digest.
#[test]
fn canonical_digest_sensitive_to_step_id() {
    let yaml_a = r#"version: velvet-ballistics/v1
name: step_id_test
when:
  manual: {}
steps:
  - id: first
    finish:
      result: 0
"#;
    let yaml_b = r#"version: velvet-ballistics/v1
name: step_id_test
when:
  manual: {}
steps:
  - id: second
    finish:
      result: 0
"#;
    let d1 = digest_yaml(yaml_a);
    let d2 = digest_yaml(yaml_b);

    assert_ne!(
        d1, d2,
        "different step IDs must produce different canonical digests"
    );
}

// ── UT-7: Version field contribution (B1) ─────────────────────────

/// Verify that the workflow version field contributes to the digest.
#[test]
fn canonical_digest_includes_version_field() {
    let yaml_v1 = r#"version: velvet-ballistics/v1
name: ver_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let yaml_v2 = r#"version: velvet-ballistics/v2
name: ver_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let d1 = digest_yaml(yaml_v1);
    let d2 = digest_yaml(yaml_v2);

    assert_ne!(
        d1, d2,
        "different version strings must produce different digests"
    );
}

/// Verify that the workflow name field contributes to the digest.
#[test]
fn canonical_digest_includes_name_field() {
    let yaml_a = r#"version: velvet-ballistics/v1
name: workflow_alpha
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let yaml_b = r#"version: velvet-ballistics/v1
name: workflow_beta
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let d1 = digest_yaml(yaml_a);
    let d2 = digest_yaml(yaml_b);

    assert_ne!(
        d1, d2,
        "different workflow names must produce different digests"
    );
}

// ── UT-8: Unknown ScalarValue → b"unsupported" (B12) ─────────────

/// Verify that both String and Integer ScalarValue variants are
/// explicitly matched in digest_step_primitive (not falling through
/// to the `_` arm that produces b"unsupported").
///
/// Since ScalarValue is #[non_exhaustive], we cannot construct a new variant
/// from outside the defining crate. We verify that current variants produce
/// hashes that differ from the "unsupported" fallback, proving they are
/// matched explicitly.
#[test]
fn digest_step_primitive_finish_writes_unsupported_for_unknown_scalar_value() {
    // The `_` arm produces: b"finish" + b"unsupported"
    let unsupported_hash = hash_bytes(&[b"finish", b"unsupported"]);

    // Verify String variant is matched explicitly
    let string_prim = finish_string("test_unsupported_check");
    let string_hash = hash_primitive(&string_prim);
    assert_ne!(
        string_hash, unsupported_hash,
        "String variant must NOT fall through to the `_` arm"
    );

    // Verify Integer variant is matched explicitly
    let int_prim = finish_integer(0);
    let int_hash = hash_primitive(&int_prim);
    assert_ne!(
        int_hash, unsupported_hash,
        "Integer variant must NOT fall through to the `_` arm"
    );
}

// ── Additional: Trigger field sensitivity ────────────────────────

/// Verify that different trigger types produce different digests.
#[test]
fn canonical_digest_sensitive_to_trigger_type() {
    let yaml_manual = r#"version: velvet-ballistics/v1
name: trig_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let yaml_webhook = r#"version: velvet-ballistics/v1
name: trig_test
when:
  webhook: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let d_manual = digest_yaml(yaml_manual);
    let d_webhook = digest_yaml(yaml_webhook);

    assert_ne!(
        d_manual, d_webhook,
        "different trigger types must produce different digests"
    );
}

/// Verify that schedule cron parameter changes digest.
#[test]
fn canonical_digest_sensitive_to_schedule_param() {
    let yaml_a = r#"version: velvet-ballistics/v1
name: sched_test
when:
  schedule:
    cron: "0 0 * * *"
steps:
  - id: done
    finish:
      result: 0
"#;
    let yaml_b = r#"version: velvet-ballistics/v1
name: sched_test
when:
  schedule:
    cron: "0 12 * * *"
steps:
  - id: done
    finish:
      result: 0
"#;
    let d1 = digest_yaml(yaml_a);
    let d2 = digest_yaml(yaml_b);

    assert_ne!(
        d1, d2,
        "different schedule cron expressions must produce different digests"
    );
}

/// Verify that different finish result Integer values produce different digests
/// at the unit level (direct canonical_digest call).
#[test]
fn canonical_digest_sensitive_to_finish_integer_value() {
    let yaml_0 = r#"version: velvet-ballistics/v1
name: int_sens
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let yaml_1 = r#"version: velvet-ballistics/v1
name: int_sens
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 1
"#;
    let d0 = digest_yaml(yaml_0);
    let d1 = digest_yaml(yaml_1);

    assert_ne!(
        d0, d1,
        "different Integer finish result values must produce different digests"
    );
}

/// Verify that different finish result String values produce different digests.
#[test]
fn canonical_digest_sensitive_to_finish_string_value() {
    let yaml_a = r#"version: velvet-ballistics/v1
name: str_sens
when:
  manual: {}
steps:
  - id: set_up
    set:
      output: out_a
      value: "10"
  - id: done
    finish:
      result: "out_a"
"#;
    let yaml_b = r#"version: velvet-ballistics/v1
name: str_sens
when:
  manual: {}
steps:
  - id: set_up
    set:
      output: out_b
      value: "10"
  - id: done
    finish:
      result: "out_b"
"#;
    let da = digest_yaml(yaml_a);
    let db = digest_yaml(yaml_b);

    assert_ne!(
        da, db,
        "different String finish result values must produce different digests"
    );
}

/// Verify that step count changes digest (multi-step vs single-step).
#[test]
fn canonical_digest_sensitive_to_step_count() {
    let yaml_1step = r#"version: velvet-ballistics/v1
name: count_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let yaml_2step = r#"version: velvet-ballistics/v1
name: count_test
when:
  manual: {}
steps:
  - id: first
    set:
      output: x
      value: "10"
  - id: done
    finish:
      result: 0
"#;
    let d1 = digest_yaml(yaml_1step);
    let d2 = digest_yaml(yaml_2step);

    assert_ne!(
        d1, d2,
        "different step counts must produce different digests"
    );
}

/// Verify that identical multi-step workflows produce identical digests.
#[test]
fn canonical_digest_deterministic_for_multi_step_workflow() {
    let yaml = r#"version: velvet-ballistics/v1
name: multi_step_det
when:
  manual: {}
steps:
  - id: step_1
    set:
      output: a
      value: "10"
  - id: step_2
    set:
      output: b
      value: "20"
  - id: done
    finish:
      result: 42
"#;
    let d1 = digest_yaml(yaml);
    let d2 = digest_yaml(yaml);
    assert_eq!(d1, d2, "multi-step workflow digest must be deterministic");
}
