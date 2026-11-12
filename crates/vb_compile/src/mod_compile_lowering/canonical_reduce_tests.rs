//! Compile-only reduce coverage: multi-step layout, typed initial parsing,
//! accumulator type preservation, and loop-body alias rejection.
//!

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
    assert_eq!(parts.nodes.len(), 8);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let accumulator = output(&workflow, 6)?;
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
    assert_eq!(done, StepIdx::new(6));
    assert_ne!(output(&workflow, 3)?, accumulator);
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::Copy {
            source: output(&workflow, 3)?
        }
    );
    assert_eq!(output(&workflow, 4)?, accumulator);
    assert_eq!(
        node(&workflow, 4)?.routing.next,
        NodeContinuation::Step(StepIdx::new(5))
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::ReduceNext {
            iterator_slot: output(&workflow, 1)?,
            accumulator,
            body: StepIdx::new(2),
            done: StepIdx::new(6),
        }
    );
    assert_eq!(
        node(&workflow, 6)?.kind,
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
        (
            "9223372036854775807",
            ConstValue::I64(i64::MAX),
            ExprType::I64,
        ),
        (
            "-9223372036854775808",
            ConstValue::I64(i64::MIN),
            ExprType::I64,
        ),
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
        "-9223372036854775809",
        "inf",
        "NaN",
        "1e999",
    ] {
        let rejected = parse_typed_initial(text, 0);
        let ok = matches!(
            &rejected,
            Err(errors) if errors.0.iter().any(|e| matches!(
                e,
                CompileError::StepFieldShape {
                    field: "aggregate.initial",
                    expected: "a literal constant: null, boolean, integer, or finite float",
                    ..
                }
            ))
        );
        assert!(
            ok,
            "initial {text} must fail with the canonical initial-shape error: {rejected:?}"
        );
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
    assert_eq!(workflow.to_parts().nodes.len(), 12);
    assert_eq!(
        node(&workflow, 2)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 1.into(),
            body: StepIdx::new(3),
            done: StepIdx::new(6),
        }
    );
    assert_ne!(output(&workflow, 7)?, output(&workflow, 10)?);
    assert_eq!(
        node(&workflow, 8)?.kind,
        CompiledNodeKind::Copy {
            source: output(&workflow, 7)?
        }
    );
    assert_eq!(output(&workflow, 8)?, output(&workflow, 10)?);
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
    assert_eq!(workflow.to_parts().nodes.len(), 8);
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
    assert_ne!(output(&workflow, 3)?, output(&workflow, 6)?);
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::Copy {
            source: output(&workflow, 3)?
        }
    );
    assert_eq!(output(&workflow, 4)?, output(&workflow, 6)?);
    Ok(())
}

#[test]
fn do_last_reduce_result_is_unknown_for_bool_operation() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: folded\n    reduce:\n      variable: total\n      input: '8'\n      initial: '10'\n      steps:\n        - id: tmp_step\n          set: {{output: tmp_value, value: '2'}}\n        - id: update\n          do:\n            action: '0'\n            input: '8'\n  - id: probed\n    set: {{output: probed_value, value: 'not $steps.folded'}}\n  - id: done\n    finish: {{result: '$steps.probed'}}\n"
    );
    let workflow = compile(&text)?;
    let CompiledNodeKind::ReduceStart { initial, .. } = node(&workflow, 1)?.kind else {
        return Err("missing ReduceStart".to_owned());
    };
    assert_eq!(workflow.constant(initial), Some(&ConstValue::I64(10)));
    Ok(())
}

#[test]
fn final_nested_container_contributes_the_accumulator_value() -> Result<(), String> {
    let body = "        - id: tmp_step\n          set: {output: tmp_value, value: '2'}\n        - id: loop\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner\n                set: {output: inner_value, value: '6'}\n";
    let workflow = compile(&source("10", body))?;
    assert_eq!(workflow.node_count(), 12);
    let accumulator = output(&workflow, 10)?;
    let result = output(&workflow, 7)?;
    assert_ne!(result, accumulator);
    assert_eq!(
        node(&workflow, 8)?.kind,
        CompiledNodeKind::Copy { source: result }
    );
    assert_eq!(output(&workflow, 8)?, accumulator);
    assert_eq!(
        node(&workflow, 9)?.kind,
        CompiledNodeKind::ReduceNext {
            iterator_slot: output(&workflow, 1)?,
            accumulator,
            body: StepIdx::new(2),
            done: StepIdx::new(10),
        }
    );
    let branch = "        - id: tmp_step\n          set: {output: tmp_value, value: '2'}\n        - id: route\n          choose:\n            branches:\n              - when: 'true'\n                steps:\n                  - id: inner\n                    set: {output: inner_value, value: '6'}\n            otherwise: done\n";
    rejects(&source("10", branch), |e| {
        matches!(e,
        CompileError::UnknownStepLabel { label, .. } if label.as_ref() == "done")
    })?;
    let scoped_branch = branch.replace("            otherwise: done\n", "");
    let workflow = compile(&source("10", &scoped_branch))?;
    let accumulator = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::ReduceFinish { accumulator } => Some(accumulator),
            _ => None,
        })
        .ok_or("missing ReduceFinish")?;
    let parts = workflow.to_parts();
    let update = parts
        .nodes
        .iter()
        .find(|node| {
            node.output.slot() == Some(accumulator)
                && matches!(node.kind, CompiledNodeKind::Copy { .. })
        })
        .ok_or("missing accumulator Copy")?;
    assert_eq!(
        update.kind,
        CompiledNodeKind::Copy {
            source: output(&workflow, 2)?
        }
    );
    assert!(
        parts
            .nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::Choose { .. }))
    );
    Ok(())
}

