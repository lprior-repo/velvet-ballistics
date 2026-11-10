//! Compile-only regressions for canonical Reduce lowering.
//! Runtime re-entry is qualified by source-drive tests.

use crate::{CompileError, compile_workflow};
use vb_core::{CompiledNodeKind, StepIdx};

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: reduce_test\nwhen: {manual: {}}\n";

fn compile(text: &str) -> Result<vb_core::CompiledWorkflow, String> {
    compile_workflow(text.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

fn node(workflow: &vb_core::CompiledWorkflow, id: u16) -> Result<&vb_core::CompiledNode, String> {
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
        "{HEADER}steps:\n  - id: before\n    set: {{output: answer, value: '42'}}\n  - id: sum\n    reduce:\n      input: \"0\"\n      variable: acc\n      initial: '10'\n      steps:\n        - id: inc\n          set: {{output: step1, value: '$acc + 1'}}\n        - id: dbl\n          set: {{output: step2, value: '$step1 * 2'}}\n  - id: done\n    finish: {{result: 'answer'}}\n"
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
    // Debug: print node kinds
    // println!("node count: {}", parts.nodes.len());
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
        "{HEADER}steps:\n  - id: before\n    set: {{output: answer, value: '42'}}\n  - id: sum\n    reduce:\n      input: \"0\"\n      variable: acc\n      initial: '10'\n      steps:\n        - id: inc\n          set: {{output: acc, value: '$acc + 1'}}\n  - id: done\n    finish: {{result: 'answer'}}\n"
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
        "{HEADER}steps:\n  - id: before\n    set: {{output: outside, value: '7'}}\n  - id: reduce_step\n    reduce:\n      input: \"0\"\n      variable: acc\n      initial: '1'\n      steps:\n        - id: use_outside\n          set: {{output: acc, value: '$acc + $outside'}}\n  - id: done\n    finish: {{result: 'outside'}}\n"
    );
    let _workflow = compile(&text)?;
    Ok(())
}

#[test]
fn reduce_empty_body_rejected() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: sum\n    reduce:\n      input: \"0\"\n      variable: acc\n      initial: '0'\n      steps: []\n  - id: done\n    finish: {{result: 'answer'}}\n"
    );
    rejects(&text, |e| {
        matches!(e, CompileError::StepFieldShape { field: "reduce.steps", .. })
    })
}

#[test]
fn reduce_non_i64_final_rejected() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: sum\n    reduce:\n      input: \"0\"\n      variable: acc\n      initial: '10'\n      steps:\n        - id: first\n          set: {{output: step1, value: '$acc + 1'}}\n        - id: last\n          set: {{output: acc, value: 'true'}}\n  - id: done\n    finish: {{result: 'acc'}}\n"
    );
    rejects(&text, |e| {
        matches!(e, CompileError::StepFieldShape { field: "reduce.steps", .. })
    })
}

#[test]
fn reduce_unsupported_primitive_rejected() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: sum\n    reduce:\n      input: \"0\"\n      variable: acc\n      initial: '0'\n      steps:\n        - id: step_wait\n          wait:\n            timeout: '10s'\n  - id: done\n    finish: {{result: 'answer'}}\n"
    );
    rejects(&text, |e| {
        matches!(e, CompileError::UnsupportedStepPrimitive { .. })
    })
}
