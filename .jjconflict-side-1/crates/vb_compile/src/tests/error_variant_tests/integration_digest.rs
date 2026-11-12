// ── Integration: workflow with reference parses ─────────────────────────────

#[test]
fn workflow_with_input_reference_parses() {
    let source = br#"version: velvet-ballistics/v1
name: ref_test
when:
  manual: {}
inputs:
  user: text
steps:
  - id: step1
    save:
      value: $input.user
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(result.is_ok(), "workflow with reference should parse");
}

// ── Integration: compute_compiled_digest is deterministic ───────────────────

#[test]
fn compiled_digest_is_deterministic() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let digest1 = crate::compute_compiled_digest(source);
    let digest2 = crate::compute_compiled_digest(source);
    assert_eq!(digest1, digest2);
}

#[test]
fn different_sources_produce_different_digests() {
    let source1 = br#"version: velvet-ballistics/v1
name: test1
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let source2 = br#"version: velvet-ballistics/v1
name: test2
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let digest1 = crate::compute_compiled_digest(source1);
    let digest2 = crate::compute_compiled_digest(source2);
    assert_ne!(digest1, digest2);
}

// ── Additional error variant coverage tests ───────────────────────────────────

#[test]
fn parse_error_returns_first_error() {
    let source = UNKNOWN_STEP_FIELD_SOURCE;
    let error = parse_error(source).expect("expected error");
    assert!(matches!(error, CompileError::UnknownStepField { .. }));
}

#[test]
fn compile_errors_iter_count_matches_len() {
    let source = UNKNOWN_STEP_FIELD_SOURCE;
    let Err(CompileErrors(errors)) = YamlCompiler::default().parse_ast(source) else {
        panic!("expected error");
    };
    assert_eq!(errors.iter().count(), errors.len());
}

#[test]
fn yaml_limits_max_sequence_defaults_correctly() {
    use crate::YamlLimits;
    let limits = YamlLimits::default();
    assert_eq!(limits.max_sequence_len, 10_000);
    assert_eq!(limits.max_mapping_entries, 1_024);
    assert_eq!(limits.max_scalar_bytes, 65_536);
}

// ── PO-009: Digest determinism ─────────────────────────────────────────────────

#[test]
fn compute_compiled_digest_determinism() {
    // Repeated calls produce bit-for-bit identical digest across multiple invocations.
    let source = br#"version: velvet-ballistics/v1
name: test
when:
 manual: {}
steps:
 - id: done
   finish:
     result: 0
"#;
    let digest1 = crate::compute_compiled_digest(source);
    let digest2 = crate::compute_compiled_digest(source);
    let digest3 = crate::compute_compiled_digest(source);
    assert_eq!(digest1, digest2, "first and second call must match");
    assert_eq!(digest2, digest3, "second and third call must match");
}

// ── PO-010: Artifact digest depends on source digest ───────────────────────────

#[test]
fn artifact_digest_depends_on_source() {
    // The artifact digest is a function of source digest plus IR serialization.
    // Changing source digest changes artifact digest.
    use crate::compile_workflow;
    use crate::emit_compiled_artifact;

    let source1 = br#"version: velvet-ballistics/v1
name: test1
when:
 manual: {}
steps:
 - id: done
   finish:
     result: 0
"#;
    let source2 = br#"version: velvet-ballistics/v1
name: test2
when:
 manual: {}
steps:
 - id: done
   finish:
     result: 0
"#;

    // Compile both sources
    let workflow1 = compile_workflow(source1).expect("source1 should compile");
    let workflow2 = compile_workflow(source2).expect("source2 should compile");

    // Get artifact bytes
    let artifact1 = emit_compiled_artifact(&workflow1).expect("workflow1 should emit");
    let artifact2 = emit_compiled_artifact(&workflow2).expect("workflow2 should emit");

    // Source digests differ
    let source_digest1 = crate::compute_compiled_digest(source1);
    let source_digest2 = crate::compute_compiled_digest(source2);
    assert_ne!(
        source_digest1, source_digest2,
        "different sources have different digests"
    );

    // Artifact bytes differ (because source differs)
    assert_ne!(
        artifact1.as_ref(),
        artifact2.as_ref(),
        "artifact bytes differ when source differs"
    );
}

