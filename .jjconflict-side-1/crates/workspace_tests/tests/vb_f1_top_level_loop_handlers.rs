#![forbid(unsafe_code)]

#[path = "vb_f2_signal_body_results/support.rs"]
mod support;
use support::*;
use vb_core::{CompiledNodeKind as K, ConstValue, ExprOp, SlotValue};
use vb_runtime::journal::RuntimeJournalEvent as E;

fn handled_body_error(primitive: &str, nested_together: bool) -> TestResult {
    let fault_body = if nested_together {
        "- id: nested\n  together:\n    branches:\n      - label: ready\n        steps:\n          - id: before\n            set: {output: before, value: '13'}\n      - label: failing\n        steps:\n          - id: fault\n            set: {output: fault, value: '1 / $input.zero'}\n- id: after\n  set: {output: after, value: '7'}\n"
    } else {
        "- id: fault\n  set: {output: fault, value: '1 / $input.zero'}\n"
    };
    let fallback = if nested_together {
        "- id: fallback\n  together:\n    branches:\n      - label: recovered\n        steps:\n          - id: recovered\n            set: {output: recovered, value: '77'}\n      - label: companion\n        steps:\n          - id: companion\n            set: {output: companion, value: '88'}\n"
    } else {
        "- id: fallback\n  set: {output: recovered, value: '77'}\n"
    };
    let body = format!(
        "- id: loop\n{}    steps:\n{}  on_error: {{handler: fallback}}\n  then: done\n{fallback}",
        indent(primitive, 2),
        indent(fault_body, 6),
    );
    let yaml = source(
        "inputs: {zero: number}\nvars: {xs: [3, 5]}\n",
        &body,
        "$saved",
    );
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    vb_validate::shared::validate(&plan.to_parts())?;
    let mut parts = plan.to_parts();
    if nested_together {
        parts.resource_contract.max_fanout = 2;
    }
    let plan = vb_core::CompiledWorkflow::try_from_parts(parts.clone())?;
    vb_validate::shared::validate(&parts)?;
    let group_joins = parts
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, K::TogetherJoin { .. }))
        .map(|node| node.id)
        .collect::<Vec<_>>();
    if nested_together {
        assert_eq!(group_joins.len(), 2);
        assert_eq!(plan.resource_contract().max_fanout, 2);
    } else {
        assert_eq!(group_joins.len(), 0);
    }
    let fault = parts
        .nodes
        .iter()
        .find(|node| match node.kind {
            K::EvalExpr { expr } => plan
                .expression(expr)
                .is_some_and(|program| program.ops.contains(&ExprOp::Div)),
            _ => false,
        })
        .ok_or("division step")?
        .id;
    let recovery = parts
        .nodes
        .iter()
        .find(|node| match node.kind {
            K::SetConst { value } => plan.constant(value) == Some(&ConstValue::I64(77)),
            K::EvalExpr { expr } => plan.expression(expr).is_some_and(|program| {
                matches!(program.ops.as_ref(), [ExprOp::LoadConst(value)]
                    if plan.constant(*value) == Some(&ConstValue::I64(77)))
            }),
            _ => false,
        })
        .ok_or("handler value step")?;
    let recovery_output = recovery.output.slot().ok_or("handler output")?;
    let finishes = parts
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                K::ForEachJoin { .. }
                    | K::CollectFinish { .. }
                    | K::ReduceFinish { .. }
                    | K::RepeatFinish { .. }
            )
        })
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(finishes.len(), 1);
    for quantum in [1, 64] {
        let outcome = if nested_together {
            execute_compiled(
                vb_core::CompiledWorkflow::try_from_parts(parts.clone())?,
                quantum,
                Some(SlotValue::I64(0)),
            )?
        } else {
            execute(&yaml, quantum, Some(SlotValue::I64(0)))?
        };
        assert_eq!(outcome.value, SlotValue::I64(42));
        assert_eq!((outcome.asks, outcome.waits), (0, 0));
        let failures = outcome
            .events
            .iter()
            .filter_map(|event| match event {
                E::StepFailed { step, .. } => Some(*step),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(failures, vec![fault]);
        let completed_finishes = outcome
            .events
            .iter()
            .filter_map(|event| match event {
                E::StepSucceeded { step, .. } if finishes.contains(step) => Some(*step),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(completed_finishes, Vec::new());
        if nested_together {
            let completed_groups = outcome
                .events
                .iter()
                .filter_map(|event| match event {
                    E::StepSucceeded { step, .. } if group_joins.contains(step) => Some(*step),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                completed_groups,
                vec![*group_joins.last().ok_or("handler group join")?]
            );
        }
        let recovered = outcome
            .events
            .iter()
            .filter_map(|event| match event {
                E::SlotWritten { slot, value, .. } if *slot == recovery_output => {
                    Some(postcard::from_bytes::<SlotValue>(value))
                }
                _ => None,
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(recovered, vec![SlotValue::I64(77)]);
        let recovery_lifecycle = outcome
            .events
            .iter()
            .filter_map(|event| match event {
                E::StepStarted { step, .. } if *step == recovery.id => Some(false),
                E::StepSucceeded { step, .. } if *step == recovery.id => Some(true),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(recovery_lifecycle, vec![false, true]);
    }
    Ok(())
}

#[test]
fn repeat_body_error_reaches_top_level_handler() -> TestResult {
    for nested in [false, true] {
        handled_body_error("repeat:\n  max_attempts: 2", nested)?;
    }
    Ok(())
}

#[test]
fn collect_body_error_reaches_top_level_handler() -> TestResult {
    for nested in [false, true] {
        handled_body_error(
            "collect:\n  variable: batch_value\n  source: $vars.xs\n  pages: 2\n  items: 1",
            nested,
        )?;
    }
    Ok(())
}

#[test]
fn reduce_body_error_reaches_top_level_handler() -> TestResult {
    for nested in [false, true] {
        handled_body_error(
            "reduce:\n  variable: total\n  input: $vars.xs\n  initial: '0'",
            nested,
        )?;
    }
    Ok(())
}

#[test]
fn for_each_body_error_reaches_top_level_handler() -> TestResult {
    for nested in [false, true] {
        handled_body_error("for_each:\n  variable: entry\n  input: $vars.xs", nested)?;
    }
    Ok(())
}
