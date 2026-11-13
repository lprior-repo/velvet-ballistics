//! Scoped Choose/Together: branch bodies through shared dispatch, join/skip
//! outputs, boolean conditions, lexical isolation, deterministic slots.
//! Bead: vb-eirmk.4.2. All tests drive public `compile_workflow`; no
//! constructor-only coverage.
#![forbid(unsafe_code)]

use crate::{CompileError, compile_workflow};
use vb_core::workflow::NodeContinuation;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx};

const HEADER: &str = "version: velvet-ballistics/v1\nname: choose_scoped\nwhen: {manual: {}}\n";

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

fn top_choose(branches: &str, otherwise: Option<&str>) -> String {
    let tail = otherwise.map_or(String::new(), |label| format!("      otherwise: {label}\n"));
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: flag\n    set: {{output: go, value: 'true'}}\n  - id: pick\n    choose:\n      branches:\n{branches}{tail}  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

#[test]
fn choose_multi_branches_layout_and_skip() -> Result<(), String> {
    let branches = concat!(
        "        - when: '$go'\n          steps:\n",
        "            - id: a\n              set: {output: xa, value: '1'}\n",
        "        - when: 'true'\n          steps:\n",
        "            - id: b\n              set: {output: yb, value: '2'}\n",
    );
    let wf = compile(&top_choose(branches, Some("done")))?;
    assert_eq!(wf.to_parts().nodes.len(), 6);
    let CompiledNodeKind::Choose { branches, otherwise } = &node(&wf, 2)?.kind else {
        return Err("node 2 not Choose".to_owned());
    };
    let branches = branches.as_ref();
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0].target, StepIdx::new(3));
    assert_eq!(branches[1].target, StepIdx::new(4));
    assert_eq!(otherwise.step(), Some(StepIdx::new(5)));
    assert_eq!(node(&wf, 3)?.routing.next, NodeContinuation::Step(StepIdx::new(5)));
    Ok(())
}

#[test]
fn choose_empty_branch_skips_to_common_next() -> Result<(), String> {
    let branches = "        - when: 'true'\n          steps: []\n";
    let wf = compile(&top_choose(branches, Some("done")))?;
    let CompiledNodeKind::Choose { otherwise, .. } = &node(&wf, 2)?.kind else {
        return Err("node 2 not Choose".to_owned());
    };
    assert_eq!(otherwise.step(), Some(StepIdx::new(3)));
    Ok(())
}

#[test]
fn choose_empty_table_without_otherwise_rejected() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: pick\n    choose:\n      branches: []\n  - id: done\n    finish: {{result: '1'}}\n"
    );
    rejects(&text, |e| matches!(e, CompileError::StepFieldShape { field: "choose.branches", .. }))
}

#[test]
fn choose_nonboolean_condition_rejected() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: num\n    set: {{output: num_val, value: '1'}}\n  - id: pick\n    choose:\n      branches:\n        - when: '$num_val'\n          steps:\n            - id: a\n              set: {{output: xa, value: '1'}}\n      otherwise: done\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(&text, |e| matches!(e, CompileError::TypeMismatch { field: "choose.branches[].when", .. }))
}

fn top_together(branches: &str) -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: fan\n    together:\n      branches:\n{branches}  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

#[test]
fn together_multi_branches_layout_and_join() -> Result<(), String> {
    let branches = concat!(
        "        - label: a\n          steps:\n",
        "            - id: only_a\n              set: {output: xa, value: '1'}\n",
        "        - label: b\n          steps:\n",
        "            - id: only_b\n              set: {output: yb, value: '2'}\n",
    );
    let wf = compile(&top_together(branches))?;
    assert_eq!(wf.to_parts().nodes.len(), 10);
    let CompiledNodeKind::TogetherStart { branches, join } = &node(&wf, 1)?.kind else {
        return Err("node 1 not TogetherStart".to_owned());
    };
    assert_eq!(branches.as_ref(), &[StepIdx::new(2), StepIdx::new(5)]);
    assert_eq!(*join, StepIdx::new(8));
    let CompiledNodeKind::TogetherJoin { branch_count, .. } = &node(&wf, 8)?.kind else {
        return Err("node 8 not TogetherJoin".to_owned());
    };
    assert_eq!(branch_count.get(), 2);
    assert_ne!(output(&wf, 1)?, output(&wf, 8)?);
    Ok(())
}

#[test]
fn together_empty_branch_writes_null_and_joins() -> Result<(), String> {
    let branches = concat!(
        "        - label: a\n          steps: []\n",
        "        - label: b\n          steps:\n",
        "            - id: only_b\n              set: {output: yb, value: '2'}\n",
    );
    let wf = compile(&top_together(branches))?;
    let CompiledNodeKind::TogetherStart { branches, join } = &node(&wf, 1)?.kind else {
        return Err("node 1 not TogetherStart".to_owned());
    };
    assert_eq!(branches.as_ref(), &[StepIdx::new(2), StepIdx::new(4)]);
    assert_eq!(*join, StepIdx::new(7));
    assert!(matches!(&node(&wf, 3)?.kind, CompiledNodeKind::SetConst { .. } | CompiledNodeKind::Copy { .. }));
    Ok(())
}

