use super::validate_workflow_ast;
use crate::ast::{
    AstExpression, AstMapEntry, AstValue, StepAst, StepKindAst, TriggerAst, WorkflowAst,
};
use crate::{CompileError, YamlCompiler, YamlLimits};
use vb_core::{CompiledNodeKind, CompiledWorkflow, SlotValue, StepIdx};

fn compile_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(error) => Ok(error),
    }
}

fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(error) => Ok(error),
    }
}

fn compile_parse_errors(source: &[u8]) -> Result<(CompileError, CompileError), String> {
    Ok((compile_error(source)?, parse_error(source)?))
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

fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn ensure_pair(source: &[u8], check: fn(CompileError) -> Result<(), String>) -> Result<(), String> {
    ensure(
        compile_error_text(source) == parse_error_text(source),
        "compile and parse_ast diagnostics diverged",
    )?;
    check(compile_error(source)?)?;
    check(parse_error(source)?)
}

fn ensure_parse_ok(source: &[u8]) -> Result<(), String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("parse_ast unexpectedly failed: {error}")),
    }
}

fn ensure_compile_and_parse_ok(source: &[u8]) -> Result<(), String> {
    ensure_compiles(source)?;
    ensure_parse_ok(source)
}

fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, String> {
    YamlCompiler::default()
        .compile(source)
        .map_err(|error| format!("compile unexpectedly failed: {error}"))
}

fn parse_workflow(source: &[u8]) -> Result<WorkflowAst, String> {
    YamlCompiler::default()
        .parse_ast(source)
        .map_err(|error| format!("parse_ast unexpectedly failed: {error}"))
}

fn ensure_value(
    actual: SlotValue,
    expected: SlotValue,
    message: &'static str,
) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{message}: expected {expected:?}, got {actual:?}"))
    }
}

fn ensure_expression(
    actual: AstExpression,
    expected: AstExpression,
    message: &'static str,
) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{message}: expected {expected:?}, got {actual:?}"))
    }
}

fn finish_expression(source: &[u8]) -> Result<AstExpression, String> {
    let ast = parse_workflow(source)?;
    match ast.steps.first().map(|step| &step.kind) {
        Some(StepKindAst::Finish { result }) => Ok(result.clone()),
        Some(kind) => Err(format!("first step was not finish: {kind:?}")),
        None => Err("workflow did not contain a finish step".to_owned()),
    }
}

fn choose_expression(source: &[u8]) -> Result<AstExpression, String> {
    let ast = parse_workflow(source)?;
    match ast.steps.get(1).map(|step| &step.kind) {
        Some(StepKindAst::Choose { condition, .. }) => Ok(condition.clone()),
        Some(kind) => Err(format!("second step was not choose: {kind:?}")),
        None => Err("workflow did not contain a choose step".to_owned()),
    }
}

fn ensure_finish_const_value(source: &[u8], expected: SlotValue) -> Result<(), String> {
    let workflow = compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::new(0))
        .ok_or_else(|| "compiled workflow did not contain step 0".to_owned())?;
    match node.kind {
        CompiledNodeKind::FinishConst { value } => workflow
            .constant(value)
            .copied()
            .ok_or_else(|| format!("finish const referenced missing constant {value:?}"))
            .and_then(|actual| ensure_value(actual, expected, "finish const payload mismatch")),
        kind => Err(format!("finish did not lower to FinishConst: {kind:?}")),
    }
}

fn ensure_choose_slot_node(source: &[u8]) -> Result<(), String> {
    let workflow = compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::new(1))
        .ok_or_else(|| "compiled workflow did not contain step 1".to_owned())?;
    match node.kind {
        CompiledNodeKind::ChooseSlot {
            condition,
            on_true,
            on_false,
        } if condition.get() == 0 && on_true.get() == 2 && on_false.get() == 2 => Ok(()),
        kind => Err(format!(
            "initialized boolean slot did not lower to exact ChooseSlot: {kind:?}"
        )),
    }
}

fn ensure_literal_choose_value(
    workflow: &CompiledWorkflow,
    value: vb_core::ConstIdx,
) -> Result<(), String> {
    workflow
        .constant(value)
        .copied()
        .ok_or_else(|| format!("literal choose referenced missing constant {value:?}"))
        .and_then(|actual| {
            ensure_value(
                actual,
                SlotValue::Bool(true),
                "choose literal payload mismatch",
            )
        })
}

