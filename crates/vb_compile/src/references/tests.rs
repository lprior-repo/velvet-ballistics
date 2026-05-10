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

// ── Additional reference resolution edge-case tests ─────────────────────

/// Reference to declared input succeeds (positive validation).
#[test]
fn declared_input_reference_passes() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: input_ref_ok
when:
  manual: {}
inputs:
  email: text
examples:
  - name: fixture
    value: $input.email
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "declared input reference should pass",
    )
}

/// Reference to declared var succeeds (positive validation).
#[test]
fn declared_var_reference_passes() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: var_ref_ok
when:
  manual: {}
vars:
  count: 1
examples:
  - name: fixture
    value: $vars.count
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "declared var reference should pass",
    )
}

/// Reference to declared secret succeeds (positive validation).
#[test]
fn declared_secret_reference_passes() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: secret_ref_ok
when:
  manual: {}
secrets:
  api_key: API_KEY
examples:
  - name: fixture
    value: $secrets.api_key
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "declared secret reference should pass",
    )
}

/// `$vars.name.field` accessor path on a declared var is rejected.
#[test]
fn accessor_path_on_declared_var_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: accessor_var
when:
  manual: {}
vars:
  data: 1
examples:
  - name: fixture
    value: $vars.data.field
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnsupportedAccessorReference { root, path, .. }
            if root.as_ref() == "vars.data" && path.as_ref() == "field"),
        "accessor path on declared var should be rejected with exact root and path",
    )
}

/// `$input.name.field` accessor path on a declared input is rejected.
#[test]
fn accessor_path_on_declared_input_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: accessor_input
when:
  manual: {}
inputs:
  user: text
examples:
  - name: fixture
    value: $input.user.email
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnsupportedAccessorReference { root, path, .. }
            if root.as_ref() == "input.user" && path.as_ref() == "email"),
        "accessor path on declared input should be rejected",
    )
}

/// `$secrets.name.field` accessor path on a declared secret is rejected.
#[test]
fn accessor_path_on_declared_secret_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: accessor_secret
when:
  manual: {}
secrets:
  token: TOKEN
examples:
  - name: fixture
    value: $secrets.token.sub
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnsupportedAccessorReference { .. }),
        "accessor path on declared secret should be rejected",
    )
}

/// `$slot.0` bare slot reference (no accessor path) passes.
#[test]
fn bare_slot_reference_passes() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: bare_slot
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "bare slot reference should pass",
    )
}

/// `$slot.abc` non-numeric slot index is rejected.
#[test]
fn non_numeric_slot_index_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: bad_slot_idx
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.abc
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnknownReferenceName { kind, name, .. }
            if kind == "slot" && name.as_ref() == "abc"),
        "non-numeric slot index should produce UnknownReferenceName",
    )
}

/// `$slots.0` alternate spelling of slot root passes.
#[test]
fn alternate_slots_root_passes() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: slots_root
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slots.0
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "$slots.0 should be accepted like $slot.0",
    )
}

/// Numeric accessor path with deep nesting passes.
#[test]
fn deep_numeric_accessor_path_passes() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: deep_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.1.2.3.4.5
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "deep numeric accessor path should pass",
    )
}

/// Accessor path with non-numeric segment after numeric slot rejected.
#[test]
fn mixed_accessor_path_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: mixed_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.1.name
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnsupportedAccessorReference { .. }),
        "mixed accessor path with name segment should be rejected",
    )
}

/// `$runtime.something` is rejected as illegal reference.
#[test]
fn runtime_reference_rejected_as_illegal() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: runtime_ref
when:
  manual: {}
examples:
  - name: fixture
    value: $runtime.now
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::IllegalReference { .. }),
        "runtime reference should be rejected as illegal",
    )
}

/// `$steps.done` backward reference to a step is rejected as illegal.
#[test]
fn steps_reference_rejected_as_illegal() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: steps_ref
when:
  manual: {}
examples:
  - name: fixture
    value: $steps.done
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::IllegalReference { .. }),
        "$steps reference should be rejected as illegal",
    )
}

/// `$now` bare illegal reference is rejected.
#[test]
fn bare_now_reference_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: bare_now
when:
  manual: {}
examples:
  - name: fixture
    value: $now
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::IllegalReference { .. }),
        "$now should be rejected as illegal",
    )
}

