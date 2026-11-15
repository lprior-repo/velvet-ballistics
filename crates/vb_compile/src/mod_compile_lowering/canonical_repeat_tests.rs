//! Compile-only regressions; runtime re-entry is qualified by source-drive tests.

use crate::{CompileError, compile_workflow};
use vb_core::workflow::{NodeContinuation, NodeOutput};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, ExprOp, SlotIdx, StepIdx,
};

const HEADER: &str = "version: velvet-ballistics/v1\nname: repeat_lexical\nwhen: {manual: {}}\n";
const TWO_SETS: &str = "        - id: body_first\n          set: {output: inside, value: '7'}\n        - id: body_second\n          set: {output: final_value, value: '$inside + 4'}\n";

fn source(declarations: &str, body: &str, finish: &str, attempts: u16) -> String {
    format!(
        "{HEADER}{declarations}steps:\n  - id: before\n    set: {{output: answer, value: '42'}}\n  - id: bounded\n    repeat:\n      max_attempts: {attempts}\n      steps:\n{body}  - id: done\n    finish: {{result: '{finish}'}}\n"
    )
}

fn compile(text: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(text.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

fn node(workflow: &CompiledWorkflow, id: u16) -> Result<&CompiledNode, String> {
    workflow
        .node(StepIdx::new(id))
        .ok_or_else(|| format!("missing node {id}"))
}

fn output(workflow: &CompiledWorkflow, id: u16) -> Result<SlotIdx, String> {
    node(workflow, id)?
        .output
        .slot()
        .ok_or_else(|| format!("missing output {id}"))
}

fn rejects(text: &str, predicate: impl Fn(&CompileError) -> bool) -> Result<(), String> {
    let errors = match compile_workflow(text.as_bytes()) {
        Ok(_) => return Err("unexpected source acceptance".to_owned()),
        Err(errors) => errors,
    };
    if errors.0.iter().any(predicate) {
        Ok(())
    } else {
        Err(format!("wrong diagnostic: {errors:?}"))
    }
}

#[test]
fn flat_repeat_layout_uses_actual_width_and_private_outputs() -> Result<(), String> {
    for attempts in [1, 3] {
        let workflow = compile(&source("", TWO_SETS, "answer", attempts))?;
        let parts = workflow.to_parts();
        assert_eq!(parts.nodes.len(), 8);
        for (index, node) in parts.nodes.iter().enumerate() {
            assert_eq!(node.id.as_usize(), index);
        }
        let counter = output(&workflow, 1)?;
        let first = output(&workflow, 2)?;
        let last = output(&workflow, 3)?;
        let parent = output(&workflow, 0)?;
        let result = output(&workflow, 4)?;
        assert_eq!(
            std::collections::HashSet::from([parent, counter, first, last, result]).len(),
            5
        );
        assert_eq!(
            node(&workflow, 1)?.kind,
            CompiledNodeKind::RepeatStart {
                max_attempts: attempts.into(),
                body: StepIdx::new(2),
                done: StepIdx::new(6),
            }
        );
        assert_eq!(
            node(&workflow, 2)?.routing.next,
            NodeContinuation::Step(StepIdx::new(3))
        );
        assert_eq!(
            node(&workflow, 3)?.routing.next,
            NodeContinuation::Step(StepIdx::new(4))
        );
        assert_eq!(
            node(&workflow, 4)?.kind,
            CompiledNodeKind::Copy { source: last }
        );
        assert_eq!(
            node(&workflow, 4)?.routing.next,
            NodeContinuation::Step(StepIdx::new(5))
        );
        assert_eq!(node(&workflow, 5)?.output, NodeOutput::Slot(counter));
        assert_eq!(
            node(&workflow, 5)?.routing.next,
            NodeContinuation::Step(StepIdx::new(2))
        );
        assert_eq!(
            node(&workflow, 5)?.kind,
            CompiledNodeKind::RepeatCheck {
                attempt_slot: counter,
                done: StepIdx::new(6),
            }
        );
        assert_eq!(
            node(&workflow, 6)?.kind,
            CompiledNodeKind::RepeatFinish { result }
        );
        assert_eq!(node(&workflow, 6)?.output, NodeOutput::Slot(result));
        assert_eq!(
            node(&workflow, 6)?.routing.next,
            NodeContinuation::Step(StepIdx::new(7))
        );
        assert_eq!(
            node(&workflow, 7)?.kind,
            CompiledNodeKind::Finish { result: parent }
        );
    }
    Ok(())
}

#[test]
fn repeat_expressions_bind_input_vars_and_prior_local_step() -> Result<(), String> {
    let body = "        - id: body_first\n          set: {output: inside, value: '$input.amount + $vars.increment'}\n        - id: body_second\n          set: {output: final_value, value: '$step.body_first + 1'}\n";
    let workflow = compile(&source(
        "inputs: {amount: number}\nvars: {increment: 2}\n",
        body,
        "answer",
        3,
    ))?;
    assert_eq!(workflow.to_parts().nodes.len(), 9);
    assert_eq!(workflow.input_slots().len(), 1);
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing declared input")?
        .slot;
    let variable = output(&workflow, 0)?;
    let CompiledNodeKind::SetConst { value } = &node(&workflow, 0)?.kind else {
        return Err("missing variable prelude".to_owned());
    };
    assert_eq!(workflow.constant(*value), Some(&ConstValue::I64(2)));
    assert_eq!(
        expression_ops(&workflow, 3)?,
        [
            ExprOp::LoadSlot(input),
            ExprOp::LoadSlot(variable),
            ExprOp::Add
        ]
    );
    assert_add_one(&workflow, 4, output(&workflow, 3)?)
}

fn expression_ops(workflow: &CompiledWorkflow, id: u16) -> Result<&[ExprOp], String> {
    let CompiledNodeKind::EvalExpr { expr } = &node(workflow, id)?.kind else {
        return Err(format!("node {id} is not the expected expression"));
    };
    let program = workflow
        .expression(*expr)
        .ok_or("missing expression program")?;
    Ok(&program.ops)
}

fn assert_add_one(workflow: &CompiledWorkflow, id: u16, slot: SlotIdx) -> Result<(), String> {
    let [
        ExprOp::LoadSlot(actual),
        ExprOp::LoadConst(constant),
        ExprOp::Add,
    ] = expression_ops(workflow, id)?
    else {
        return Err(format!("node {id} is not exactly a slot plus one"));
    };
    assert_eq!(*actual, slot);
    assert_eq!(workflow.constant(*constant), Some(&ConstValue::I64(1)));
    Ok(())
}

fn assert_attempt_number_expression(
    workflow: &CompiledWorkflow,
    id: u16,
    attempt_slot: SlotIdx,
    max_attempts: u16,
) -> Result<(), String> {
    let [
        ExprOp::LoadSlot(actual),
        ExprOp::LoadConst(packed_limit),
        ExprOp::Sub,
        ExprOp::LoadConst(one),
        ExprOp::Add,
    ] = expression_ops(workflow, id)?
    else {
        return Err(format!("node {id} is not a packed attempt decode"));
    };
    assert_eq!(*actual, attempt_slot);
    let expected_limit = i64::from(max_attempts)
        .checked_shl(32)
        .ok_or_else(|| "attempt maximum shift overflow".to_owned())?;
    assert_eq!(
        workflow.constant(*packed_limit),
        Some(&ConstValue::I64(expected_limit))
    );
    assert_eq!(workflow.constant(*one), Some(&ConstValue::I64(1)));
    Ok(())
}

#[test]
fn repeat_do_reads_prior_local_and_feeds_following_expression() -> Result<(), String> {
    let body = "        - id: body_first\n          set: {output: inside, value: '7'}\n        - id: invoke\n          do: {action: '7', input: '$step.body_first'}\n        - id: body_second\n          set: {output: final_value, value: '$step.invoke + 1'}\n";
    let workflow = compile(&source("", body, "answer", 3))?;
    assert_eq!(workflow.to_parts().nodes.len(), 9);
    let first = output(&workflow, 2)?;
    let action_output = output(&workflow, 3)?;
    assert_eq!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::Do {
            action: vb_core::ActionId::new(7),
            input: first,
        }
    );
    assert_eq!(
        node(&workflow, 3)?.routing.next,
        NodeContinuation::Step(StepIdx::new(4))
    );
    assert_ne!(action_output, first);
    assert_ne!(action_output, output(&workflow, 1)?);
    assert_add_one(&workflow, 4, action_output)
}

#[test]
fn repeat_forward_body_reference_is_rejected() -> Result<(), String> {
    let body = "        - id: body_first\n          set: {output: inside, value: '$later + 1'}\n        - id: body_second\n          set: {output: later, value: '7'}\n";
    rejects(&source("", body, "answer", 3), |e| {
        matches!(e,
        CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "later")
    })
}

#[test]
fn repeat_body_alias_does_not_escape() -> Result<(), String> {
    rejects(&source("", TWO_SETS, "$inside", 3), |e| {
        matches!(e,
        CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "inside")
    })
}

#[test]
fn repeat_known_boolean_arithmetic_is_rejected() -> Result<(), String> {
    let body = "        - id: body_first\n          set: {output: inside, value: 'true'}\n        - id: body_second\n          set: {output: final_value, value: '$inside + 1'}\n";
    rejects(&source("", body, "answer", 3), |e| {
        matches!(
            e,
            CompileError::TypeMismatch {
                expected: "number",
                found: "boolean",
                ..
            }
        )
    })
}

#[test]
fn repeat_body_guard_lowers_to_boolean_branch() -> Result<(), String> {
    let body = "        - id: body_first\n          if: 'false'\n          set: {output: inside, value: '7'}\n        - id: body_final\n          set: {output: final_value, value: '9'}\n";
    let workflow = compile(&source("", body, "answer", 3))?;
    let CompiledNodeKind::Choose {
        branches,
        otherwise,
    } = &node(&workflow, 2)?.kind
    else {
        return Err("guard did not lower to Choose".to_owned());
    };
    assert_eq!(branches.len(), 1);
    assert_eq!(
        branches.first().ok_or("guard branch")?.target,
        StepIdx::new(3)
    );
    assert_eq!(otherwise.step(), Some(StepIdx::new(4)));
    assert!(matches!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::SetConst { .. }
    ));
    assert_eq!(
        node(&workflow, 3)?.routing.next,
        NodeContinuation::Step(StepIdx::new(4))
    );
    let final_value = output(&workflow, 4)?;
    assert!(
        matches!(node(&workflow, 4)?.kind, CompiledNodeKind::SetConst { value } if workflow.constant(value) == Some(&ConstValue::I64(9)))
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::Copy {
            source: final_value
        }
    );
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: output(&workflow, 5)?
        }
    );
    Ok(())
}

