        inputs: Vec::new(),
        vars: Vec::new(),
        secrets: vec![AstMapEntry {
            name: "token".into(),
            value: "TOKEN".into(),
            mark: None,
        }],
        result: Vec::new(),
        examples: Vec::new(),
        steps: vec![secret_tainted_finish_step()],
        mark: None,
    }
}

fn secret_tainted_finish_step() -> StepAst {
    StepAst {
        id: "done".into(),
        name: None,
        primitive: StepPrimitiveAst::Finish,
        kind: StepKindAst::Finish {
            result: AstExpression::Reference("$secrets.token".into()),
        },
        mark: None,
    }
}

fn nested_secret_list_finish_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: taint_case
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: capture
    save:
      value:
        - $secrets.token
  - id: done
    finish:
      result: 0
"#
}

fn nested_secret_object_finish_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: taint_case
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: capture
    save:
      value:
        token: $secrets.token
  - id: done
    finish:
      result: 0
"#
}

fn clean_input_finish_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: clean_case
when:
  manual: {}
inputs:
  user: text
steps:
  - id: done
    finish:
      result: $input.user
"#
}

fn clean_vars_finish_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: clean_case
when:
  manual: {}
vars:
  label: true
steps:
  - id: done
    finish:
      result: $vars.label
"#
}

fn forward_finish_slot_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: finish_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 1
"#
}

fn reference_preempt_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: preempt_case
when:
  manual: {}
inputs:
  user: text
examples:
  - name: bad_ref
    value: $input.missing
steps:
  - id: build
    save:
      value: 1
  - id: route
    choose:
      condition: 0
      on_true: 2
      on_false: 2
  - id: done
    finish:
      result: 0
"#
}

fn ensure_forward_finish_slot(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::UnknownSlotType {
                field: "finish.result",
                slot: 1
            }
        ),
        "forward finish slot did not use unknown slot diagnostic",
    )
}

fn ensure_supported_scalar_finish_const(value: &str, expected: ConstValue) -> Result<(), String> {
    let source = finish_literal_source(value);

    ensure_compile_and_parse_ok(&source)?;
    ensure_finish_const_value(&source, expected)
}

fn ensure_supported_scalar_finish_ast(
    value: &str,
    expected: AstValue,
    message: &'static str,
) -> Result<(), String> {
    let source = finish_literal_source(value);

    ensure_expression(
        finish_expression(&source)?,
        AstExpression::Literal(expected),
        message,
    )
}

fn ensure_supported_scalar_finish_asts() -> Result<(), String> {
    ensure_supported_boolean_and_null_finish_asts()?;
    ensure_supported_integer_finish_asts()
}

fn ensure_supported_boolean_and_null_finish_asts() -> Result<(), String> {
    ensure_supported_scalar_finish_ast("null", AstValue::Null, "null literal mismatch")?;
    ensure_supported_scalar_finish_ast("true", AstValue::Bool(true), "true literal mismatch")?;
    ensure_supported_scalar_finish_ast("false", AstValue::Bool(false), "false literal mismatch")
}

fn ensure_supported_integer_finish_asts() -> Result<(), String> {
    ensure_supported_scalar_finish_ast("42", AstValue::I64(42), "positive integer mismatch")?;
    ensure_supported_scalar_finish_ast("-7", AstValue::I64(-7), "negative integer mismatch")
}


mod tests_util;
mod tests_util2;
mod tests1;
mod tests2;
mod tests3;
mod tests4;
