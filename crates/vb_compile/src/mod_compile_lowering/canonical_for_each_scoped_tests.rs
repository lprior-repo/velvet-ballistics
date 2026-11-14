//! Nested ForEach: scoped bodies via shared dispatch, budgets, taint.
//! Bead: vb-eirmk.4.3. All tests drive public `compile_workflow`.
#![forbid(unsafe_code)]

use crate::{CompileError, compile_workflow};
use vb_core::budget::WholeWorkflowBudget;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx};

const HEADER: &str = "version: velvet-ballistics/v1\nname: for_each_scoped\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  outer_items: {is: list, of: any, max: 4}\n  inner_items: {is: list, of: any, max: 4}\n";

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
        Ok(_) => return Err("unexpected acceptance".to_owned()),
        Err(errors) => errors,
    };
    if errors.0.iter().any(predicate) {
        Ok(())
    } else {
        Err(format!("wrong diagnostic: {errors:?}"))
    }
}

fn source(body: &str) -> String {
    format!("{HEADER}{INPUTS}steps:\n{body}")
}

fn finish_tail() -> &'static str {
    "  - id: done\n    finish: {result: '$seed'}\n"
}

fn nested_body(inner_steps: &str) -> String {
    source(&format!(
        "  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n{inner_steps}"
    )) + finish_tail()
}

#[test]
fn nested_layout_limits_and_slots() -> Result<(), String> {
    let body = "            steps:\n              - id: leaf\n                set: {output: picked, value: '$entry'}\n";
    let wf = compile(&nested_body(body))?;
    assert_eq!(wf.to_parts().nodes.len(), 15);
    let CompiledNodeKind::ForEachStart {
        item_slot: outer_item,
        body: outer_body,
        done: outer_done,
        limit: outer_limit,
        ..
    } = &node(&wf, 3)?.kind
    else {
        return Err("node 3 not outer ForEachStart".to_owned());
    };
    assert_eq!(
        (*outer_body, *outer_done, outer_limit.get()),
        (StepIdx::new(4), StepIdx::new(13), 4)
    );
    let CompiledNodeKind::ForEachStart {
        item_slot: inner_item,
        body: inner_body,
        done: inner_done,
        limit: inner_limit,
        ..
    } = &node(&wf, 6)?.kind
    else {
        return Err("node 6 not inner ForEachStart".to_owned());
    };
    assert_eq!(
        (*inner_body, *inner_done, inner_limit.get()),
        (StepIdx::new(7), StepIdx::new(10), 4)
    );
    assert_ne!(outer_item, inner_item);
    Ok(())
}

#[test]
fn nested_empty_single_multiple_bodies() -> Result<(), String> {
    let empty = nested_body("            steps: []\n");
    let wf_empty = compile(&empty)?;
    assert_eq!(wf_empty.to_parts().nodes.len(), 14);
    let single = nested_body(
        "            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n",
    );
    let wf_single = compile(&single)?;
    assert_eq!(wf_single.to_parts().nodes.len(), 15);
    let multi = nested_body(
        "            steps:\n              - id: pre\n                set: {output: pre_val, value: '$sub'}\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n",
    );
    let wf_multi = compile(&multi)?;
    assert_eq!(wf_multi.to_parts().nodes.len(), 16);
    Ok(())
}

#[test]
fn sibling_top_level_loops_isolated() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: first\n    for_each:\n      input: '$input.outer_items'\n      variable: a_var\n      steps:\n        - id: leaf\n          set: {output: x_val, value: '$a_var'}\n  - id: second\n    for_each:\n      input: '$input.inner_items'\n      variable: b_var\n      steps:\n        - id: leaf2\n          set: {output: y_val, value: '$b_var'}\n",
    ) + finish_tail();
    let wf = compile(&text)?;
    assert_eq!(wf.to_parts().nodes.len(), 16);
    let mut starts = 0;
    for node in wf.to_parts().nodes.iter() {
        if matches!(node.kind, CompiledNodeKind::ForEachStart { .. }) {
            starts += 1;
        }
    }
    assert_eq!(starts, 2);
    Ok(())
}

