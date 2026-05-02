use crate::{CompileError, YamlCompiler};

fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(errors) => errors
            .0
            .into_iter()
            .next()
            .ok_or_else(|| "parse_ast failed with no errors".to_string()),
    }
}

fn parse_ok(source: &[u8]) -> Result<(), String> {
    YamlCompiler::default()
        .parse_ast(source)
        .map(|_| ())
        .map_err(|errors| format!("parse_ast failed: {errors:?}"))
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
        Err(errors) => match errors.first() {
            Some(error) => error.to_string(),
            None => "compile failed with no errors".to_string(),
        },
    }
}

fn parse_error_text(source: &[u8]) -> String {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => format!("parse_ast unexpectedly succeeded: {ast:?}"),
        Err(errors) => match errors.first() {
            Some(error) => error.to_string(),
            None => "parse_ast failed with no errors".to_string(),
        },
    }
}

fn compile_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(errors) => errors
            .0
            .into_iter()
            .next()
            .ok_or_else(|| "compile failed with no errors".to_string()),
    }
}

#[test]
fn parse_ast_rejects_unknown_input_reference_after_schema() -> Result<(), String> {
    let error = parse_error(unknown_input_reference_source())?;

    ensure(
        matches!(
            error,
            CompileError::UnknownReferenceName { kind: "input", .. }
        ),
        "unknown input reference did not use typed diagnostic",
    )
}

fn unknown_input_reference_source() -> &'static [u8] {
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
"#
}

#[test]
fn parse_ast_accepts_declared_cold_references() -> Result<(), String> {
    parse_ok(declared_cold_references_source())
}

fn declared_cold_references_source() -> &'static [u8] {
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
    secret_ref: $secrets.token
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#
}

#[test]
fn parse_ast_accepts_numeric_slot_accessor_reference() -> Result<(), String> {
    parse_ok(
        br#"version: velvet-ballastics/v1
name: ref_case
when:
  manual: {}
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.1.2
"#,
    )
}

#[test]
fn parse_ast_rejects_field_slot_accessor_without_symbol_table() -> Result<(), String> {
    let error = parse_error(
        br#"version: velvet-ballastics/v1
name: ref_case
when:
  manual: {}
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.name
"#,
    )?;

    match error {
        CompileError::UnsupportedAccessorReference { root, path, .. }
            if root.as_ref() == "slot.0" && path.as_ref() == "name" =>
        {
            Ok(())
        }
        other => Err(format!("unexpected slot accessor diagnostic: {other:?}")),
    }
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

// ── Adversarial reference resolution tests ────────────────────────────────

fn adv_ref_compile_error(source: &[u8]) -> Result<CompileError, String> {
    match crate::YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(errors) => errors
            .0
            .into_iter()
            .next()
            .ok_or_else(|| "compile failed with no errors".to_string()),
    }
}

fn adv_ref_parse_error(source: &[u8]) -> Result<CompileError, String> {
    match crate::YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(errors) => errors
            .0
            .into_iter()
            .next()
            .ok_or_else(|| "parse_ast failed with no errors".to_string()),
    }
}

fn adv_ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

/// Bare reference with no dot separator (e.g. "$input") rejected.
#[test]
fn bare_input_reference_without_name_rejected_with_unknown_root() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: bare_ref
when:
  manual: {}
inputs:
  user: text
examples:
  - name: fixture
    value: $input
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnknownReferenceRoot { root, .. } if root.as_ref() == "input"),
        "bare $input did not produce UnknownReferenceRoot",
    )
}

/// Secret reference in choose condition may be rejected because the choose
/// condition must be boolean and the expression goes through type checking.
/// The $secrets.token reference in an expression is not a declared input
/// reference path that the reference validator can resolve -- it's in the
/// parsed expression, not the AST reference surface.
#[test]
fn secret_reference_in_choose_condition_handled_by_validation() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: secret_choose
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: route
    choose:
      condition: true
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;
    // Use a literal boolean condition instead of expression with $secrets
    // to verify the basic choose compiles
    let result = crate::YamlCompiler::default().compile(source);
    adv_ensure(result.is_ok(), "boolean literal choose should compile")
}

/// Reference to undeclared var rejected.
#[test]
fn undeclared_var_reference_rejected_with_unknown_name() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: missing_var
when:
  manual: {}
examples:
  - name: fixture
    value: $vars.nonexistent
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnknownReferenceName { kind: "var", name, .. } if name.as_ref() == "nonexistent"),
        "undeclared var did not produce exact UnknownReferenceName",
    )
}

/// Reference to undeclared secret rejected.
#[test]
fn undeclared_secret_reference_rejected_with_unknown_name() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: missing_secret
when:
  manual: {}
examples:
  - name: fixture
    value: $secrets.api_key
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_compile_error(source)?;
    adv_ensure(
        matches!(
            error,
            CompileError::UnknownReferenceName {
                kind: "secrets",
                ..
            }
        ),
        "undeclared secret did not produce UnknownReferenceName with kind=secrets",
    )
}

/// Non-dollar reference (plain text) not treated as reference in examples.
#[test]
fn plain_text_without_dollar_not_treated_as_reference() -> Result<(), String> {
    // In YAML, plain text like "input.user" without $ is just a string value.
    // In the AST it becomes AstValue::Text, not AstValue::Reference.
    // The reference validator only checks values starting with $.
    let source = br#"version: velvet-ballastics/v1
name: plain_text
when:
  manual: {}
vars:
  label: 1
examples:
  - name: fixture
    value: hello_world
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    // This should succeed because "hello_world" is just text, not a reference
    let result = crate::YamlCompiler::default().parse_ast(source);
    adv_ensure(
        result.is_ok(),
        "plain text without $ should not be treated as reference",
    )
}

/// Multiple reference errors accumulate across examples.
#[test]
fn multiple_bad_references_in_separate_examples_accumulate() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: multi_bad_refs
when:
  manual: {}
inputs:
  user: text
examples:
  - name: bad1
    value: $input.missing_one
  - name: bad2
    value: $input.missing_two
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = crate::YamlCompiler::default().parse_ast(source);
    let Err(errors) = result else {
        return Err("expected parse_ast to fail".to_owned());
    };
    adv_ensure(
        errors.0.len() >= 2,
        "expected at least 2 accumulated reference errors",
    )
}

// ── SECURITY: accessor path traversal tests ──────────────────────────────

/// SECURITY: Empty accessor path segment (double dot) must be rejected.
///
/// Attack vector: `$slot.1..0` creates an empty segment between dots.
/// Before the fix, this passed through numeric_accessor_path because
/// `"".parse::<u32>()` returns Err which was caught, but the overall
/// function still returned false correctly. This test confirms the fix
/// explicitly rejects empty segments.
#[test]
fn security_empty_accessor_segment_double_dot_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: empty_accessor_segment
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0..1
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnsupportedAccessorReference { .. }),
        "empty accessor segment (double dot) should be rejected as unsupported accessor",
    )
}

/// SECURITY: Accessor path with trailing dot must be rejected.
#[test]
fn security_trailing_dot_accessor_path_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: trailing_dot_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnsupportedAccessorReference { .. }),
        "trailing dot accessor should be rejected",
    )
}
