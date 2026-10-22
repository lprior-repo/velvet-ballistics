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
    TEMPLATE
        .replace("BODY_ID", step_id)
        .replace("OUTPUT", output)
}

#[test]
fn nested_step_id_output_framing_collision_regression() -> Result<(), String> {
    let a = make_variant("a", "setb");
    let b = make_variant("aset", "b");

    if a == b {
        return Err("test fixture error: source bytes are identical".to_string());
    }

    let source_a = parse_workflow_source(&a).map_err(|error| format!("parse a: {error:?}"))?;
    let source_b = parse_workflow_source(&b).map_err(|error| format!("parse b: {error:?}"))?;

    let digest_a = canonical_digest(&source_a).map_err(|error| format!("digest a: {error:?}"))?;
    let digest_b = canonical_digest(&source_b).map_err(|error| format!("digest b: {error:?}"))?;

    eprintln!("different source bytes: {}", a != b);
    eprintln!("semantic digest a: {digest_a:?}");
    eprintln!("semantic digest b: {digest_b:?}");
    eprintln!("canonical collision: {}", digest_a == digest_b);

    if digest_a == digest_b {
        return Err(
            "distinct nested step/output source identities collided on source digest".to_string(),
        );
    }

    let workflow_a = compile_source(&source_a).map_err(|error| format!("compile a: {error:?}"))?;
    let workflow_b = compile_source(&source_b).map_err(|error| format!("compile b: {error:?}"))?;

    let compiled_a = workflow_a.digest();
    let compiled_b = workflow_b.digest();
    eprintln!("compiled digest a: {compiled_a:?}");
    eprintln!("compiled digest b: {compiled_b:?}");
    eprintln!("compiled collision: {}", compiled_a == compiled_b);

    if compiled_a == compiled_b {
        return Err(
            "distinct nested step/output source identities collided on compiled digest".to_string(),
        );
    }

    Ok(())
}

// Optional field None vs Some discriminant test
// Wait(None, Some("x")) must differ from Wait(Some("none"), Some("x"))
#[test]
fn optional_field_none_vs_some_discriminant() -> Result<(), String> {
    let a = r#"version: velvet-ballistics/v1
name: wait_none
when:
  manual: {}
steps:
  - id: wait
    wait:
      timeout: "10s"
  - id: done
    finish:
      result: 0
"#;
    let b = r#"version: velvet-ballistics/v1
name: wait_some
when:
  manual: {}
steps:
  - id: wait
    wait:
      event: "none"
      timeout: "10s"
  - id: done
    finish:
      result: 0
"#;
    let source_a = parse_workflow_source(a).map_err(|error| format!("parse a: {error:?}"))?;
    let source_b = parse_workflow_source(b).map_err(|error| format!("parse b: {error:?}"))?;
    let digest_a = canonical_digest(&source_a).map_err(|error| format!("digest a: {error:?}"))?;
    let digest_b = canonical_digest(&source_b).map_err(|error| format!("digest b: {error:?}"))?;
    eprintln!("optional field test: digest_a={digest_a:?} digest_b={digest_b:?}");
    if digest_a == digest_b {
        return Err(
            "Wait(None,Some(x)) collided with Wait(Some(none),Some(x)) on optional discriminant"
                .to_string(),
        );
    }
    Ok(())
}

// Finish String vs Integer type tag test
#[test]
fn finish_string_vs_integer_type_tag() -> Result<(), String> {
    let a = r#"version: velvet-ballistics/v1
name: finish_string
when:
  manual: {}
steps:
  - id: done
    finish:
      result: "0"
"#;
    let b = r#"version: velvet-ballistics/v1
name: finish_integer
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let source_a = parse_workflow_source(a).map_err(|error| format!("parse a: {error:?}"))?;
    let source_b = parse_workflow_source(b).map_err(|error| format!("parse b: {error:?}"))?;
    let digest_a = canonical_digest(&source_a).map_err(|error| format!("digest a: {error:?}"))?;
    let digest_b = canonical_digest(&source_b).map_err(|error| format!("digest b: {error:?}"))?;
    eprintln!("finish type test: digest_a={digest_a:?} digest_b={digest_b:?}");
    if digest_a == digest_b {
        return Err(
            "Finish with string '0' collided with Finish with integer 0 on type tag".to_string(),
        );
    }
    Ok(())
}

