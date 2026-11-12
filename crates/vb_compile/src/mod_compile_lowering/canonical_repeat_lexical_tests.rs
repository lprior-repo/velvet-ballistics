//! Lexical and counter ownership checks for nested Repeat.
use crate::{CompileError, compile_workflow};
use vb_core::workflow::{NodeContinuation, NodeOutput};
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx};
const HEADER: &str = "version: velvet-ballistics/v1\nname: repeat_edge\nwhen: {manual: {}}\n";
fn compile(text: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(text.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}
fn node(workflow: &CompiledWorkflow, id: u16) -> Result<&CompiledNode, String> {
    workflow.node(StepIdx::new(id)).ok_or_else(|| format!("missing node {id}"))
}
fn output(workflow: &CompiledWorkflow, id: u16) -> Result<SlotIdx, String> {
    node(workflow, id)?.output.slot().ok_or_else(|| format!("missing output {id}"))
}
fn rejects(text: &str, predicate: impl Fn(&CompileError) -> bool) -> Result<(), String> {
    let errors = match compile_workflow(text.as_bytes()) {
        Ok(_) => return Err("unexpected source acceptance".to_owned()),
        Err(errors) => errors,
    };
    if errors.0.iter().any(predicate) { Ok(()) } else { Err(format!("wrong diagnostic: {errors:?}")) }
}
fn repeat_source(body: &str, attempts: u16) -> String {
    format!("{HEADER}steps:\n  - id: s\n    set: {{output: seed, value: '1'}}\n  - id: r\n    repeat:\n      max_attempts: {attempts}\n      steps:\n{body}  - id: done\n    finish: {{result: '$seed'}}\n")
}
#[test]
fn nested_repeat_counters_do_not_alias_body_outputs() -> Result<(), String> {
    let body = "        - id: inner\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: leaf\n                set: {output: inside, value: '7'}\n";
    let workflow = compile(&repeat_source(body, 1))?;
    let outer = output(&workflow, 1)?;
    let inner = output(&workflow, 2)?;
    let leaf = output(&workflow, 3)?;
    assert_ne!(outer, inner);
    assert_ne!(outer, leaf);
    assert_ne!(inner, leaf);
    // Outer check output is outer counter, inner check output is inner counter
    assert_eq!(node(&workflow, 6)?.output, NodeOutput::Slot(outer));
    assert_eq!(node(&workflow, 4)?.output, NodeOutput::Slot(inner));
    Ok(())
}

// Lexical: duplicate output inside repeat body is rejected.
#[test]
fn repeat_duplicate_output_inside_body_is_rejected() -> Result<(), String> {
    let body = "        - id: a\n          set: {output: dup, value: '1'}\n        - id: b\n          set: {output: dup, value: '2'}\n";
    rejects(&repeat_source(body, 1), |e| {
        matches!(e, CompileError::DuplicateOutputName { name } if name.as_ref() == "dup")
    })
}

// Lexical: duplicate step id inside repeat body.
#[test]
fn repeat_duplicate_step_id_inside_body_is_rejected() -> Result<(), String> {
    let body = "        - id: same\n          set: {output: a, value: '1'}\n        - id: same\n          set: {output: b, value: '2'}\n";
    rejects(&repeat_source(body, 1), |e| {
        matches!(e, CompileError::DuplicateStepId { .. })
    })
}

// Lexical: forward reference inside nested repeat is rejected.
#[test]
fn nested_repeat_forward_reference_is_rejected() -> Result<(), String> {
    let body = "        - id: outer_inner\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: a\n                set: {output: x, value: '$later'}\n              - id: b\n                set: {output: later, value: '7'}\n";
    rejects(&repeat_source(body, 1), |e| {
        matches!(e, CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "later")
    })
}

// Lexical: inner alias must not escape to outer body.
#[test]
fn nested_repeat_inner_alias_does_not_escape() -> Result<(), String> {
    let body = "        - id: inner\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: leaf\n                set: {output: secret, value: '7'}\n        - id: grab\n          set: {output: grabbed, value: '$secret'}\n";
    rejects(&repeat_source(body, 1), |e| {
        matches!(e, CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "secret")
    })
}

// Lexical: unknown $step reference inside nested repeat.
#[test]
fn nested_repeat_unknown_step_reference_is_rejected() -> Result<(), String> {
    let body = "        - id: inner\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: leaf\n                set: {output: x, value: '$step.missing'}\n";
    rejects(&repeat_source(body, 1), |e| {
        matches!(
            e,
            CompileError::UnknownReferenceName { kind, name, .. }
                if *kind == "step" && name.as_ref() == "missing"
        )
    })
}

// Empty body and zero attempts are rejected with field diagnostics.
#[test]
fn repeat_zero_attempts_and_empty_body_are_rejected() -> Result<(), String> {
    let valid = "        - id: a\n          set: {output: x, value: '1'}\n";
    rejects(&repeat_source(valid, 0), |e| {
        matches!(e, CompileError::StepFieldShape { field, .. } if *field == "repeat.max_attempts")
    })?;
    let empty_source = format!(
        "{HEADER}steps:\n  - id: s\n    set: {{output: seed, value: '1'}}\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps: []\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(&empty_source, |e| {
        matches!(e, CompileError::StepFieldShape { field, .. } if *field == "steps")
    })
}

// Check routes: outer check loops to outer body, inner check loops to inner body.
#[test]
fn nested_repeat_check_routes_bind_to_owning_body() -> Result<(), String> {
    let body = "        - id: inner\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: leaf\n                set: {output: x, value: '1'}\n        - id: after\n          set: {output: z, value: '2'}\n";
    let workflow = compile(&repeat_source(body, 3))?;
    // 0 seed,1 outer-start,2 inner-start,3 leaf,4 inner-check,5 inner-finish,6 after,7 outer-check,8 outer-finish,9 done
    assert_eq!(workflow.to_parts().nodes.len(), 10);
    assert_eq!(
        node(&workflow, 4)?.routing.next,
        NodeContinuation::Step(StepIdx::new(3))
    );
    assert_eq!(
        node(&workflow, 7)?.routing.next,
        NodeContinuation::Step(StepIdx::new(2))
    );
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: output(&workflow, 1)?,
            done: StepIdx::new(8),
        }
    );
    Ok(())
}

// Time/attempt caps: max_attempts=65535 is accepted; overflow beyond is not
// attempted here. Verifies the cap is preserved through lowering.
#[test]
fn repeat_max_attempts_cap_is_preserved() -> Result<(), String> {
    let body = "        - id: a\n          set: {output: x, value: '1'}\n";
    let workflow = compile(&repeat_source(body, 65535))?;
    assert_eq!(
        node(&workflow, 1)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 65535.into(),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        }
    );
    Ok(())
}