#[test]
fn deep_aggregate_variable_alias_rejected() -> Result<(), String> {
    rejects(
        &source(
            "10",
            "        - id: folded\n          reduce:\n            variable: total\n            input: '8'\n            initial: '0'\n            steps:\n              - id: update\n                set: {output: local_value, value: '$total'}\n        - id: update\n          set: {output: local_value, value: '$total + $steps.folded'}\n",
        ),
        |e| {
            matches!(
                e,
                CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"
            )
        },
    )
}

#[test]
fn deep_choose_branch_alias_rejected() -> Result<(), String> {
    rejects(
        &source(
            "10",
            "        - id: route\n          choose:\n            branches:\n              - when: 'true'\n                steps:\n                  - id: taken\n                    set: {output: total, value: '6'}\n            otherwise: done\n        - id: update\n          set: {output: local_value, value: '$total'}\n",
        ),
        |e| matches!(e, CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"),
    )
}

#[test]
fn deep_together_in_repeat_alias_rejected() -> Result<(), String> {
    rejects(
        &source(
            "10",
            "        - id: loop\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: fan\n                together:\n                  branches:\n                    - label: a\n                      steps:\n                        - id: only\n                          set: {output: total, value: '6'}\n        - id: update\n          set: {output: local_value, value: '$total + $steps.loop'}\n",
        ),
        |e| {
            matches!(
                e,
                CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"
            )
        },
    )
}

#[test]
fn reduce_do_repeat_cross_lane_layout() -> Result<(), String> {
    let body = "        - id: loop\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner\n                set: {output: tmp_value, value: '6'}\n              - id: act\n                do:\n                  action: '0'\n                  input: '1'\n        - id: update\n          do:\n            action: '0'\n            input: '1'\n";
    let workflow = compile(&source("10", body))?;
    let parts = workflow.to_parts();
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let do_ids: Vec<u16> = parts
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, CompiledNodeKind::Do { .. }))
        .map(|n| n.id.as_usize() as u16)
        .collect();
    assert_eq!(do_ids.len(), 2);
    let final_do = do_ids[1];
    let accumulator = output(
        &workflow,
        workflow
            .node_count()
            .checked_sub(2)
            .ok_or_else(|| "reduce workflow too small".to_owned())?,
    )?;
    let copy = final_do.checked_add(1).ok_or("copy index overflow")?;
    assert_ne!(output(&workflow, final_do)?, accumulator);
    assert_eq!(
        node(&workflow, copy)?.kind,
        CompiledNodeKind::Copy {
            source: output(&workflow, final_do)?
        }
    );
    assert_eq!(output(&workflow, copy)?, accumulator);
    assert_eq!(
        node(&workflow, final_do)?.routing.next,
        NodeContinuation::Step(StepIdx::new(copy))
    );
    Ok(())
}

#[test]
fn set_last_reduce_result_is_i64_for_bool_rejection() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: folded\n    reduce:\n      variable: total\n      input: '8'\n      initial: '10'\n      steps:\n{TWO_STEPS}  - id: probed\n    set: {{output: probed_value, value: 'not $steps.folded'}}\n  - id: done\n    finish: {{result: '$steps.probed'}}\n"
    );
    rejects(&text, |e| matches!(e, CompileError::TypeMismatch { .. }))
}

mod generic_body_regressions {
    use crate::{CompileError, compile_workflow};
    use vb_core::{CompiledNodeKind, StepIdx};

    const HEADER: &str = "version: velvet-ballistics/v1\nname: reduce_test\nwhen: {manual: {}}\n";

    fn compile(text: &str) -> Result<vb_core::CompiledWorkflow, String> {
        compile_workflow(text.as_bytes()).map_err(|e| format!("compile: {e:?}"))
    }

