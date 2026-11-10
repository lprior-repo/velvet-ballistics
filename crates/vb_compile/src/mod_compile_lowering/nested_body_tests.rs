//! Nested body composition: loops and branches inside loop bodies.
//!
//! Compile-only layout proofs; execution is qualified by runtime drive tests
//! and the workspace `nested_composition` end-to-end suite.

use crate::{CompileError, compile_workflow};
use vb_core::workflow::{NodeContinuation, NodeOutput};
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx};

const HEADER: &str = "version: velvet-ballistics/v1\nname: nested_bodies\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  outer_items: {is: list, of: any, max: 4}\n  inner_items: {is: list, of: any, max: 4}\n";

fn source(body: &str) -> String {
    format!("{HEADER}{INPUTS}steps:\n{body}")
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

fn finish_tail() -> &'static str {
    "  - id: done\n    finish: {result: '$seed'}\n"
}

#[test]
fn nested_for_each_in_for_each_layout() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 7);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let CompiledNodeKind::ForEachStart {
        body: outer_body,
        done: outer_done,
        limit: outer_limit,
        ..
    } = &node(&workflow, 1)?.kind
    else {
        return Err("node 1 is not the outer ForEachStart".to_owned());
    };
    assert_eq!(
        (*outer_body, *outer_done, *outer_limit),
        (StepIdx::new(2), StepIdx::new(6), 4)
    );
    let CompiledNodeKind::ForEachStart {
        body: inner_body,
        done: inner_done,
        limit: inner_limit,
        ..
    } = &node(&workflow, 2)?.kind
    else {
        return Err("node 2 is not the inner ForEachStart".to_owned());
    };
    assert_eq!(
        (*inner_body, *inner_done, *inner_limit),
        (StepIdx::new(3), StepIdx::new(5), 4)
    );
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 2)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(3),
            done: StepIdx::new(5),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 1)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(2),
            done: StepIdx::new(6),
        }
    );
    Ok(())
}

#[test]
fn nested_repeat_in_repeat_threads_inner_value() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: inner\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: leaf\n                set: {output: picked, value: '7'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 11);
    let leaf = output(&workflow, 3)?;
    let inner_result = output(&workflow, 4)?;
    let outer_result = output(&workflow, 7)?;
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::Copy { source: leaf }
    );
    assert_eq!(
        node(&workflow, 6)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: inner_result
        }
    );
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::Copy {
            source: inner_result
        }
    );
    assert_eq!(
        node(&workflow, 9)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: outer_result
        }
    );
    Ok(())
}

#[test]
fn nested_choose_in_for_each_layout() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: flag\n    set: {output: go, value: 'true'}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: route\n          choose:\n            branches:\n              - when: '$flag'\n                steps:\n                  - id: taken\n                    set: {output: picked, value: '$entry'}\n            otherwise: after_route\n        - id: after_route\n          set: {output: after_route, value: '$entry'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.nodes.len(), 8);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    assert_eq!(
        node(&workflow, 4)?.routing.next,
        NodeContinuation::Step(StepIdx::new(5))
    );
    assert_eq!(
        node(&workflow, 6)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: node(&workflow, 2)?
                .output
                .slot()
                .ok_or("missing iterator")?,
            body: StepIdx::new(3),
            done: StepIdx::new(7),
        }
    );
    let CompiledNodeKind::Choose { otherwise, .. } = node(&workflow, 3)?.kind else {
        return Err("missing nested Choose".to_owned());
    };
    assert_eq!(otherwise, Some(StepIdx::new(5)));
    Ok(())
}

#[test]
fn nested_for_each_in_repeat_does_not_export_leaf_value() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: sub\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$sub'}\n        - id: grab\n          set: {output: grabbed, value: '$step.sub'}\n",
    ) + finish_tail();
    rejects(
        &text,
        |e| matches!(e, CompileError::UnknownReferenceName { kind: "step", name, .. } if name.as_ref() == "sub"),
    )
}

#[test]
fn nested_repeat_in_repeat_with_step_reference() -> Result<(), String> {
    let text = source(
        "  - id: seed\n    set: {output: base, value: '1'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: sub\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: leaf\n                set: {output: picked, value: '7'}\n        - id: grab\n          set: {output: grabbed, value: '$step.sub'}\n",
    ) + finish_tail();
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 12);
    let leaf = output(&workflow, 3)?;
    let inner = output(&workflow, 4)?;
    let grab = output(&workflow, 7)?;
    let result = output(&workflow, 8)?;
    assert_ne!(leaf, grab);
    assert_ne!(leaf, inner);
    assert_eq!(
        node(&workflow, 8)?.kind,
        CompiledNodeKind::Copy { source: grab }
    );
    assert_eq!(
        node(&workflow, 10)?.kind,
        CompiledNodeKind::RepeatFinish { result }
    );
    assert_eq!(node(&workflow, 10)?.output, NodeOutput::Slot(result));
    Ok(())
}

#[path = "nested_body_tests/controls.rs"]
mod controls;
