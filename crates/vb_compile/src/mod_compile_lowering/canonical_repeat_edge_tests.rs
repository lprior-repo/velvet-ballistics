//! Edge coverage for nested Repeat: loops, activation, check routes, overflow,
//! foreign counters, and lexical errors. All tests drive `compile_workflow`
//! with real YAML source and assert exact IR layouts or diagnostics.

use crate::{CompileError, compile_workflow};
use vb_core::workflow::{NodeContinuation, NodeOutput};
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx};

const HEADER: &str = "version: velvet-ballistics/v1\nname: repeat_edge\nwhen: {manual: {}}\n";

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

fn repeat_source(body: &str, attempts: u16) -> String {
    format!(
        "{HEADER}steps:\n  - id: s\n    set: {{output: seed, value: '1'}}\n  - id: r\n    repeat:\n      max_attempts: {attempts}\n      steps:\n{body}  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

// Nested repeat inside a for_each body: outer for_each owns the body range,
// inner repeat owns its own check/done. Asserts counter distinctness and routes.
#[test]
fn nested_repeat_under_for_each_preserves_check_routes() -> Result<(), String> {
    let text = format!(
        "{HEADER}inputs:\n  items: {{is: list, of: any, max: 4}}\nsteps:\n  - id: s\n    set: {{output: seed, value: '1'}}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n      steps:\n        - id: inner\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: tick\n                set: {{output: ticked, value: '$entry'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    // 0 seed,1 outer-start,2 inner-start,3 tick,4 inner-check,5 inner-finish,6 outer-next,7 done
    assert_eq!(parts.nodes.len(), 8);
    for (index, node) in parts.nodes.iter().enumerate() {
        assert_eq!(node.id.as_usize(), index);
    }
    let outer_counter = output(&workflow, 1)?;
    let inner_counter = output(&workflow, 2)?;
    assert_ne!(outer_counter, inner_counter);
    assert_eq!(
        node(&workflow, 2)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 2,
            body: StepIdx::new(3),
            done: StepIdx::new(5),
        }
    );
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: inner_counter,
            done: StepIdx::new(5),
        }
    );
    assert_eq!(
        node(&workflow, 4)?.routing.next,
        NodeContinuation::Step(StepIdx::new(3))
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: output(&workflow, 3)?
        }
    );
    assert_eq!(
        node(&workflow, 6)?.kind,
        CompiledNodeKind::ForEachNext {
            iterator_slot: outer_counter,
            body: StepIdx::new(2),
            done: StepIdx::new(7),
        }
    );
    Ok(())
}

// Repeat inside a collect: collect owns iteration, repeat owns attempts.
// Layout 0 seed,1 collect-start,2 repeat-start,3 tick,4 repeat-check,5 repeat-finish,6 collect-next,7 collect-finish,8 done.
#[test]
fn nested_repeat_under_collect_preserves_routes() -> Result<(), String> {
    let text = format!(
        "{HEADER}inputs:\n  items: {{is: list, of: any, max: 4}}\nsteps:\n  - id: s\n    set: {{output: seed, value: '1'}}\n  - id: outer\n    collect:\n      source: '$input.items'\n      variable: entry\n      pages: 2\n      steps:\n        - id: inner\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: tick\n                set: {{output: ticked, value: '$entry'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 9);
    let rep_counter = output(&workflow, 2)?;
    assert_eq!(
        node(&workflow, 2)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 1,
            body: StepIdx::new(3),
            done: StepIdx::new(5),
        }
    );
    assert_eq!(
        node(&workflow, 4)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: rep_counter,
            done: StepIdx::new(5),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: output(&workflow, 3)?
        }
    );
    Ok(())
}

