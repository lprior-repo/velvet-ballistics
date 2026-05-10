use crate::{CompileError, YamlCompiler};

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

fn compile_error_text(source: &[u8]) -> String {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => format!("compile unexpectedly succeeded: {workflow:?}"),
        Err(errors) => errors.to_string(),
    }
}

fn parse_error_text(source: &[u8]) -> String {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => format!("parse_ast unexpectedly succeeded: {ast:?}"),
        Err(errors) => errors.to_string(),
    }
}

fn compile_parse_errors(source: &[u8]) -> Result<(CompileError, CompileError), String> {
    Ok((compile_error(source)?, parse_error(source)?))
}

fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn ensure_error_text_parity(source: &[u8]) -> Result<(), String> {
    ensure(
        compile_error_text(source) == parse_error_text(source),
        "compile and parse_ast diagnostics diverged",
    )
}

fn ensure_unknown_target(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::UnknownStepTarget { step: 1, target: 3 }
        ),
        "unknown target did not use public typed diagnostic",
    )
}

fn ensure_duplicate_step_id(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(error, CompileError::DuplicateStepId { .. }),
        "duplicate step ID did not preempt control-flow validation",
    )
}

fn ensure_unsupported_then(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(error, CompileError::UnsupportedStepControlField { .. }),
        "unsupported then did not preempt control-flow validation",
    )
}

fn ensure_non_last_finish(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::StepFieldShape {
                field: "finish",
                ..
            }
        ),
        "non-last finish did not use existing lowering diagnostic",
    )
}

fn ensure_last_non_finish(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(error, CompileError::LastStepMustFinish),
        "last non-finish did not use existing lowering diagnostic",
    )
}

fn ensure_backward_target(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::BackwardBranchTarget { step: 1, target: 0 }
        ),
        "backward target did not use typed diagnostic",
    )
}

fn ensure_self_target(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::BackwardBranchTarget { step: 1, target: 1 }
        ),
        "self cycle did not use typed diagnostic",
    )
}

fn ensure_unreachable(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(error, CompileError::UnreachableStep { step: 2 }),
        "unreachable step did not use typed diagnostic",
    )
}

fn ensure_unknown_input_reference(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::UnknownReferenceName { kind: "input", .. }
        ),
        "reference error did not preempt control-flow validation",
    )
}

fn ensure_input_schema_shape(error: CompileError) -> Result<(), String> {
    ensure(
        matches!(
            error,
            CompileError::FieldShape {
                field: "inputs",
                ..
            }
        ),
        "input schema error did not preempt control-flow validation",
    )
}

fn ensure_pair(source: &[u8], check: fn(CompileError) -> Result<(), String>) -> Result<(), String> {
    ensure_error_text_parity(source)?;
    let (compile, parse) = compile_parse_errors(source)?;
    check(compile)?;
    check(parse)
}

#[test]
fn compile_and_parse_ast_reject_unknown_numeric_choose_target() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_unknown_target)
}

#[test]
fn duplicate_step_id_preempts_control_flow_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: duplicate
    choose:
      condition: 0
      on_true: 3
      on_false: 3
  - id: duplicate
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_duplicate_step_id)
}

#[test]
fn unsupported_then_preempts_control_flow_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: route
    then: done
    choose:
      condition: 0
      on_true: 3
      on_false: 3
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_unsupported_then)
}

#[test]
fn non_last_finish_uses_lowering_diagnostic_before_cfg() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: early
    finish:
      result: 0
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_non_last_finish)
}

#[test]
fn last_non_finish_uses_lowering_diagnostic_before_cfg() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: build_result
    save:
      value: 1
"#;

    ensure_pair(source, ensure_last_non_finish)
}

#[test]
fn parse_ast_rejects_unreachable_steps_after_reference_validation() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 3
  - id: skipped
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;
    ensure_unreachable(error)
}

#[test]
fn compile_rejects_unreachable_steps_after_reference_validation() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 3
  - id: skipped
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    let error = compile_error(source)?;
    ensure_unreachable(error)
}

#[test]
fn compile_and_parse_ast_preserve_first_diagnostic_parity() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 3
  - id: skipped
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    ensure_error_text_parity(source)
}

#[test]
fn reference_errors_still_preempt_control_flow_errors() -> Result<(), String> {
    ensure_pair(
        reference_preemption_source(),
        ensure_unknown_input_reference,
    )
}

fn reference_preemption_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
inputs:
  user: text
examples:
  - name: bad_ref
    value: $input.missing
steps:
  - id: route
    choose:
      condition: 0
      on_true: 2
      on_false: 2
  - id: skipped
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#
}

#[test]
fn input_schema_errors_still_preempt_control_flow_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
inputs: true
steps:
  - id: route
    choose:
      condition: 0
      on_true: 2
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_input_schema_shape)
}

#[test]
fn parse_ast_rejects_backward_step_targets() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: first
    save:
      value: 1
  - id: route
    choose:
      condition: true
      on_true: 0
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;
    ensure_backward_target(error)
}

#[test]
fn compile_rejects_backward_step_targets() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: first
    save:
      value: 1
  - id: route
    choose:
      condition: true
      on_true: 0
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_backward_target)
}

#[test]
fn compile_rejects_self_cycles() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: first
    save:
      value: 1
  - id: route
    choose:
      condition: true
      on_true: 1
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    let error = compile_error(source)?;
    ensure_self_target(error)
}

#[test]
fn parse_ast_rejects_self_cycles() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: control_flow_case
when:
  manual: {}
steps:
  - id: first
    save:
      value: 1
  - id: route
    choose:
      condition: true
      on_true: 1
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_self_target)
}
