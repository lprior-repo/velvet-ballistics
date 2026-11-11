//! Compile-only reduce coverage: multi-step layout, typed initial parsing,
//! accumulator type preservation, and loop-body alias rejection.
//!
//! The slot dependency gate (13) admits only writes that reach the
//! accumulator without a feedback loop through another slot. The bodies
//! below therefore feed the final accumulator write from constants, outer
//! bindings, and nested results, never from steps that themselves read the
//! accumulator slot.

use super::{checked_width_n, parse_typed_initial};
use crate::{CompileError, CompileErrors, compile_workflow};
use vb_core::workflow::{NodeContinuation, NodeOutput};
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

const HEADER: &str = "version: velvet-ballistics/v1\nname: reduce_multi\nwhen: {manual: {}}\n";
const SINGLE_STEP: &str =
    "        - id: update\n          set: {output: local_value, value: '$total + 1'}\n";
const IDENTITY: &str =
    "        - id: update\n          set: {output: local_value, value: '$total'}\n";
const TWO_STEPS: &str = "        - id: tmp_step\n          set: {output: tmp_value, value: '2'}\n        - id: update\n          set: {output: local_value, value: '$total + $tmp_value'}\n";

fn source(initial: &str, body: &str) -> String {
    format!(
        "{HEADER}steps:\n  - id: folded\n    reduce:\n      variable: total\n      input: '8'\n      initial: '{initial}'\n      steps:\n{body}  - id: done\n    finish: {{result: '$steps.folded'}}\n"
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
fn multi_step_reduce_layout_writes_final_step_into_accumulator() -> Result<(), String> {
    let workflow = compile(&source("10", TWO_STEPS))?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 7);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let accumulator = output(&workflow, 5)?;
    let CompiledNodeKind::ReduceStart {
        input,
        initial,
        body,
        done,
        ..
    } = node(&workflow, 1)?.kind
    else {
        return Err("missing ReduceStart".to_owned());
    };
    assert_eq!(
        node(&workflow, 0)?.kind,
        CompiledNodeKind::Copy {
            source: SlotIdx::new(8)
        }
    );
    assert_eq!(input, output(&workflow, 0)?);
    assert_eq!(workflow.constant(initial), Some(&ConstValue::I64(10)));
    assert_eq!(body, StepIdx::new(2));
    assert_eq!(done, StepIdx::new(5));
    assert_eq!(output(&workflow, 3)?, accumulator);
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::ReduceNext {
            iterator_slot: output(&workflow, 1)?,
            accumulator,
            body: StepIdx::new(2),
            done: StepIdx::new(5),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::ReduceFinish { accumulator }
    );
    assert_eq!(
        node(&workflow, 3)?.routing.next,
        NodeContinuation::Step(StepIdx::new(4))
    );
    Ok(())
}

#[test]
fn single_step_reduce_has_no_update_copy() -> Result<(), String> {
    let workflow = compile(&source("10", SINGLE_STEP))?;
    assert_eq!(workflow.to_parts().nodes.len(), 6);
    assert_eq!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::ReduceNext {
            iterator_slot: output(&workflow, 1)?,
            accumulator: output(&workflow, 4)?,
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        }
    );
    Ok(())
}

#[test]
fn parse_typed_initial_variants() -> Result<(), String> {
    let cases = [
        ("10", ConstValue::I64(10), ExprType::I64),
        ("-5", ConstValue::I64(-5), ExprType::I64),
        ("+5", ConstValue::I64(5), ExprType::I64),
        ("true", ConstValue::Bool(true), ExprType::Bool),
        ("false", ConstValue::Bool(false), ExprType::Bool),
        ("null", ConstValue::Null, ExprType::Null),
        (
            "0.5",
            ConstValue::F64(vb_core::FiniteF64::new(0.5).map_err(|e| e.to_string())?),
            ExprType::F64,
        ),
        (
            "-0.5",
            ConstValue::F64(vb_core::FiniteF64::new(-0.5).map_err(|e| e.to_string())?),
            ExprType::F64,
        ),
    ];
    for (text, constant, ty) in cases {
        let parsed = parse_typed_initial(text, 0).map_err(|e| format!("{text}: {e:?}"))?;
        assert_eq!(parsed, (constant, ty), "initial {text}");
    }
    for text in [
        "$increment",
        "\"text\"",
        "1 + 2",
        "sum($total)",
        "9223372036854775808",
    ] {
        let rejected = parse_typed_initial(text, 0);
        let ok = matches!(
            &rejected,
            Err(errors) if errors.0.iter().any(|e| matches!(
                e,
                CompileError::StepFieldShape { field: "aggregate.initial", .. }
            ))
        );
        assert!(ok, "initial {text} must fail: {rejected:?}");
    }
    Ok(())
}

