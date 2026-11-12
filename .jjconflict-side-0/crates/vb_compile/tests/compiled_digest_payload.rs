//! Contract tests for canonical compiled artifact payload digests.

use vb_compile::{canonical_digest, compile_workflow, parse_workflow_source};
use vb_core::WorkflowDigest;

fn compile(yaml: &str) -> Result<vb_core::CompiledWorkflow, String> {
    compile_workflow(yaml.as_bytes()).map_err(|errors| format!("compile failed: {errors:?}"))
}

#[test]
fn compiled_digest_is_hash_of_serialized_parts_with_digest_zeroed() -> Result<(), String> {
    let yaml = r#"version: velvet-ballistics/v1
name: payload_digest
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 7
"#;
    let workflow = compile(yaml)?;
    let mut parts = workflow.to_parts();
    let expected_digest = parts.digest;
    parts.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let payload = postcard::to_allocvec(&parts)
        .map_err(|error| format!("compiled parts serialization failed: {error}"))?;
    let actual_digest = WorkflowDigest::from_bytes(blake3::hash(&payload).into());

    if actual_digest == expected_digest {
        Ok(())
    } else {
        Err(format!(
            "digest {expected_digest:?} does not match canonical payload digest {actual_digest:?}"
        ))
    }
}

#[test]
fn formatting_and_comments_do_not_change_compiled_digest() -> Result<(), String> {
    let compact = r#"version: velvet-ballistics/v1
name: stable_source
when: {manual: {}}
steps:
  - id: done
    finish: {result: 7}
"#;
    let annotated = r#"# same semantic workflow
version: velvet-ballistics/v1
name: stable_source
when:
  manual: {}
steps:
  # this comment is not part of the compiled artifact
  - id: done
    finish:
      result: 7
"#;
    let compact_digest = compile(compact)?.digest();
    let annotated_digest = compile(annotated)?.digest();

    if compact_digest == annotated_digest {
        Ok(())
    } else {
        Err(format!(
            "semantic equivalents differ: {compact_digest:?} vs {annotated_digest:?}"
        ))
    }
}

#[test]
fn source_identity_survives_parts_roundtrip() -> Result<(), String> {
    let yaml = r#"version: velvet-ballistics/v1
name: roundtrip_identity
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 7
"#;
    let source =
        parse_workflow_source(yaml).map_err(|error| format!("source parsing failed: {error:?}"))?;
    let expected_identity = canonical_digest(&source)
        .map_err(|error| format!("canonical source identity failed: {error:?}"))?;
    let workflow = vb_compile::compile_source(&source)
        .map_err(|error| format!("source compilation failed: {error:?}"))?;
    let parts = workflow.to_parts();
    if parts.semantic_source_identity != Some(expected_identity) {
        return Err("compiled parts omit or change canonical source identity".to_string());
    }
    let reconstructed = vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|error| format!("parts reconstruction failed: {error:?}"))?;
    if reconstructed.semantic_source_identity() == Some(expected_identity) {
        Ok(())
    } else {
        Err("try_from_parts did not preserve canonical source identity".to_string())
    }
}