#[test]
fn nested_repeat_has_separate_validated_regions() -> Result<(), String> {
    let body = "        - id: inner\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner_value\n                set: {output: inside, value: '7'}\n";
    let workflow = compile(&source("", body, "answer", 3))?;
    let parts = workflow.to_parts();
    let controls: Vec<_> = parts
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, CompiledNodeKind::RepeatCheck { .. }))
        .collect();
    assert_eq!(controls.len(), 2);
    for control in controls {
        let region = workflow
            .loop_region(control.id)
            .ok_or("missing loop region")?;
        assert_eq!(region.control(), control.id);
    }
    Ok(())
}

#[test]
fn repeat_attempt_number_is_bound_as_an_integer() -> Result<(), String> {
    let body = "        - id: attempt_value\n          set: {output: observed, value: '$attempt.number'}\n";
    let workflow = compile(&source("", body, "answer", 3))?;
    assert_attempt_number_expression(&workflow, 2, output(&workflow, 1)?, 3)?;
    Ok(())
}

#[test]
fn repeat_attempt_number_keeps_integer_type_in_boolean_position() -> Result<(), String> {
    let body = "        - id: guarded\n          if: '$attempt.number'\n          set: {output: observed, value: '1'}\n";
    rejects(&source("", body, "answer", 3), |error| {
        matches!(
            error,
            CompileError::TypeMismatch {
                field: "if",
                expected: "boolean",
                found: "non-boolean"
            }
        )
    })
}