#[test]
fn parse_typed_initial_whitespace_padded_negatives() -> Result<(), String> {
    let cases = [
        (" -5", ConstValue::I64(-5), ExprType::I64),
        ("-5 ", ConstValue::I64(-5), ExprType::I64),
        (" -5 ", ConstValue::I64(-5), ExprType::I64),
        ("\t-5", ConstValue::I64(-5), ExprType::I64),
        ("\t-5\t", ConstValue::I64(-5), ExprType::I64),
        ("  -5  ", ConstValue::I64(-5), ExprType::I64),
        (
            " -0.5",
            ConstValue::F64(vb_core::FiniteF64::new(-0.5).map_err(|e| e.to_string())?),
            ExprType::F64,
        ),
        (
            " -0.5 ",
            ConstValue::F64(vb_core::FiniteF64::new(-0.5).map_err(|e| e.to_string())?),
            ExprType::F64,
        ),
    ];
    for (text, constant, ty) in cases {
        let parsed = parse_typed_initial(text, 0).map_err(|e| format!("{text:?}: {e:?}"))?;
        assert_eq!(parsed, (constant, ty), "initial {text:?}");
    }
    Ok(())
}

#[test]
fn padded_neg_i64_initial_pins_reduce_start_constant() -> Result<(), String> {
    for initial_text in [" -5", " -5 "] {
        let workflow = compile(&source(initial_text, IDENTITY))?;
        let CompiledNodeKind::ReduceStart { initial, .. } = node(&workflow, 1)?.kind else {
            return Err("missing ReduceStart".to_owned());
        };
        assert_eq!(
            workflow.constant(initial),
            Some(&ConstValue::I64(-5)),
            "initial {initial_text:?}"
        );
    }
    Ok(())
}

#[test]
fn typed_constants_reach_reduce_start() -> Result<(), String> {
    let flags = [("true", ConstValue::Bool(true)), ("null", ConstValue::Null)];
    for (text, constant) in flags {
        let workflow = compile(&source(text, IDENTITY))?;
        let CompiledNodeKind::ReduceStart { initial, .. } = node(&workflow, 1)?.kind else {
            return Err("missing ReduceStart".to_owned());
        };
        assert_eq!(workflow.constant(initial), Some(&constant));
    }
    let workflow = compile(&source("0.5", IDENTITY))?;
    let CompiledNodeKind::ReduceStart { initial, .. } = node(&workflow, 1)?.kind else {
        return Err("missing ReduceStart".to_owned());
    };
    assert_eq!(
        workflow.constant(initial),
        Some(&ConstValue::F64(
            vb_core::FiniteF64::new(0.5).map_err(|e| e.to_string())?
        ))
    );
    Ok(())
}

#[test]
fn type_mismatched_multi_body_rejected() -> Result<(), String> {
    rejects(
        &source(
            "10",
            "        - id: tmp_step\n          set: {output: tmp_value, value: '2'}\n        - id: update\n          set: {output: local_value, value: '$total == 10'}\n",
        ),
        |e| {
            matches!(
                e,
                CompileError::StepFieldShape {
                    field: "reduce.steps",
                    ..
                }
            )
        },
    )
}

#[test]
fn empty_body_rejected() -> Result<(), String> {
    rejects(
        &format!(
            "{HEADER}steps:\n  - id: folded\n    reduce:\n      variable: total\n      input: '8'\n      initial: '10'\n      steps: []\n  - id: done\n    finish: {{result: '$steps.folded'}}\n"
        ),
        |e| matches!(e, CompileError::StepFieldShape { field: "steps", .. }),
    )
}

