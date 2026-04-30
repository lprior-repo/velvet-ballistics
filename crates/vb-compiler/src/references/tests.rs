use crate::{CompileError, YamlCompiler};

fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(error) => Ok(error),
    }
}

fn parse_ok(source: &[u8]) -> Result<(), String> {
    YamlCompiler::default()
        .parse_ast(source)
        .map(|_| ())
        .map_err(|error| format!("parse_ast failed: {error:?}"))
}

fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn compile_error_text(source: &[u8]) -> String {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => format!("compile unexpectedly succeeded: {workflow:?}"),
        Err(error) => error.to_string(),
    }
}

fn parse_error_text(source: &[u8]) -> String {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => format!("parse_ast unexpectedly succeeded: {ast:?}"),
        Err(error) => error.to_string(),
    }
}

fn compile_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(error) => Ok(error),
    }
}

#[test]
fn parse_ast_rejects_unknown_input_reference_after_schema() -> Result<(), String> {
    let error = parse_error(
        br#"version: velvet-ballastics/v1
name: ref_case
when:
  manual: {}
inputs:
  user: text
examples:
  - name: fixture
    value: $input.missing
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#,
    )?;

    ensure(
        matches!(
            error,
            CompileError::UnknownReferenceName { kind: "input", .. }
        ),
        "unknown input reference did not use typed diagnostic",
    )
}

#[test]
fn parse_ast_accepts_declared_cold_references() -> Result<(), String> {
    parse_ok(
        br#"version: velvet-ballastics/v1
name: ref_case
when:
  manual: {}
inputs:
  user: text
vars:
  retries: 1
secrets:
  token: TOKEN
examples:
  - name: fixture
    input_ref: $input.user
    var_ref: $vars.retries
    secret_ref: $secret.token
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#,
    )
}

#[test]
fn parse_ast_rejects_illegal_runtime_references() -> Result<(), String> {
    for reference in ["$runtime.now", "$now", "$random", "$steps.done"] {
        let source = format!(
            "version: velvet-ballastics/v1\nname: ref_case\nwhen:\n  manual: {{}}\nexamples:\n  - name: fixture\n    value: {reference}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let error = parse_error(source.as_bytes())?;
        ensure(
            matches!(error, CompileError::IllegalReference { .. }),
            "illegal reference did not use typed diagnostic",
        )?;
    }
    Ok(())
}

#[test]
fn parse_ast_rejects_unknown_reference_root() -> Result<(), String> {
    let error = parse_error(
        br#"version: velvet-ballastics/v1
name: ref_case
when:
  manual: {}
examples:
  - name: fixture
    value: $env.HOME
steps:
  - id: done
    finish:
      result: 0
"#,
    )?;

    ensure(
        matches!(error, CompileError::UnknownReferenceRoot { .. }),
        "unknown root did not use typed diagnostic",
    )
}

#[test]
fn reference_validation_does_not_preempt_lowering_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: ref_case
when:
  manual: {}
examples:
  - name: fixture
    value: $input.missing
steps:
  - id: build
    save:
      value: $input.missing
  - id: done
    finish:
      result: 0
"#;

    ensure(
        compile_error_text(source) == parse_error_text(source),
        "reference validation changed lowering-first diagnostic order",
    )
}

#[test]
fn compile_rejects_unknown_reference_in_retained_surface() -> Result<(), String> {
    let error = compile_error(
        br#"version: velvet-ballastics/v1
name: ref_case
when:
  manual: {}
inputs:
  user: text
examples:
  - name: bad
    value: $input.missing
steps:
  - id: done
    finish:
      result: 0
"#,
    )?;

    match error {
        CompileError::UnknownReferenceName { kind: "input", .. } => Ok(()),
        other => Err(format!(
            "compile did not reject unknown retained input reference: {other:?}"
        )),
    }
}

#[test]
fn compile_rejects_illegal_reference_in_retained_surface() -> Result<(), String> {
    let error = compile_error(
        br#"version: velvet-ballastics/v1
name: ref_case
when:
  manual: {}
examples:
  - name: bad
    value: $runtime.now
steps:
  - id: done
    finish:
      result: 0
"#,
    )?;

    match error {
        CompileError::IllegalReference { .. } => Ok(()),
        other => Err(format!(
            "compile did not reject illegal retained reference: {other:?}"
        )),
    }
}