fn ensure_literal_choose_node(source: &[u8]) -> Result<(), String> {
    let workflow = compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::new(1))
        .ok_or_else(|| "compiled workflow did not contain step 1".to_owned())?;
    match node.kind {
        CompiledNodeKind::SetConst {
            output,
            value,
            next,
        } if output.get() == 1 && next.get() == 2 => ensure_literal_choose_value(&workflow, value),
        kind => Err(format!(
            "literal boolean choose did not lower to exact SetConst: {kind:?}"
        )),
    }
}

fn initialized_boolean_slot_choose_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: choose_case
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
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

fn literal_boolean_choose_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: choose_case
when:
  manual: {}
steps:
  - id: value
    save:
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
        Err(CompileError::UnsupportedConstantValue { step: 0 }) => Ok(()),
        Err(error) => Err(format!(
            "compile did not reject finish literal with exact UnsupportedConstantValue: {error}"
        )),
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
        Err(error) => Err(format!("compile unexpectedly failed: {error}")),
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
        kind: StepKindAst::Finish {
            result: AstExpression::Reference("$secret.token".into()),
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
        - $secret.token
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
        token: $secret.token
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

fn ensure_supported_scalar_finish_const(value: &str, expected: SlotValue) -> Result<(), String> {
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

#[test]
fn compile_and_parse_ast_reject_non_boolean_choose_condition() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: type_case
when:
  manual: {}
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
"#;

    ensure_pair(source, ensure_choose_type_mismatch)
}

#[test]
fn compile_accepts_initialized_boolean_slot_choose_condition() -> Result<(), String> {
    let source = initialized_boolean_slot_choose_source();

    ensure_compile_and_parse_ok(source)?;
    ensure_expression(
        choose_expression(source)?,
        AstExpression::Slot(vb_core::SlotIdx::new(0)),
        "initialized boolean slot AST condition mismatch",
    )?;
    ensure_choose_slot_node(source)
}

#[test]
fn compile_and_parse_ast_accept_boolean_literal_choose_condition() -> Result<(), String> {
    let source = literal_boolean_choose_source();

    ensure_compile_and_parse_ok(source)?;
    ensure_expression(
        choose_expression(source)?,
        AstExpression::Literal(AstValue::Bool(true)),
        "boolean literal AST condition mismatch",
    )?;
    ensure_literal_choose_node(source)
}

#[test]
fn compile_and_parse_ast_reject_uninitialized_choose_slot() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: choose_case
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: 1
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_unknown_choose_slot)
}

#[test]
fn compile_and_parse_ast_reject_choose_slot_index_out_of_range_exactly() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: choose_case
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: 65536
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: null
"#;

    ensure_pair(source, ensure_slot_index_out_of_range)
}

#[test]
fn compile_and_parse_ast_reject_missing_finish_slot() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: finish_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_unknown_finish_slot)
}

#[test]
fn parse_ast_rejects_finish_slot_index_out_of_range_exactly() -> Result<(), String> {
    let source = large_finish_slot_source();
    let compiler = YamlCompiler::new(YamlLimits {
        max_source_bytes: 4_000_000,
        max_depth: 64,
        max_nodes: 500_000,
        max_sequence_len: 70_000,
        max_mapping_entries: 1_024,
        max_scalar_bytes: 65_536,
    });

    match compiler.parse_ast(&source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(error) => ensure_slot_index_out_of_range(error),
    }
}

#[test]
fn validator_rejects_secret_tainted_finish_result() -> Result<(), String> {
    let ast = secret_tainted_finish_ast();

    match validate_workflow_ast(&ast) {
        Ok(()) => Err("validator unexpectedly accepted secret result".to_owned()),
        Err(error) => ensure_secret_result(error),
    }
}

#[test]
fn compile_and_parse_ast_reject_secret_reference_finish_result_exactly() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: taint_case
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: done
    finish:
      result: $secret.token
"#;

    ensure_secret_result_pair(source)
}

#[test]
fn compile_and_parse_ast_reject_secret_slot_finish_result_exactly() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: taint_case
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: capture
    save:
      value: $secret.token
  - id: done
    finish:
      result: 0
"#;

    ensure_secret_result_pair(source)
}

#[test]
fn compile_and_parse_ast_reject_nested_secret_slot_finish_results() -> Result<(), String> {
    ensure_secret_result_pair(nested_secret_list_finish_source())?;
    ensure_secret_result_pair(nested_secret_object_finish_source())
}

#[test]
fn parse_ast_accepts_clean_public_finish_references_exactly() -> Result<(), String> {
    ensure_expression(
        finish_expression(clean_input_finish_source())?,
        AstExpression::Reference("$input.user".into()),
        "input finish reference was not retained exactly",
    )?;
    ensure_expression(
        finish_expression(clean_vars_finish_source())?,
        AstExpression::Reference("$vars.label".into()),
        "vars finish reference was not retained exactly",
    )
}

