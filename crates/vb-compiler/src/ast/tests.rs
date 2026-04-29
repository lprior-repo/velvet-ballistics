use super::*;
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
        .map_err(|error| format!("parse_ast failed: {error:?}"))
}

fn parse_err(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(error) => Ok(error),
    }
}

#[test]
fn parse_ast_retains_vars_and_examples_surface() -> Result<(), String> {
    let ast = parse(
        b"version: velvet/v1\nname: ast_surface\nwhen:\n  manual: {}\nvars:\n  retries: 3\nexamples:\n  - name: happy\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
    )?;

    ensure(ast.vars.len() == 1, "vars surface not retained")?;
    ensure(ast.examples.len() == 1, "examples surface not retained")?;
    ensure(ast.steps.len() == 1, "step surface not retained")?;
    Ok(())
}

#[test]
fn parse_ast_rejects_ipc_like_compile_boundary() -> Result<(), String> {
    let error = parse_err(
        b"version: velvet/v1\nname: ast_surface\nwhen:\n  ipc: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
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
        b"version: velvet/v1\nname: ast_surface\nwhen:\n  manual:\n    extra: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
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
        b"version: velvet/v1\nname: ast_surface\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    mystery: true\n    finish:\n      result: 0\n",
    )?;

    ensure(
        matches!(error, CompileError::UnknownStepField { .. }),
        "unknown step field did not fail with UnknownStepField",
    )?;
    Ok(())
}

#[test]
fn parse_ast_keeps_available_source_marks() -> Result<(), String> {
    let source = "version: velvet/v1\nname: ast_surface\nwhen:\n  manual: {}\nvars:\n  retries: 3\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
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
    ensure_mark(step_mark, source, "id: done", 8, 4)?;
    Ok(())
}
