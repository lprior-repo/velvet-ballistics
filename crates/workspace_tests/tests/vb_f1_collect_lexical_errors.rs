use vb_compile::{CompileError, compile_workflow};

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: collect_lexical
when:
  manual: {}
steps:
  - id: bounded
    collect:
      variable: batch
      source: "0"
      pages: 2
      items: 1
      steps:
        - id: copy
          set:
            output: copied
            value: "$batch"
  - id: done
    finish:
      result: "$step.bounded"
"#;

fn reject(source: &str, expected: CompileError) -> Result<(), String> {
    match compile_workflow(source.as_bytes()) {
        Ok(_) => Err("compile_ok".to_string()),
        Err(e) => {
            if e.0.contains(&expected) {
                Ok(())
            } else {
                Err(format!("{:?}", e.0))
            }
        }
    }
}

#[test]
fn missing_reference() -> Result<(), String> {
    let s = SOURCE.replace("source: \"0\"", "source: \"$missing\"");
    reject(
        &s,
        CompileError::UnknownReferenceName {
            kind: "output",
            reference: "$missing".into(),
            name: "missing".into(),
        },
    )
}

#[test]
fn future_reference() -> Result<(), String> {
    let s = SOURCE.replace("value: \"$batch\"", "value: \"$future\"");
    reject(
        &s,
        CompileError::UnknownReferenceName {
            kind: "output",
            reference: "$future".into(),
            name: "future".into(),
        },
    )
}

#[test]
fn duplicate_output_name_in_set() -> Result<(), String> {
    let s = SOURCE.replace("output: copied", "output: batch");
    reject(
        &s,
        CompileError::DuplicateOutputName {
            name: "batch".into(),
        },
    )
}

#[test]
fn duplicate_output_name_in_id() -> Result<(), String> {
    let s = SOURCE.replace("id: copy", "id: batch");
    reject(
        &s,
        CompileError::DuplicateOutputName {
            name: "batch".into(),
        },
    )
}

#[test]
fn invalid_collect_variable_name() -> Result<(), String> {
    let s = SOURCE.replace("variable: batch", "variable: page");
    reject(
        &s,
        CompileError::InvalidName {
            field: "collect.variable",
            value: "page".into(),
        },
    )
}

#[test]
fn capture_outer_binding() -> Result<(), String> {
    let seed = "  - id: seed\n    set: { output: batch, value: '1' }\n";
    let s = SOURCE.replace("  - id: bounded", &format!("{}\n  - id: bounded", seed));
    reject(
        &s,
        CompileError::DuplicateOutputName {
            name: "batch".into(),
        },
    )
}

#[test]
fn collect_alias_cannot_escape() -> Result<(), String> {
    let esc = "  - id: after_loop\n    set: { output: after, value: '$copied' }\n";
    let s = SOURCE.replace("  - id: done", &format!("{}\n  - id: done", esc));
    reject(
        &s,
        CompileError::UnknownReferenceName {
            kind: "output",
            reference: "$copied".into(),
            name: "copied".into(),
        },
    )
}