#[test]
fn together_zero_branches_rejected() -> Result<(), String> {
    let text = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: fan\n    together:\n      branches: []\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(&text, |e| matches!(e, CompileError::StepFieldShape { field: "together.branches", .. }))
}

#[test]
fn branch_output_does_not_escape_choose_or_together() -> Result<(), String> {
    let choose_escape = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: pick\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: a\n              set: {{output: branch_out, value: '1'}}\n      otherwise: done\n  - id: after\n    set: {{output: after_val, value: '$branch_out'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(&choose_escape, |e| matches!(e, CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "branch_out"))?;
    let together_escape = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: fan\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: only_a\n              set: {{output: branch_out, value: '1'}}\n  - id: after\n    set: {{output: after_val, value: '$branch_out'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(&together_escape, |e| matches!(e, CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "branch_out"))
}

#[test]
fn nested_together_rejected_at_any_depth() -> Result<(), String> {
    let direct = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: inner\n              together:\n                branches:\n                  - label: x\n                    steps:\n                      - id: leaf\n                        set: {{output: yx, value: '1'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(&direct, |e| matches!(e, CompileError::Workflow(vb_core::WorkflowError::NestedTogether { .. })))?;
    let via_choose = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: route\n              choose:\n                branches:\n                  - when: 'true'\n                    steps:\n                      - id: inner\n                        together:\n                          branches:\n                            - label: x\n                              steps:\n                                - id: leaf\n                                  set: {{output: yx, value: '1'}}\n                otherwise: after_route\n            - id: after_route\n              set: {{output: after_val, value: '1'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(&via_choose, |e| matches!(e, CompileError::Workflow(vb_core::WorkflowError::NestedTogether { .. })))?;
    let via_repeat = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: r\n              repeat:\n                max_attempts: 2\n                steps:\n                  - id: inner\n                    together:\n                      branches:\n                        - label: x\n                          steps:\n                            - id: leaf\n                              set: {{output: yx, value: '1'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    rejects(&via_repeat, |e| matches!(e, CompileError::Workflow(vb_core::WorkflowError::NestedTogether { .. })))
}

#[test]
fn together_in_choose_and_choose_in_together_allowed() -> Result<(), String> {
    let together_in_choose = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: pick\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: fan\n              together:\n                branches:\n                  - label: a\n                    steps:\n                      - id: only_a\n                        set: {{output: xa, value: '1'}}\n      otherwise: done\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let wf = compile(&together_in_choose)?;
    assert!(matches!(&node(&wf, 1)?.kind, CompiledNodeKind::Choose { .. }));
    let choose_in_together = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: fan\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: pick\n              choose:\n                branches:\n                  - when: 'true'\n                    steps:\n                      - id: inner\n                        set: {{output: xa, value: '1'}}\n                otherwise: after_pick\n            - id: after_pick\n              set: {{output: after_val, value: '2'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let wf2 = compile(&choose_in_together)?;
    assert!(wf2.to_parts().nodes.iter().any(|n| matches!(&n.kind, CompiledNodeKind::Choose { .. })));
    Ok(())
}

#[test]
fn suspending_do_in_choose_and_together_compiles() -> Result<(), String> {
    let choose_do = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: pick\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: act\n              do: {{action: '1', input: '0'}}\n      otherwise: done\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let wf = compile(&choose_do)?;
    assert!(matches!(&node(&wf, 2)?.kind, CompiledNodeKind::Do { .. }));
    let together_do = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: fan\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: act_a\n              do: {{action: '1', input: '0'}}\n        - label: b\n          steps:\n            - id: act_b\n              do: {{action: '2', input: '0'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    );
    let wf2 = compile(&together_do)?;
    assert!(matches!(&node(&wf2, 3)?.kind, CompiledNodeKind::Do { .. }));
    Ok(())
}

#[test]
fn deterministic_slots_choose_and_together() -> Result<(), String> {
    let branches = "        - when: 'true'\n          steps:\n            - id: a\n              set: {output: xa, value: '1'}\n";
    let text = top_choose(branches, Some("done"));
    let a = compile(&text)?;
    let b = compile(&text)?;
    assert_eq!(a.to_parts().nodes, b.to_parts().nodes);
    assert_eq!(a.to_parts().slot_count, b.to_parts().slot_count);
    let together_text = top_together("        - label: a\n          steps:\n            - id: only_a\n              set: {output: xa, value: '1'}\n");
    let c = compile(&together_text)?;
    let d = compile(&together_text)?;
    assert_eq!(c.to_parts().nodes, d.to_parts().nodes);
    Ok(())
}