    fn node(
        workflow: &vb_core::CompiledWorkflow,
        id: u16,
    ) -> Result<&vb_core::CompiledNode, String> {
        workflow
            .node(StepIdx::new(id))
            .ok_or_else(|| format!("missing node {id}"))
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
    fn reduce_numeric8_two_set_steps() -> Result<(), String> {
        let text = format!(
            "{HEADER}steps:\n  - id: before\n    set: {{output: answer, value: '42'}}\n  - id: sum\n    reduce:\n      input: \"8\"\n      variable: acc\n      initial: '10'\n      steps:\n        - id: inc\n          set: {{output: step1, value: '$acc + 1'}}\n        - id: dbl\n          set: {{output: step2, value: '$step1 * 2'}}\n  - id: done\n    finish: {{result: 'answer'}}\n"
        );
        let workflow = compile(&text)?;
        let parts = workflow.to_parts();
        // node 0: initial answer set
        // node 1: reduce copy input
        // node 2: reduce start
        // node 3: first body step (set step1 = acc + 1)
        // node 4: second body step (set step2 = step1 * 2)
        // node 5: accumulator copy (multi-body adds copy node)
        // node 6: reduce next
        // node 7: reduce done
        // node 8: finish
        assert_eq!(parts.nodes.len(), 9);
        for (index, node) in parts.nodes.iter().enumerate() {
            assert_eq!(node.id.as_usize(), index);
        }
        let start = node(&workflow, 2)?;
        assert!(
            matches!(start.kind, CompiledNodeKind::ReduceStart { .. }),
            "expected ReduceStart at node 2"
        );
        let next = node(&workflow, 6)?;
        assert!(
            matches!(next.kind, CompiledNodeKind::ReduceNext { .. }),
            "expected ReduceNext at node 6"
        );
        let finish = node(&workflow, 7)?;
        assert!(
            matches!(finish.kind, CompiledNodeKind::ReduceFinish { .. }),
            "expected ReduceFinish at node 7"
        );
        let copy = node(&workflow, 5)?;
        assert!(
            matches!(copy.kind, CompiledNodeKind::Copy { .. }),
            "expected accumulator Copy at node 5"
        );
        Ok(())
    }

    #[test]
    fn reduce_single_set_step_parity() -> Result<(), String> {
        let text = format!(
            "{HEADER}steps:\n  - id: before\n    set: {{output: answer, value: '42'}}\n  - id: sum\n    reduce:\n      input: \"8\"\n      variable: acc\n      initial: '10'\n      steps:\n        - id: inc\n          set: {{output: local_acc, value: '$acc + 1'}}\n  - id: done\n    finish: {{result: 'answer'}}\n"
        );
        let workflow = compile(&text)?;
        let parts = workflow.to_parts();
        // node 0: initial answer set
        // node 1: reduce copy input
        // node 2: reduce start
        // node 3: body step (single, no copy node needed)
        // node 4: reduce next
        // node 5: reduce done
        // node 6: finish
        assert_eq!(parts.nodes.len(), 7);
        for (index, node) in parts.nodes.iter().enumerate() {
            assert_eq!(node.id.as_usize(), index);
        }
        let start = node(&workflow, 2)?;
        assert!(
            matches!(start.kind, CompiledNodeKind::ReduceStart { .. }),
            "expected ReduceStart at node 2"
        );
        Ok(())
    }

    #[test]
    fn reduce_lexical_scope() -> Result<(), String> {
        let text = format!(
            "{HEADER}steps:\n  - id: before\n    set: {{output: outside, value: '7'}}\n  - id: reduce_step\n    reduce:\n      input: \"8\"\n      variable: acc\n      initial: '1'\n      steps:\n        - id: use_outside\n          set: {{output: local_acc, value: '$acc + $outside'}}\n  - id: done\n    finish: {{result: 'outside'}}\n"
        );
        let _workflow = compile(&text)?;
        Ok(())
    }

    #[test]
    fn reduce_empty_body_rejected() -> Result<(), String> {
        let text = format!(
            "{HEADER}steps:\n  - id: sum\n    reduce:\n      input: \"8\"\n      variable: acc\n      initial: '0'\n      steps: []\n  - id: done\n    finish: {{result: 'answer'}}\n"
        );
        rejects(&text, |e| {
            matches!(
                e,
                CompileError::StepFieldShape {
                    field: "reduce.steps",
                    ..
                }
            )
        })
    }

    #[test]
    fn reduce_non_i64_final_rejected() -> Result<(), String> {
        let text = format!(
            "{HEADER}steps:\n  - id: sum\n    reduce:\n      input: \"8\"\n      variable: acc\n      initial: '10'\n      steps:\n        - id: first\n          set: {{output: step1, value: '$acc + 1'}}\n        - id: last\n          set: {{output: local_acc, value: 'true'}}\n  - id: done\n    finish: {{result: 'acc'}}\n"
        );
        rejects(&text, |e| {
            matches!(
                e,
                CompileError::StepFieldShape {
                    field: "reduce.steps",
                    ..
                }
            )
        })
    }

    #[test]
    fn reduce_unsupported_primitive_rejected() -> Result<(), String> {
        let text = format!(
            "{HEADER}steps:\n  - id: sum\n    reduce:\n      input: \"8\"\n      variable: acc\n      initial: '0'\n      steps:\n        - id: step_wait\n          wait:\n            timeout: '10'\n  - id: done\n    finish: {{result: 'answer'}}\n"
        );
        // A wait-only body cannot supply the next accumulator value.
        crate::parse_workflow_source(&text).map_err(|e| format!("parse: {e:?}"))?;
        let errors = compile_workflow(text.as_bytes())
            .err()
            .ok_or("wait-only reduce body was accepted")?;
        assert_eq!(
            errors.0,
            [CompileError::StepFieldShape {
                step: 0,
                field: "reduce.steps",
                expected: "at least one set or do step",
            }]
        );
        Ok(())
    }
}
