use super::*;
use crate::expression::{BinaryOp, ParsedExpression};
use crate::{CompileError, YamlCompiler};

fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn ensure_mark(
    mark: crate::SourceMark,
    source: &str,
    needle: &'static str,
    line: usize,
    column: usize,
) -> Result<(), String> {
    let index = source
        .find(needle)
        .ok_or_else(|| format!("missing expected source needle: {needle}"))?;
    if mark.available && mark.index == index && mark.line == line && mark.column == column {
        Ok(())
    } else {
        Err(format!(
            "mark mismatch for {needle}: got {mark:?}, expected index={index}, line={line}, column={column}"
        ))
    }
}

fn parse(source: &[u8]) -> Result<WorkflowAst, String> {
    YamlCompiler::default()
        .parse_ast(source)
        .map_err(|errors| format!("parse_ast failed: {errors:?}"))
}

fn parse_err(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(errors) => match errors.first() {
            Some(error) => Ok(error.clone()),
            None => Err("CompileErrors was empty".to_string()),
        },
    }
}

fn compile_err(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(errors) => match errors.first() {
            Some(error) => Ok(error.clone()),
            None => Err("CompileErrors was empty".to_string()),
        },
    }
}

fn ensure_parse_compile_error_parity(source: &[u8]) -> Result<(), String> {
    let parse = parse_err(source)?;
    let compile = compile_err(source)?;
    ensure(
        format!("{parse:?}") == format!("{compile:?}"),
        "parse_ast and compile diagnostics diverged",
    )
}

fn first_choose_condition(source: &[u8]) -> Result<AstExpression, String> {
    let ast = parse(source)?;
    match ast.steps.first().map(|step| &step.kind) {
        Some(StepKindAst::Choose { condition, .. }) => Ok(condition.clone()),
        Some(kind) => Err(format!("first step was not choose: {kind:?}")),
        None => Err("workflow did not contain a step".to_owned()),
    }
}

fn parsed_binary(expression: &AstExpression) -> Result<BinaryOp, String> {
    match expression {
        AstExpression::Parsed(parsed) => parsed_binary_op(parsed),
        other => Err(format!("expected parsed expression, got {other:?}")),
    }
}

fn parsed_binary_op(expression: &ParsedExpression) -> Result<BinaryOp, String> {
    match expression {
        ParsedExpression::Binary { op, .. } => Ok(*op),
        other => Err(format!("expected parsed binary expression, got {other:?}")),
    }
}

#[test]
fn parse_ast_retains_vars_and_examples_surface() -> Result<(), String> {
    let ast = parse(
        b"version: velvet-ballastics/v1\nname: ast_surface\nwhen:\n  manual: {}\nvars:\n  retries: 3\nexamples:\n  - name: happy\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
    )?;

    ensure(ast.vars.len() == 1, "vars surface not retained")?;
    ensure(ast.examples.len() == 1, "examples surface not retained")?;
    ensure(ast.steps.len() == 2, "step surface not retained")?;
    Ok(())
}

#[test]
fn parse_ast_rejects_ipc_like_compile_boundary() -> Result<(), String> {
    let error = parse_err(
        b"version: velvet-ballastics/v1\nname: ast_surface\nwhen:\n  ipc: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
    )?;

    ensure(
        matches!(error, CompileError::UnknownTriggerKind { .. }),
        "ipc trigger did not fail with UnknownTriggerKind",
    )?;
    Ok(())
}

#[test]
fn parse_ast_rejects_unknown_trigger_fields() -> Result<(), String> {
    let error = parse_err(
        b"version: velvet-ballastics/v1\nname: ast_surface\nwhen:\n  manual:\n    extra: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
    )?;

    ensure(
        matches!(error, CompileError::UnknownTriggerField { .. }),
        "unknown trigger field did not fail with UnknownTriggerField",
    )?;
    Ok(())
}

#[test]
fn parse_ast_rejects_unknown_step_fields() -> Result<(), String> {
    let error = parse_err(
        b"version: velvet-ballastics/v1\nname: ast_surface\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    mystery: true\n    finish:\n      result: 0\n",
    )?;

    ensure(
        matches!(error, CompileError::UnknownStepField { .. }),
        "unknown step field did not fail with UnknownStepField",
    )?;
    Ok(())
}

#[test]
fn parse_ast_keeps_available_source_marks() -> Result<(), String> {
    let source = "version: velvet-ballastics/v1\nname: ast_surface\nwhen:\n  manual: {}\nvars:\n  retries: 3\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n";
    let ast = parse(source.as_bytes())?;
    let mark = ast.mark.ok_or_else(|| "workflow mark missing".to_owned())?;
    let trigger_mark = match ast.trigger {
        TriggerAst::Manual { mark } => mark.ok_or_else(|| "trigger mark missing".to_owned())?,
        _ => return Err("expected manual trigger".to_owned()),
    };
    let var_mark = ast
        .vars
        .first()
        .and_then(|entry| entry.mark)
        .ok_or_else(|| "vars mark missing".to_owned())?;
    let step_mark = ast
        .steps
        .first()
        .and_then(|step| step.mark)
        .ok_or_else(|| "step mark missing".to_owned())?;

    ensure(mark.available, "workflow mark unavailable")?;
    ensure(mark.index == 0, "workflow mark index not document start")?;
    ensure_mark(trigger_mark, source, "manual", 4, 2)?;
    ensure_mark(var_mark, source, "retries", 6, 2)?;
    ensure_mark(step_mark, source, "id: build_result", 8, 4)?;
    Ok(())
}