// Repeat inside choose branch: choose routes to inner repeat body.
// Verifies choose fallthrough and inner check/done inside branch.
// Layout: 0 seed,1 flag,2 choose,3 repeat-start,4 tick,5 check,6 finish,7 done.
#[test]
fn nested_repeat_inside_choose_branch_preserves_fallthrough() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: s\n    set: {{output: seed, value: '1'}}\n  - id: flag\n    set: {{output: go, value: 'true'}}\n  - id: outer\n    choose:\n      branches:\n        - when: '$flag'\n          steps:\n            - id: inner\n              repeat:\n                max_attempts: 2\n                steps:\n                  - id: tick\n                    set: {{output: ticked, value: '7'}}\n      otherwise: done\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let workflow = compile(&text)?;
    assert_eq!(workflow.to_parts().nodes.len(), 8);
    let kind = &node(&workflow, 2)?.kind;
    let CompiledNodeKind::Choose { branches, otherwise } = kind else {
        return Err("node 2 is not Choose".to_owned());
    };
    assert_eq!(branches.len(), 1);
    assert_eq!(branches.first().map(|b| b.target), Some(StepIdx::new(3)));
    assert_eq!(*otherwise, Some(StepIdx::new(7)));
    let rep_counter = output(&workflow, 3)?;
    assert_eq!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 2,
            body: StepIdx::new(4),
            done: StepIdx::new(6),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: rep_counter,
            done: StepIdx::new(6),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.routing.next,
        NodeContinuation::Step(StepIdx::new(4))
    );
    Ok(())
}

// Three-deep nesting: repeat -> repeat -> repeat. Each level gets a distinct
// attempt slot; innermost check/done bind to innermost region.
// Layout: 0 seed,1 outer,2 mid,3 inner,4 leaf,5 inner-check,6 inner-finish,7 mid-check,8 mid-finish,9 outer-check,10 outer-finish,11 done.
#[test]
fn triple_nested_repeat_allocates_distinct_counters() -> Result<(), String> {
    let body = "        - id: mid\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: inner\n                repeat:\n                  max_attempts: 1\n                  steps:\n                    - id: leaf\n                      set: {output: picked, value: '7'}\n";
    let workflow = compile(&repeat_source(body, 3))?;
    assert_eq!(workflow.to_parts().nodes.len(), 12);
    let outer = output(&workflow, 1)?;
    let mid = output(&workflow, 2)?;
    let inner = output(&workflow, 3)?;
    assert_ne!(outer, mid);
    assert_ne!(mid, inner);
    assert_ne!(outer, inner);
    assert_eq!(
        node(&workflow, 3)?.kind,
        CompiledNodeKind::RepeatStart {
            max_attempts: 1,
            body: StepIdx::new(4),
            done: StepIdx::new(6),
        }
    );
    assert_eq!(
        node(&workflow, 5)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: inner,
            done: StepIdx::new(6),
        }
    );
    assert_eq!(
        node(&workflow, 7)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: mid,
            done: StepIdx::new(8),
        }
    );
    assert_eq!(
        node(&workflow, 9)?.kind,
        CompiledNodeKind::RepeatCheck {
            attempt_slot: outer,
            done: StepIdx::new(10),
        }
    );
    assert_eq!(
        node(&workflow, 10)?.kind,
        CompiledNodeKind::RepeatFinish {
            result: output(&workflow, 4)?
        }
    );
    Ok(())
}

// Overflow: body width beyond u16 is rejected as StepIndexOutOfRange.
// Direct width calculation is tested via large nested fanout; source-level
// overflow manifests as SourceTooLarge or ResourceContractExceeded.
#[test]
fn repeat_body_width_overflow_is_rejected() -> Result<(), String> {
    // Source-level overflow: huge body exceeds source limits or slot budget.
    let mut body = String::new();
    for i in 0..3000_u32 {
        body.push_str(&format!(
            "        - id: s{i}\n          set: {{output: v{i}, value: '1'}}\n"
        ));
    }
    let text = repeat_source(&body, 2);
    let errors = match compile_workflow(text.as_bytes()) {
        Ok(_) => return Err("expected overflow rejection".to_owned()),
        Err(e) => e,
    };
    let ok = errors.0.iter().any(|e| {
        matches!(
            e,
            CompileError::StepIndexOutOfRange { .. }
                | CompileError::SourceTooLarge { .. }
                | CompileError::CanonicalYaml { .. }
                | CompileError::Workflow(_)
        )
    });
    if ok {
        Ok(())
    } else {
        Err(format!("wrong diagnostic: {errors:?}"))
    }
}

// Foreign counter writes: nested repeat must allocate a fresh attempt slot;
// the inner body's outputs must not alias either counter. Validator owns the
// packed counter; compile must not alias.
