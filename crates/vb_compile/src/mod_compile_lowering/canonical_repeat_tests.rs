//! Compile-only regressions; runtime re-entry is qualified by source-drive tests.

use crate::{CompileError, compile_workflow};
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
        assert_eq!(parts.nodes.len(), 7);
        for (index, node) in parts.nodes.iter().enumerate() {
            assert_eq!(node.id.as_usize(), index);
        }
        let counter = output(&workflow, 1)?;
        let first = output(&workflow, 2)?;
        let last = output(&workflow, 3)?;
        let parent = output(&workflow, 0)?;
        assert_eq!(
            std::collections::HashSet::from([parent, counter, first, last]).len(),
            4
        );
        assert_eq!(
            node(&workflow, 1)?.kind,
            CompiledNodeKind::RepeatStart {
                max_attempts: attempts,
                body: StepIdx::new(2),
                done: StepIdx::new(5),
            }
        );
        assert_eq!(node(&workflow, 2)?.next, Some(StepIdx::new(3)));
        assert_eq!(node(&workflow, 3)?.next, Some(StepIdx::new(4)));
        assert_eq!(node(&workflow, 4)?.output, Some(counter));
        assert_eq!(node(&workflow, 4)?.next, Some(StepIdx::new(2)));
        assert_eq!(
            node(&workflow, 4)?.kind,
            CompiledNodeKind::RepeatCheck {
                attempt_slot: counter,
                done: StepIdx::new(5),
            }
        );
        assert_eq!(
            node(&workflow, 5)?.kind,
            CompiledNodeKind::RepeatFinish { result: last }
        );
        assert_eq!(node(&workflow, 5)?.output, Some(last));
        assert_eq!(node(&workflow, 5)?.next, Some(StepIdx::new(6)));
        assert_eq!(
            node(&workflow, 6)?.kind,
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
    assert_eq!(workflow.to_parts().nodes.len(), 8);
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

#[test]
fn repeat_do_reads_prior_local_and_feeds_following_expression() -> Result<(), String> {
    let body = "        - id: body_first\n          set: {output: inside, value: '7'}\n        - id: invoke\n          do: {action: '7', input: '$step.body_first'}\n        - id: body_second\n          set: {output: final_value, value: '$step.invoke + 1'}\n";
    let workflow = compile(&source("", body, "answer", 3))?;
    assert_eq!(workflow.to_parts().nodes.len(), 8);
    let first = output(&workflow, 2)?;
    let action_output = output(&workflow, 3)?;
    assert_eq!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::Do {
            action: vb_core::ActionId::new(7),
            input: first,
        }
    );
    assert_eq!(node(&workflow, 3)?.next, Some(StepIdx::new(4)));
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
fn repeat_body_controls_remain_unsupported() -> Result<(), String> {
    let body = "        - id: body_first\n          if: 'false'\n          set: {output: inside, value: '7'}\n";
    rejects(&source("", body, "answer", 3), |e| {
        matches!(e,
        CompileError::UnsupportedStepControlField { field, .. } if field.as_ref() == "if")
    })
}

#[test]
fn nested_repeat_activation_remains_unsupported() -> Result<(), String> {
    let body = "        - id: inner\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner_value\n                set: {output: inside, value: '7'}\n";
    rejects(&source("", body, "answer", 3), |e| {
        matches!(
            e,
            CompileError::UnsupportedStepPrimitive {
                primitive: "repeat",
                ..
            }
        )
    })
}
