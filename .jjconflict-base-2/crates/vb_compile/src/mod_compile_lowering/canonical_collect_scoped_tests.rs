//! Scoped Collect: bounded page/item caps, lexical owner isolation, and
//! continuation preservation through the shared scoped dispatch.
//! Bead: vb-eirmk.4.4.1. All tests drive the public `compile_workflow` API;
//! no constructor-only coverage.
#![forbid(unsafe_code)]

use crate::{CompileError, compile_workflow};
use vb_core::workflow::NodeContinuation;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, FanoutLimit, SlotIdx, StepIdx};

const HEADER: &str = "version: velvet-ballistics/v1\nname: collect_scoped\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  items: {is: list, of: any, max: 8}\n";

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

fn top_collect(pages: &str, items: &str, body: &str) -> String {
    format!(
        "{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: seed, value: '1'}}\n  - id: gather\n    collect:\n      variable: batch\n      source: '$input.items'\n      {pages}\n      {items}\n      steps:\n{body}  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

fn collect_body(output: &str, value: &str) -> String {
    format!("        - id: consume\n          set: {{output: {output}, value: '{value}'}}\n")
}

#[test]
fn top_level_collect_layout_caps_and_continuation() -> Result<(), String> {
    let body = collect_body("x_val", "$batch");
    let workflow = compile(&top_collect("pages: 2", "items: 3", &body))?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 6);
    for (i, n) in parts.nodes.iter().enumerate() {
        assert_eq!(n.id.as_usize(), i);
    }
    let collector = output(&workflow, 1)?;
    let finish_slot = output(&workflow, 4)?;
    assert_ne!(collector, finish_slot);
    let CompiledNodeKind::CollectStart {
        source,
        limit,
        page_size,
        body,
        done,
    } = &node(&workflow, 1)?.kind
    else {
        return Err("node 1 is not CollectStart".to_owned());
    };
    assert_eq!(*limit, FanoutLimit::new(6));
    assert_eq!(*page_size, 3);
    assert_eq!(*body, StepIdx::new(2));
    assert_eq!(*done, StepIdx::new(4));
    assert_eq!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::CollectNext {
            collector_slot: collector,
            body: StepIdx::new(2),
            done: StepIdx::new(4)
        }
    );
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::CollectFinish {
            collector_slot: collector
        }
    );
    let _ = source;
    assert_eq!(
        node(&workflow, 4)?.routing.next,
        NodeContinuation::Step(StepIdx::new(5))
    );
    Ok(())
}

#[test]
fn collect_missing_and_zero_bounds_are_rejected() -> Result<(), String> {
    let body = collect_body("x_val", "$batch");
    let missing = format!(
        "{HEADER}{INPUTS}steps:\n  - id: gather\n    collect:\n      variable: batch\n      source: '$input.items'\n      steps:\n{body}  - id: done\n    finish: {{result: '$batch'}}\n"
    );
    rejects(&missing, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "collect",
                ..
            }
        )
    })?;
    let zero = top_collect("pages: 0", "items: 1", &body);
    rejects(&zero, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "collect",
                ..
            }
        )
    })?;
    Ok(())
}

#[test]
fn collect_empty_body_is_rejected() -> Result<(), String> {
    let text = format!(
        "{HEADER}{INPUTS}steps:\n  - id: gather\n    collect:\n      variable: batch\n      source: '$input.items'\n      pages: 1\n      items: 1\n      steps: []\n  - id: done\n    finish: {{result: '$batch'}}\n"
    );
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "collect.body",
                ..
            }
        )
    })
}

#[test]
fn collect_reserved_and_step_variables_are_rejected() -> Result<(), String> {
    for variable in ["page", "step", "input", "collect"] {
        let text = format!(
            "{HEADER}{INPUTS}steps:\n  - id: gather\n    collect:\n      variable: {variable}\n      source: '$input.items'\n      pages: 1\n      items: 1\n      steps:\n        - id: consume\n          set: {{output: x_val, value: '$batch'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
        );
        rejects(&text, |e| {
            matches!(
                e,
                CompileError::InvalidName {
                    field: "collect.variable",
                    ..
                }
            )
        })?;
    }
    Ok(())
}