#[test]
fn parse_ast_retains_source_primitive_identity() -> Result<(), String> {
    let ast = parse(
        br#"version: velvet-ballastics/v1
name: ast_primitives
when:
  manual: {}
steps:
  - id: seed
    save:
      value: 0
  - id: call_run
    run:
      action: 1
      input: 0
  - id: call_do
    do:
      action: 2
      input: 0
  - id: write_save
    save:
      value: 4
  - id: done
    finish:
      result: 0
"#,
    )?;

    let run_step = ast
        .steps
        .get(1)
        .ok_or_else(|| "missing run step".to_owned())?;
    let do_step = ast
        .steps
        .get(2)
        .ok_or_else(|| "missing do step".to_owned())?;
    let save_step = ast
        .steps
        .get(3)
        .ok_or_else(|| "missing save step".to_owned())?;

    ensure(
        run_step.primitive == StepPrimitiveAst::Run,
        "run source primitive was not retained",
    )?;
    ensure(
        do_step.primitive == StepPrimitiveAst::Do,
        "do source primitive was not retained",
    )?;
    ensure(
        save_step.primitive == StepPrimitiveAst::Save,
        "save source primitive was not retained",
    )
}

#[test]
fn parse_ast_rejects_multiple_triggers_before_lowering() -> Result<(), String> {
    let error = parse_err(
        br#"version: velvet-ballastics/v1
name: ast_surface
when:
  manual: {}
  event:
    name: ready
steps:
  - id: done
    finish:
      result: 0
"#,
    )?;

    ensure(
        matches!(error, CompileError::InvalidTriggerCount { count: 2 }),
        "multiple triggers did not fail with InvalidTriggerCount",
    )
}

#[test]
fn parse_ast_reports_multiple_primitive_step_index() -> Result<(), String> {
    let error = parse_err(
        br#"version: velvet-ballastics/v1
name: ast_surface
when:
  manual: {}
steps:
  - id: first
    save:
      value: 1
  - id: broken
    save:
      value: 2
    finish:
      result: 1
"#,
    )?;

    ensure(
        matches!(error, CompileError::MultipleStepPrimitives { step: 1 }),
        "multiple primitives did not report the malformed step index",
    )
}

#[test]
fn parse_ast_exposes_parsed_expression_public_surface() -> Result<(), String> {
    let expression = first_choose_condition(
        br#"version: velvet-ballastics/v1
name: ast_surface
when:
  manual: {}
inputs:
  flag: boolean
steps:
  - id: route
    choose:
      condition: "$input.flag and true"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
    )?;

    ensure(
        parsed_binary(&expression)? == BinaryOp::And,
        "public AST did not retain parsed expression tree",
    )
}

#[test]
fn expression_diagnostics_preserve_compile_parse_ast_parity() -> Result<(), String> {
    ensure_parse_compile_error_parity(
        br#"version: velvet-ballastics/v1
name: ast_surface
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.flag =="
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
    )
}

#[test]
fn parse_ast_accepts_valid_rooted_refs_in_expression_strings() -> Result<(), String> {
    let expression = first_choose_condition(rooted_refs_expression_source())?;

    ensure(
        parsed_binary(&expression)? == BinaryOp::And,
        "root was not textual and",
    )
}

fn rooted_refs_expression_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: ast_surface
when:
  manual: {}
inputs:
  flag: boolean
vars:
  enabled: true
secrets:
  token: TOKEN
steps:
  - id: route
    choose:
      condition: "$input.flag and $vars.enabled and ($secrets.token == \"x\")"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#
}

#[test]
fn symbolic_expression_diagnostics_preserve_compile_parse_ast_parity() -> Result<(), String> {
    ensure_parse_compile_error_parity(symbolic_condition("$input.flag && true").as_bytes())?;
    ensure_parse_compile_error_parity(symbolic_condition("$input.flag || true").as_bytes())?;
    ensure_parse_compile_error_parity(symbolic_condition("!$input.flag").as_bytes())?;
    ensure_parse_compile_error_parity(symbolic_condition("$input.count % 2 == 0").as_bytes())
}

fn symbolic_condition(condition: &'static str) -> String {
    format!(
        concat!(
            "version: velvet-ballastics/v1\nname: ast_surface\nwhen:\n",
            "  manual: {{}}\ninputs:\n  flag: boolean\n  count: number\n",
            "steps:\n  - id: route\n    choose:\n      condition: \"{}\"\n",
            "      on_true: 1\n      on_false: 1\n  - id: done\n",
            "    finish:\n      result: 0\n"
        ),
        condition
    )
}