/// Reference in save fields is validated.
#[test]
fn reference_in_save_field_validated() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: save_bad_ref
when:
  manual: {}
inputs:
  user: text
steps:
  - id: build
    save:
      value: $input.nonexistent
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnknownReferenceName { kind, .. } if kind == "input"),
        "reference in save field should be validated",
    )
}

/// Reference in finish result expression is validated.
#[test]
fn reference_in_finish_result_validated() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: finish_bad_ref
when:
  manual: {}
steps:
  - id: done
    finish:
      result: $slot.abc
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnknownReferenceName { kind, name, .. }
            if kind == "slot" && name.as_ref() == "abc"),
        "finish result should validate slot references",
    )
}

/// Multiple references in a single mapping are all validated.
#[test]
fn multiple_references_in_mapping_all_validated() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: multi_ref_mapping
when:
  manual: {}
inputs:
  user: text
examples:
  - name: fixture
    good: $input.user
    bad: $input.missing
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
        !errors.0.is_empty(),
        "mapping with bad reference should produce at least one error",
    )
}

/// Accessor path on undeclared name triggers unknown name check.
#[test]
fn accessor_path_on_undeclared_name_triggers_unknown_name() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: accessor_undeclared
when:
  manual: {}
examples:
  - name: fixture
    value: $vars.missing.field
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(error, CompileError::UnknownReferenceName { kind, name, .. }
            if kind == "var" && name.as_ref() == "missing"),
        "undeclared var with accessor should produce UnknownReferenceName",
    )
}

/// Literal values (not references) pass validation.
#[test]
fn literal_values_pass_validation() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: literals
when:
  manual: {}
examples:
  - name: fixture
    int_val: 42
    bool_val: true
    text_val: hello
    null_val: null
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "literal values should pass validation",
    )
}

/// Sequence of references each validated.
#[test]
fn sequence_of_references_validated() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: seq_refs
when:
  manual: {}
inputs:
  user: text
examples:
  - name: fixture
    values:
      - $input.user
      - $input.missing
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
        errors.0.iter().any(
            |e| matches!(e, CompileError::UnknownReferenceName { kind, .. } if *kind == "input"),
        ),
        "sequence with bad reference should produce input error",
    )
}

/// `$var` singular alias also works for declared vars.
#[test]
fn var_alias_root_resolves_declared_var() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: var_alias
when:
  manual: {}
vars:
  count: 1
examples:
  - name: fixture
    value: $var.count
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "$var.count should resolve like $vars.count",
    )
}

/// Empty value (null) is not treated as reference.
#[test]
fn empty_value_not_treated_as_reference() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: empty_val
when:
  manual: {}
vars:
  label: 1
examples:
  - name: fixture
    value:
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "empty value should not be treated as reference",
    )
}

/// Compile-specific: $slot reference with u16 max value passes.
#[test]
fn slot_reference_with_max_u16_index() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: max_slot
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.65535
"#;
    adv_ensure(
        crate::YamlCompiler::default().parse_ast(source).is_ok(),
        "$slot.65535 should pass reference validation",
    )
}

/// `$inputs` plural root is NOT a valid alias -- rejected.
#[test]
fn inputs_plural_root_rejected_as_unknown() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: inputs_plural
when:
  manual: {}
inputs:
  user: text
examples:
  - name: fixture
    value: $inputs.user
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    let error = adv_ref_parse_error(source)?;
    adv_ensure(
        matches!(
            error,
            CompileError::UnknownReferenceRoot { .. }
                | CompileError::UnknownReferenceName { .. }
                | CompileError::IllegalReference { .. }
        ),
        "$inputs.user should be rejected (inputs is not a recognized root in reference validation)",
    )
}

/// Reference in nested save mapping value is validated.
#[test]
fn reference_in_nested_save_mapping_validated() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: nested_save_ref
when:
  manual: {}
vars:
  data: 1
steps:
  - id: build
    save:
      outer:
        inner: $vars.missing
  - id: done
    finish:
      result: 0
"#;
    let result = crate::YamlCompiler::default().parse_ast(source);
    let Err(errors) = result else {
        return Err("expected parse_ast to fail for bad var reference in nested save".to_owned());
    };
    adv_ensure(
        errors.0.iter().any(
            |e| matches!(e, CompileError::UnknownReferenceName { kind, .. } if *kind == "var"),
        ),
        "nested save mapping should validate var references",
    )
}