#[test]
fn repeat_attempt_number_is_available_to_direct_inputs() -> Result<(), String> {
    let body = "        - id: invoke\n          do: {action: '7', input: '$attempt.number'}\n        - id: pause\n          wait: {timeout: '$attempt.number'}\n";
    let workflow = compile(&source("", body, "answer", 3))?;
    let parts = workflow.to_parts();
    let attempt_slot = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::EvalExpr { .. } => node.output.slot(),
            _ => None,
        })
        .ok_or("missing attempt-number materialization")?;
    assert!(parts.nodes.iter().any(|node| matches!(
        node.kind,
        CompiledNodeKind::Do { input, .. } if input == attempt_slot
    )));
    assert!(parts.nodes.iter().any(|node| matches!(
        node.kind,
        CompiledNodeKind::WaitUntil { deadline_slot } if deadline_slot == attempt_slot
    )));
    Ok(())
}

#[test]
fn structured_action_value_attempt_reference_is_scoped_at_depth_nine() -> Result<(), String> {
    let nested_value = |leaf: &str| {
        let fields = (0..9).rev().fold(format!("leaf: {leaf}"), |child, index| {
            format!("level{index}: {{{child}}}")
        });
        format!("{{{fields}}}")
    };
    let source_with = |leaf: &str| {
        let body = format!(
            "        - id: invoke\n          do:\n            action: '7'\n            with: {}\n",
            nested_value(leaf)
        );
        source("", &body, "answer", 3)
    };
    let literal = compile(&source_with("1"))?;
    assert!(
        !literal
            .to_parts()
            .nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::EvalExpr { .. }))
    );

    let workflow = compile(&source_with("'$attempt.number'"))?;
    let parts = workflow.to_parts();
    let repeat_body = match node(&workflow, 1)?.kind {
        CompiledNodeKind::RepeatStart { body, .. } => body,
        _ => return Err("missing RepeatStart".to_owned()),
    };
    let attempt_number = workflow
        .node(repeat_body)
        .ok_or("missing attempt-number entry")?
        .output
        .slot()
        .ok_or("missing attempt-number slot")?;
    assert_eq!(
        parts
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, CompiledNodeKind::EvalExpr { .. }))
            .count(),
        2
    );
    assert!(parts.nodes.iter().any(|node| {
        matches!(node.kind, CompiledNodeKind::EvalExpr { expr }
            if workflow.expression(expr).is_some_and(|program|
                program.ops.as_ref() == [ExprOp::LoadSlot(attempt_number)]))
    }));
    Ok(())
}

