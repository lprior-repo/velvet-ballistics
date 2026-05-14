      value: 1
  - id: route
    choose:
      condition: true
      on_true: 2
      on_false: 2
  - id: done
    finish:
      result: 0
"#
}

fn finish_literal_source(value: &str) -> Vec<u8> {
    format!(
        "version: velvet-ballastics/v1\nname: finish_case\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: {value}\n"
    )
    .into_bytes()
}

fn ensure_compile_unsupported_constant(source: &[u8]) -> Result<(), String> {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(errors) => match errors.0.into_iter().next() {
            Some(CompileError::UnsupportedConstantValue { step: 0 }) => Ok(()),
            Some(error) => Err(format!(
                "compile did not reject finish literal with exact UnsupportedConstantValue: {error}"
            )),
            None => Err("compile returned empty CompileErrors".to_owned()),
        },
    }
}

fn ensure_choose_type_mismatch(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::TypeMismatch {
                field: "choose.condition",
                expected: "boolean",
                found: "number"
            }
        ),
        "choose condition did not use boolean type diagnostic",
    )
}

fn ensure_choose_type_found(
    error: CompileError,
    expected_found: &'static str,
) -> Result<(), String> {
    match error {
        CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "boolean",
            found,
        } if found == expected_found => Ok(()),
        _ => Err(format!(
            "choose condition did not reject {expected_found} with exact type diagnostic"
        )),
    }
}

fn ensure_choose_rejects_type(source: &[u8], expected_found: &'static str) -> Result<(), String> {
    ensure(
        compile_error_text(source) == parse_error_text(source),
        "compile and parse_ast diagnostics diverged",
    )?;
    let (compile, parse) = compile_parse_errors(source)?;
    ensure_choose_type_found(compile, expected_found)?;
    ensure_choose_type_found(parse, expected_found)
}

fn ensure_secret_result(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::SecretTaintLeak {
                field: "finish.result"
            }
        ),
        "secret result did not use taint diagnostic",
    )
}

fn ensure_secret_result_pair(source: &[u8]) -> Result<(), String> {
    ensure(
        compile_error_text(source) == parse_error_text(source),
        "compile and parse_ast diagnostics diverged",
    )?;
    let (compile, parse) = compile_parse_errors(source)?;
    ensure_secret_result(compile)?;
    ensure_secret_result(parse)
}

fn ensure_reference_error(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::UnknownReferenceName { kind: "input", .. }
        ),
        "reference error did not preempt type/taint validation",
    )
}

fn ensure_unknown_choose_slot(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::UnknownSlotType {
                field: "choose.condition",
                slot: 1
            }
        ),
        "uninitialized choose slot did not use unknown slot diagnostic",
    )
}

fn ensure_unknown_finish_slot(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::UnknownSlotType {
                field: "finish.result",
                slot: 0
            }
        ),
        "uninitialized finish slot did not use unknown slot diagnostic",
    )
}

fn ensure_slot_index_out_of_range(error: CompileError) -> Result<(), String> {
    match error {
        CompileError::SlotIndexOutOfRange { value: 65536 } => Ok(()),
        other => Err(format!(
            "slot index overflow did not use exact diagnostic: {other:?}"
        )),
    }
}

fn ensure_compiles(source: &[u8]) -> Result<(), String> {
    match YamlCompiler::default().compile(source) {
        Ok(_) => Ok(()),
        Err(errors) => Err(format!("compile unexpectedly failed: {errors}")),
    }
}

fn initialized_slot_condition_source(value: &str) -> Vec<u8> {
    format!(
        "version: velvet-ballastics/v1\nname: choose_case\nwhen:\n  manual: {{}}\nsteps:\n  - id: captured\n    save:\n      value: {value}\n  - id: route\n    choose:\n      condition: 0\n      on_true: 2\n      on_false: 2\n  - id: done\n    finish:\n      result: null\n"
    )
    .into_bytes()
}

fn literal_choose_condition_source(condition: &str) -> Vec<u8> {
    format!(
        "version: velvet-ballastics/v1\nname: choose_case\nwhen:\n  manual: {{}}\nsteps:\n  - id: route\n    choose:\n      condition:{condition}\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: 0\n"
    )
    .into_bytes()
}

fn finish_result_fragment_source(result: &str) -> Vec<u8> {
    format!(
        "version: velvet-ballastics/v1\nname: finish_case\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result:{result}\n"
    )
    .into_bytes()
}

fn large_finish_slot_source() -> Vec<u8> {
    let prefix = "version: velvet-ballastics/v1\nname: finish_case\nwhen:\n  manual: {}\nsteps:\n";
    let saves = (0_u32..65_536)
        .map(|index| format!("  - id: pad_{index}\n    save:\n      value: null\n"))
        .collect::<String>();
    format!("{prefix}{saves}  - id: done\n    finish:\n      result: 65536\n").into_bytes()
}

fn secret_tainted_finish_ast() -> WorkflowAst {
    WorkflowAst {
        version: "velvet-ballastics/v1".into(),
        name: "taint_case".into(),
        trigger: TriggerAst::Manual { mark: None },
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