#[test]
fn body_output_alias_rejected() -> Result<(), String> {
    rejects(
        &source(
            "10",
            "        - id: tmp_step\n          set: {output: tmp_value, value: '2'}\n        - id: update\n          set: {output: total, value: '$tmp_value'}\n",
        ),
        |e| matches!(e, CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"),
    )
}

#[test]
fn step_id_alias_rejected() -> Result<(), String> {
    rejects(
        &source(
            "10",
            "        - id: total\n          set: {output: tmp_value, value: '2'}\n        - id: update\n          set: {output: local_value, value: '$total'}\n",
        ),
        |e| matches!(e, CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"),
    )
}

#[test]
fn nested_container_in_multi_body_compiles() -> Result<(), String> {
    let body = "        - id: loop\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner\n                set: {output: tmp_value, value: '6'}\n        - id: update\n          set: {output: local_value, value: '$total + $steps.loop'}\n";
    let workflow = compile(&source("10", body))?;
    assert_eq!(workflow.to_parts().nodes.len(), 10);
    assert_eq!(
        node(&workflow, 2)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 1.into(),
            body: StepIdx::new(3),
            done: StepIdx::new(5),
        }
    );
    assert_eq!(output(&workflow, 6)?, output(&workflow, 8)?);
    Ok(())
}

#[test]
fn deep_set_output_alias_rejected() -> Result<(), String> {
    rejects(
        &source("10", "        - id: loop\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner\n                set: {output: total, value: '6'}\n        - id: update\n          set: {output: local_value, value: '$total + $steps.loop'}\n".into()),
        |e| matches!(e, CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"),
    )
}

#[test]
fn deep_step_id_alias_rejected() -> Result<(), String> {
    rejects(
        &source(
            "10",
            "        - id: loop\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: total\n                set: {output: tmp_value, value: '6'}\n        - id: update\n          set: {output: local_value, value: '$total + $steps.loop'}\n",
        ),
        |e| matches!(e, CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"),
    )
}

#[test]
fn width_overflow_pins_exact_step_index() {
    assert_eq!(checked_width_n(65535), Ok(65535));
    assert_eq!(
        checked_width_n(65536),
        Err(CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: 65536
        }]))
    );
    assert_eq!(
        checked_width_n(usize::MAX),
        Err(CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: usize::MAX
        }]))
    );
}

#[test]
fn do_in_multi_body_layout() -> Result<(), String> {
    let body = "        - id: tmp_step\n          set: {output: tmp_value, value: '2'}\n        - id: update\n          do:\n            action: '0'\n            input: '1'\n";
    let workflow = compile(&source("10", body))?;
    assert_eq!(workflow.to_parts().nodes.len(), 7);
    assert_eq!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::Do {
            action: vb_core::ActionId::new(0),
            input: SlotIdx::new(1),
        }
    );
    assert_eq!(
        node(&workflow, 3)?.routing.next,
        NodeContinuation::Step(StepIdx::new(4))
    );
    assert_eq!(output(&workflow, 3)?, output(&workflow, 5)?);
    Ok(())
}

#[test]
fn do_last_reduce_result_is_unknown_for_bool_operation() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: folded\n    reduce:\n      variable: total\n      input: '8'\n      initial: '10'\n      steps:\n        - id: tmp_step\n          set: {{output: tmp_value, value: '2'}}\n        - id: update\n          do:\n            action: '0'\n            input: '1'\n  - id: probed\n    set: {{output: probed_value, value: 'not $steps.folded'}}\n  - id: done\n    finish: {{result: '$steps.probed'}}\n"
    );
    let workflow = compile(&text)?;
    let CompiledNodeKind::ReduceStart { initial, .. } = node(&workflow, 1)?.kind else {
        return Err("missing ReduceStart".to_owned());
    };
    assert_eq!(workflow.constant(initial), Some(&ConstValue::I64(10)));
    Ok(())
}

#[test]
fn set_last_reduce_result_is_i64_for_bool_rejection() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: folded\n    reduce:\n      variable: total\n      input: '8'\n      initial: '10'\n      steps:\n{TWO_STEPS}  - id: probed\n    set: {{output: probed_value, value: 'not $steps.folded'}}\n  - id: done\n    finish: {{result: '$steps.probed'}}\n"
    );
    rejects(&text, |e| matches!(e, CompileError::TypeMismatch { .. }))
}