#[test]
fn repeat_attempt_references_reject_bare_and_accessor_tails() -> Result<(), String> {
    for reference in ["$attempt", "$attempt.bad", "$attempt.number.bad"] {
        let body = format!(
            "        - id: attempt_value\n          set: {{output: observed, value: '{reference}'}}\n"
        );
        rejects(&source("", &body, "observed", 3), |error| {
            matches!(error, CompileError::UnknownReferenceName { .. })
        })?;
    }
    Ok(())
}

#[test]
fn repeat_attempt_number_does_not_escape_its_body() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: before\n    set: {{output: answer, value: '42'}}\n  - id: bounded\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: inside\n          set: {{output: inside_value, value: '1'}}\n  - id: escaped\n    set: {{output: escaped_value, value: '$attempt.number'}}\n  - id: done\n    finish: {{result: '$answer'}}\n"
    );
    rejects(&text, |error| {
        matches!(error, CompileError::UnknownReferenceName { .. })
    })
}

#[test]
fn attempt_root_cannot_be_shadowed_by_output_alias() -> Result<(), String> {
    let body = "        - id: shadow\n          set: {output: attempt.number, value: '1'}\n";
    rejects(&source("", body, "answer", 2), |error| {
        matches!(
            error,
            CompileError::InvalidName {
                field: "set.output",
                ..
            }
        )
    })
}

#[path = "canonical_repeat_tests/nested_layout.rs"]
mod nested_layout;