#[test]
fn collect_variable_does_not_escape_to_next_step() -> Result<(), String> {
    let text = format!(
        "{HEADER}{INPUTS}steps:\n  - id: gather\n    collect:\n      variable: batch\n      source: '$input.items'\n      pages: 1\n      items: 1\n      steps:\n        - id: consume\n          set: {{output: x_val, value: '$batch'}}\n  - id: after\n    set: {{output: after_val, value: '$batch'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(
        &text,
        |e| matches!(e, CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "batch"),
    )
}

#[test]
fn collect_outer_capture_is_rejected() -> Result<(), String> {
    let text = format!(
        "{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: batch, value: '1'}}\n  - id: gather\n    collect:\n      variable: batch\n      source: '$input.items'\n      pages: 1\n      items: 1\n      steps:\n        - id: consume\n          set: {{output: x_val, value: '$batch'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(
        &text,
        |e| matches!(e, CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "batch"),
    )
}

#[test]
fn collect_body_alias_is_rejected_shallow_and_deep() -> Result<(), String> {
    let shallow = top_collect("pages: 1", "items: 1", &collect_body("batch", "1"));
    rejects(
        &shallow,
        |e| matches!(e, CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "batch"),
    )?;
    let deep_body = "        - id: loop\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner\n                set: {output: batch, value: '6'}\n";
    let deep = top_collect("pages: 1", "items: 1", deep_body);
    rejects(
        &deep,
        |e| matches!(e, CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "batch"),
    )?;
    Ok(())
}

#[test]
fn nested_collect_in_repeat_preserves_continuation() -> Result<(), String> {
    let text = format!(
        "{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: seed, value: '1'}}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: inner\n          collect:\n            variable: batch2\n            source: '$input.items'\n            pages: 1\n            items: 2\n            steps:\n              - id: use_page\n                set: {{output: out_val, value: '$batch2'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 10);
    let collector = output(&workflow, 2)?;
    let finish = output(&workflow, 5)?;
    assert_ne!(collector, finish);
    let CompiledNodeKind::CollectStart {
        body,
        done,
        limit,
        page_size,
        ..
    } = &node(&workflow, 2)?.kind
    else {
        return Err("node 2 is not CollectStart".to_owned());
    };
    assert_eq!(
        (body.get(), done.get(), *limit, *page_size),
        (3, 5, FanoutLimit::new(2), 2)
    );
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::CollectNext {
            collector_slot: collector,
            body: StepIdx::new(3),
            done: StepIdx::new(5)
        }
    );
    assert_eq!(
        node(&workflow, 5)?.routing.next,
        NodeContinuation::Step(StepIdx::new(6))
    );
    assert_eq!(
        node(&workflow, 6)?.kind,
        CompiledNodeKind::Copy { source: finish }
    );
    assert_ne!(output(&workflow, 6)?, finish);
    assert_eq!(
        node(&workflow, 6)?.routing.next,
        NodeContinuation::Step(StepIdx::new(7))
    );
    Ok(())
}

#[test]
fn nested_collect_in_collection_keeps_two_distinct_collectors() -> Result<(), String> {
    let text = format!(
        "{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: seed, value: '1'}}\n  - id: outer_c\n    collect:\n      variable: outer_batch\n      source: '$input.items'\n      pages: 1\n      items: 1\n      steps:\n        - id: inner_c\n          collect:\n            variable: inner_batch\n            source: '$input.items'\n            pages: 1\n            items: 1\n            steps:\n              - id: leaf\n                set: {{output: out_v, value: '$inner_batch'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 9);
    let outer_collector = output(&workflow, 1)?;
    let inner_collector = output(&workflow, 2)?;
    assert_ne!(outer_collector, inner_collector);
    let CompiledNodeKind::CollectStart { body, done, .. } = &node(&workflow, 1)?.kind else {
        return Err("outer not CollectStart".to_owned());
    };
    assert_eq!((body.get(), done.get()), (2, 7));
    Ok(())
}