#[test]
fn compile_and_parse_ast_reject_non_boolean_literal_choose_conditions() -> Result<(), String> {
    ensure_choose_rejects_type(&literal_choose_condition_source(" public"), "text")?;
    ensure_choose_rejects_type(&literal_choose_condition_source(" null"), "null")?;
    ensure_choose_rejects_type(&literal_choose_condition_source("\n        - true"), "list")?;
    ensure_choose_rejects_type(
        &literal_choose_condition_source("\n        value: true"),
        "object",
    )
}

#[test]
fn compile_and_parse_ast_reject_initialized_non_boolean_slot_conditions() -> Result<(), String> {
    let text_source = initialized_slot_condition_source("public");
    let null_source = initialized_slot_condition_source("null");
    let list_source = initialized_slot_condition_source("[true]");
    let object_source = initialized_slot_condition_source("{ value: true }");

    ensure_choose_rejects_type(&text_source, "text")?;
    ensure_choose_rejects_type(&null_source, "null")?;
    ensure_choose_rejects_type(&list_source, "list")?;
    ensure_choose_rejects_type(&object_source, "object")
}

#[test]
fn compile_and_parse_ast_reject_secret_object_finish_result_exactly() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: taint_case
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: done
    finish:
      result:
        token: $secret.token
"#;

    ensure_secret_result_pair(source)
}

#[test]
fn compile_and_parse_ast_reject_uninitialized_forward_finish_slot() -> Result<(), String> {
    ensure_pair(forward_finish_slot_source(), ensure_forward_finish_slot)
}

#[test]
fn parse_ast_accepts_clean_literal_finish_results() -> Result<(), String> {
    ensure_expression(
        finish_expression(&finish_result_fragment_source(" public"))?,
        AstExpression::Literal(AstValue::Text("public".into())),
        "text finish literal was not retained exactly",
    )?;
    ensure_expression(
        finish_expression(&finish_result_fragment_source("\n        - public"))?,
        AstExpression::Literal(AstValue::Sequence(vec![AstValue::Text("public".into())])),
        "list finish literal was not retained exactly",
    )?;
    ensure_expression(
        finish_expression(&finish_result_fragment_source("\n        value: public"))?,
        AstExpression::Literal(AstValue::Mapping(vec![AstMapEntry {
            name: "value".into(),
            value: AstValue::Text("public".into()),
            mark: None,
        }])),
        "object finish literal was not retained exactly",
    )
}

#[test]
fn compile_and_parse_ast_accept_supported_scalar_finish_literals() -> Result<(), String> {
    ensure_supported_scalar_finish_const("null", SlotValue::Null)?;
    ensure_supported_scalar_finish_const("true", SlotValue::Bool(true))?;
    ensure_supported_scalar_finish_const("false", SlotValue::Bool(false))?;
    ensure_supported_scalar_finish_const("42", SlotValue::I64(42))?;
    ensure_supported_scalar_finish_const("-7", SlotValue::I64(-7))
}

#[test]
fn finish_result_zero_keeps_current_slot_zero_semantics() -> Result<(), String> {
    let zero_source = finish_literal_source("0");

    ensure_pair(&zero_source, ensure_unknown_finish_slot)
}

#[test]
fn compile_rejects_unsupported_finish_literals_with_exact_error() -> Result<(), String> {
    let text_source = finish_literal_source("public");
    let list_source = finish_literal_source("[public]");
    let object_source = finish_literal_source("{ value: public }");

    ensure_compile_unsupported_constant(&text_source)?;
    ensure_compile_unsupported_constant(&list_source)?;
    ensure_compile_unsupported_constant(&object_source)
}

#[test]
fn parse_ast_accepts_supported_scalar_finish_literals() -> Result<(), String> {
    ensure_supported_scalar_finish_asts()
}

#[test]
fn reference_errors_preempt_type_taint_errors() -> Result<(), String> {
    ensure_pair(reference_preempt_source(), ensure_reference_error)
}

#[test]
fn type_taint_errors_preempt_control_flow_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: preempt_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_choose_type_mismatch)
}

#[test]
fn type_taint_errors_preempt_backward_branch_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: preempt_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: route
    choose:
      condition: 0
      on_true: 0
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_choose_type_mismatch)
}

#[test]
fn type_taint_errors_preempt_self_branch_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: preempt_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: route
    choose:
      condition: 0
      on_true: 1
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_choose_type_mismatch)
}
