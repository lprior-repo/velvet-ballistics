//! Regression test for canonical digest collision caused by unframed
//! concatenation of step id and output variable names.
//!
//! Bead: vb-a2l02
//!
//! Two semantically distinct Repeat workflows:
//! - step id "a" with output variable "setb"
//! - step id "aset" with output variable "b"
//!
//! Previously collided because the raw concatenation produced
//! identical byte sequences. After the fix, framing must separate
//! the fields and the digests must differ.

use vb_compile::{canonical_digest, compile_source, parse_workflow_source};

const TEMPLATE: &str = r#"version: velvet-ballistics/v1
name: collision
when:
  manual: {}
steps:
  - id: retry
    repeat:
      max_attempts: 3
      steps:
        - id: BODY_ID
          set:
            output: OUTPUT
            value: "1"
  - id: done
    finish:
      result: 0
"#;

fn make_variant(step_id: &str, output: &str) -> String {
    TEMPLATE.replace("BODY_ID", step_id).replace("OUTPUT", output)
}

#[test]
fn nested_step_id_output_framing_collision_regression() -> Result<(), String> {
    let a = make_variant("a", "setb");
    let b = make_variant("aset", "b");

    if a == b {
        return Err("test fixture error: source bytes are identical".to_string());
    }

    let source_a =
        parse_workflow_source(&a).map_err(|error| format!("parse a: {error:?}"))?;
    let source_b =
        parse_workflow_source(&b).map_err(|error| format!("parse b: {error:?}"))?;

    let digest_a =
        canonical_digest(&source_a).map_err(|error| format!("digest a: {error:?}"))?;
    let digest_b =
        canonical_digest(&source_b).map_err(|error| format!("digest b: {error:?}"))?;

    eprintln!("different source bytes: {}", a != b);
    eprintln!("semantic digest a: {digest_a:?}");
    eprintln!("semantic digest b: {digest_b:?}");
    eprintln!("canonical collision: {}", digest_a == digest_b);

    if digest_a == digest_b {
        return Err("distinct nested step/output source identities collided on source digest".to_string());
    }

    let workflow_a =
        compile_source(&source_a).map_err(|error| format!("compile a: {error:?}"))?;
    let workflow_b =
        compile_source(&source_b).map_err(|error| format!("compile b: {error:?}"))?;

    let compiled_a = workflow_a.digest();
    let compiled_b = workflow_b.digest();
    eprintln!("compiled digest a: {compiled_a:?}");
    eprintln!("compiled digest b: {compiled_b:?}");
    eprintln!("compiled collision: {}", compiled_a == compiled_b);

    if compiled_a == compiled_b {
        return Err("distinct nested step/output source identities collided on compiled digest".to_string());
    }

    Ok(())
}