#[test]
fn suspending_do_in_nested_for_each() -> Result<(), String> {
    let body = "            steps:\n              - id: act\n                do: {action: '0', input: '$sub'}\n";
    let wf = compile(&nested_body(body))?;
    assert_eq!(wf.to_parts().nodes.len(), 15);
    assert!(matches!(&node(&wf, 7)?.kind, CompiledNodeKind::Do { .. }));
    Ok(())
}

#[test]
fn no_foreign_capture_rejected() -> Result<(), String> {
    let leak = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n        - id: after\n          set: {output: after_val, value: '$sub'}\n",
    ) + finish_tail();
    rejects(
        &leak,
        |e| matches!(e, CompileError::UnknownReferenceName { kind: "output", name, .. } if name.as_ref() == "sub"),
    )?;
    let shadow = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: entry\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$entry'}\n",
    ) + finish_tail();
    rejects(
        &shadow,
        |e| matches!(e, CompileError::DuplicateOutputName { name } if name.as_ref() == "entry"),
    )
}

#[test]
fn at_once_preserved_and_zero_rejected() -> Result<(), String> {
    for (at_once, expected) in [("1", 9), ("2", 6), ("4294967295", 6)] {
        let text = source(&format!(
            "  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      at_once: {at_once}\n      steps:\n        - id: leaf\n          set: {{output: picked, value: '$entry'}}\n"
        )) + finish_tail();
        let wf = compile(&text)?;
        assert_eq!(wf.to_parts().nodes.len(), expected);
    }
    let zero = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      at_once: 0\n      steps:\n        - id: leaf\n          set: {output: picked, value: '$entry'}\n",
    ) + finish_tail();
    rejects(&zero, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "for_each.at_once",
                ..
            }
        )
    })?;
    let nested_zero = nested_body(
        "            at_once: 0\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n",
    );
    rejects(&nested_zero, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "for_each.at_once",
                ..
            }
        )
    })
}

#[test]
fn taint_append_if_and_item_slots() -> Result<(), String> {
    let body = "            steps:\n              - id: leaf\n                set: {output: picked, value: '$entry'}\n";
    let wf = compile(&nested_body(body))?;
    let parts = wf.to_parts();
    let taint_exprs = parts
        .expressions
        .iter()
        .filter(|p| {
            p.ops
                .iter()
                .any(|op| matches!(op, vb_core::ExprOp::AppendIf))
        })
        .count();
    assert_eq!(taint_exprs, 2);
    let outer = output(&wf, 3)?;
    let inner = output(&wf, 6)?;
    assert_ne!(outer, inner);
    Ok(())
}

#[test]
fn exact_budgets_nested_declared() -> Result<(), String> {
    let body = "            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n";
    let wf = compile(&nested_body(body))?;
    let parts = wf.to_parts();
    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(budget.max_for_each_iterations, 8);
    assert_eq!(budget.max_total_steps, 78);
    Ok(())
}

#[test]
fn source_handle_and_ordered_outputs() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: first\n    for_each:\n      variable: entry\n      input: $input.outer_items\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry'}\n  - id: loop\n    for_each:\n      variable: next_entry\n      input: $step.first\n      steps:\n        - id: previous\n          set: {output: previous, value: '$next_entry'}\n        - id: last\n          set: {output: last_value, value: '7'}\n",
    ) + "  - id: done\n    finish: {result: '$seed'}\n";
    let wf = compile(&text)?;
    assert!(
        wf.to_parts()
            .nodes
            .iter()
            .any(|n| matches!(&n.kind, CompiledNodeKind::ForEachJoin { .. }))
    );
    let row_text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: row\n      steps:\n        - id: inner\n          for_each:\n            input: '$row'\n            variable: cell\n            steps:\n              - id: proj\n                set: {output: proj, value: '$cell'}\n",
    ) + finish_tail();
    let wf2 = compile(&row_text)?;
    assert_eq!(wf2.to_parts().nodes.len(), 15);
    Ok(())
}

#[test]
fn deterministic_slots_nested() -> Result<(), String> {
    let body = "            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n";
    let text = nested_body(body);
    let a = compile(&text)?;
    let b = compile(&text)?;
    assert_eq!(a.to_parts().nodes, b.to_parts().nodes);
    assert_eq!(a.to_parts().slot_count, b.to_parts().slot_count);
    Ok(())
}