// Do action/input fields must be hashed
#[test]
fn do_action_input_fields_hashed() -> Result<(), String> {
    let a = r#"version: velvet-ballistics/v1
name: do_a
when:
  manual: {}
steps:
  - id: act
    do:
      action: "print"
      input: "hello"
  - id: done
    finish:
      result: 0
"#;
    let b = r#"version: velvet-ballistics/v1
name: do_b
when:
  manual: {}
steps:
  - id: act
    do:
      action: "log"
      input: "world"
  - id: done
    finish:
      result: 0
"#;
    let source_a = parse_workflow_source(a).map_err(|error| format!("parse a: {error:?}"))?;
    let source_b = parse_workflow_source(b).map_err(|error| format!("parse b: {error:?}"))?;
    let digest_a = canonical_digest(&source_a).map_err(|error| format!("digest a: {error:?}"))?;
    let digest_b = canonical_digest(&source_b).map_err(|error| format!("digest b: {error:?}"))?;
    eprintln!("do field test: digest_a={digest_a:?} digest_b={digest_b:?}");
    if digest_a == digest_b {
        return Err("Do with different action/input collided on semantic fields".to_string());
    }
    Ok(())
}

// Save value must be hashed
#[test]
fn save_value_hashed() -> Result<(), String> {
    let a = r#"version: velvet-ballistics/v1
name: save_a
when:
  manual: {}
steps:
  - id: save
    save:
      output: result
      value: "42"
  - id: done
    finish:
      result: 0
"#;
    let b = r#"version: velvet-ballistics/v1
name: save_b
when:
  manual: {}
steps:
  - id: save
    save:
      output: result
      value: "99"
  - id: done
    finish:
      result: 0
"#;
    let source_a = parse_workflow_source(a).map_err(|error| format!("parse a: {error:?}"))?;
    let source_b = parse_workflow_source(b).map_err(|error| format!("parse b: {error:?}"))?;
    let digest_a = canonical_digest(&source_a).map_err(|error| format!("digest a: {error:?}"))?;
    let digest_b = canonical_digest(&source_b).map_err(|error| format!("digest b: {error:?}"))?;
    eprintln!("save field test: digest_a={digest_a:?} digest_b={digest_b:?}");
    if digest_a == digest_b {
        return Err("Save with different values collided on semantic fields".to_string());
    }
    Ok(())
}

// Input declarations must be hashed
#[test]
fn input_declarations_hashed() -> Result<(), String> {
    let a = r#"version: velvet-ballistics/v1
name: input_a
when:
  manual: {}
inputs:
  foo: string
steps:
  - id: done
    finish:
      result: 0
"#;
    let b = r#"version: velvet-ballistics/v1
name: input_b
when:
  manual: {}
inputs:
  bar: string
steps:
  - id: done
    finish:
      result: 0
"#;
    let source_a = parse_workflow_source(a).map_err(|error| format!("parse a: {error:?}"))?;
    let source_b = parse_workflow_source(b).map_err(|error| format!("parse b: {error:?}"))?;
    let digest_a = canonical_digest(&source_a).map_err(|error| format!("digest a: {error:?}"))?;
    let digest_b = canonical_digest(&source_b).map_err(|error| format!("digest b: {error:?}"))?;
    eprintln!("input field test: digest_a={digest_a:?} digest_b={digest_b:?}");
    if digest_a == digest_b {
        return Err("Different input declarations collided on semantic fields".to_string());
    }
    Ok(())
}

// YAML key order must be canonical (sorted)
#[test]
fn yaml_key_order_canonical() -> Result<(), String> {
    let a = r#"version: velvet-ballistics/v1
name: order_a
when:
  manual: {}
steps:
  - id: step
    set:
      output: x
      value: "1"
  - id: done
    finish:
      result: 0
"#;
    let b = r#"version: velvet-ballistics/v1
name: order_a
when:
  manual: {}
steps:
  - id: step
    set:
      value: "1"
      output: x
  - id: done
    finish:
      result: 0
"#;
    let source_a = parse_workflow_source(a).map_err(|error| format!("parse a: {error:?}"))?;
    let source_b = parse_workflow_source(b).map_err(|error| format!("parse b: {error:?}"))?;
    let digest_a = canonical_digest(&source_a).map_err(|error| format!("digest a: {error:?}"))?;
    let digest_b = canonical_digest(&source_b).map_err(|error| format!("digest b: {error:?}"))?;
    eprintln!("key order test: digest_a={digest_a:?} digest_b={digest_b:?}");
    if digest_a != digest_b {
        return Err(
            "Same workflow with different YAML key order produced different digests".to_string(),
        );
    }
    Ok(())
}
