use super::validate_workflow_ast;
use crate::ast::{
    AstExpression, AstMapEntry, AstValue, StepAst, StepKindAst, StepPrimitiveAst, TriggerAst,
    WorkflowAst,
};
use crate::expression::{ExpressionLiteral, ParsedExpression};
use crate::{CompileError, YamlCompiler, YamlLimits};
use vb_core::{CompiledNodeKind, CompiledWorkflow, ConstValue, SlotIdx, StepIdx};

fn compile_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(errors) => errors
            .0
            .into_iter()
            .next()
            .ok_or_else(|| "compile returned empty CompileErrors".to_owned()),
    }
}

fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(errors) => errors
            .0
            .into_iter()
            .next()
            .ok_or_else(|| "parse_ast returned empty CompileErrors".to_owned()),
    }
}

fn compile_parse_errors(source: &[u8]) -> Result<(CompileError, CompileError), String> {
    Ok((compile_error(source)?, parse_error(source)?))
}

fn compile_error_text(source: &[u8]) -> String {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => format!("compile unexpectedly succeeded: {workflow:?}"),
        Err(errors) => match errors.first() {
            Some(error) => error.to_string(),
            None => "compile returned empty CompileErrors".to_owned(),
        },
    }
}

fn parse_error_text(source: &[u8]) -> String {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => format!("parse_ast unexpectedly succeeded: {ast:?}"),
        Err(errors) => match errors.first() {
            Some(error) => error.to_string(),
            None => "parse_ast returned empty CompileErrors".to_owned(),
        },
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
        Err(errors) => Err(format!("parse_ast unexpectedly failed: {errors}")),
    }
}

fn ensure_compile_and_parse_ok(source: &[u8]) -> Result<(), String> {
    ensure_compiles(source)?;
    ensure_parse_ok(source)
}

fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, String> {
    YamlCompiler::default()
        .compile(source)
        .map_err(|errors| format!("compile unexpectedly failed: {errors}"))
}

fn parse_workflow(source: &[u8]) -> Result<WorkflowAst, String> {
    YamlCompiler::default()
        .parse_ast(source)
        .map_err(|errors| format!("parse_ast unexpectedly failed: {errors}"))
}

fn ensure_value(
    actual: ConstValue,
    expected: ConstValue,
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

fn parsed_reference_expression(reference: &'static str) -> AstExpression {
    AstExpression::Parsed(Box::new(ParsedExpression::Reference(reference.into())))
}

fn parsed_text_expression(value: &'static str) -> AstExpression {
    AstExpression::Parsed(Box::new(ParsedExpression::Literal(
        ExpressionLiteral::Text(value.into()),
    )))
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

fn ensure_finish_const_value(source: &[u8], expected: ConstValue) -> Result<(), String> {
    let workflow = compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::new(0))
        .ok_or_else(|| "compiled workflow did not contain step 0".to_owned())?;
    match &node.kind {
        CompiledNodeKind::SetConst { value }
            if node.output == Some(SlotIdx::new(0)) && node.next == Some(StepIdx::new(1)) =>
        {
            workflow
                .constant(*value)
                .copied()
                .ok_or_else(|| format!("finish literal referenced missing constant {value:?}"))
                .and_then(|actual| ensure_value(actual, expected, "finish const payload mismatch"))
        }
        kind => Err(format!(
            "finish did not lower to SetConst -> Finish: {kind:?}"
        )),
    }
}

fn ensure_choose_slot_node(source: &[u8]) -> Result<(), String> {
    let workflow = compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::new(1))
        .ok_or_else(|| "compiled workflow did not contain step 1".to_owned())?;
    match &node.kind {
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } if branches.len() == 1 && *otherwise == Some(StepIdx::new(2)) => match branches.first() {
            Some(branch) if branch.condition.get() == 0 && branch.target.get() == 2 => Ok(()),
            other => Err(format!("unexpected ChooseSlot branch: {other:?}")),
        },
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
                ConstValue::Bool(true),
                "choose literal payload mismatch",
            )
        })
}

fn ensure_literal_choose_node(source: &[u8]) -> Result<(), String> {
    let workflow = compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::new(1))
        .ok_or_else(|| "compiled workflow did not contain step 1".to_owned())?;
    match &node.kind {
        CompiledNodeKind::SetConst { value }
            if node.output == Some(SlotIdx::new(1)) && node.next == Some(StepIdx::new(2)) =>
        {
            ensure_literal_choose_value(&workflow, *value)
        }
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
