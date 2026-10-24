#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx, WorkflowParts};

const BASELINE: &str = r#"version: velvet-ballistics/v1
name: guarded_layout
when: { manual: {} }
steps:
  - id: seed
    set: { output: payload, value: "42" }
  - id: g1
    if: "true"
    do: { action: "1", input: "$seed" }
  - id: g2
    if: "false"
    do: { action: "2", input: "$seed" }
  - id: done
    finish: { result: "$seed" }
"#;

fn compile(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

fn check_guard(
    parts: &WorkflowParts,
    guard_index: usize,
    do_index: usize,
    target: u16,
    next: u16,
    slot: u16,
) -> Result<(), String> {
    let guard = parts.nodes.get(guard_index).ok_or("missing guard node")?;
    let do_node = parts.nodes.get(do_index).ok_or("missing do node")?;

    if usize::from(guard.id.get()) != guard_index {
        return Err(format!(
            "guard id {} != expected index {}",
            guard.id.get(),
            guard_index
        ));
    }
    if usize::from(do_node.id.get()) != do_index {
        return Err(format!(
            "do node id {} != expected index {}",
            do_node.id.get(),
            do_index
        ));
    }

    match &guard.kind {
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            if branches.len() != 1 {
                return Err(format!("guard has {} branches, expected 1", branches.len()));
            }
            let branch = branches.get(0).ok_or("missing branch")?;
            if branch.target != StepIdx::new(target) {
                return Err(format!(
                    "branch target {:?} != expected {}",
                    branch.target, target
                ));
            }
            if otherwise != &Some(StepIdx::new(next)) {
                return Err(format!(
                    "otherwise {:?} != expected Some({})",
                    otherwise, next
                ));
            }
        }
        other => return Err(format!("guard node is not Choose: {:?}", other)),
    }

    if guard.output.is_some() {
        return Err("guard should have no output slot".to_string());
    }

    match &do_node.kind {
        CompiledNodeKind::Do { .. } => {}
        other => return Err(format!("do node is not Do: {:?}", other)),
    }

    if do_node.output != Some(SlotIdx::new(slot)) {
        return Err(format!(
            "do output {:?} != expected Some({})",
            do_node.output, slot
        ));
    }
    if do_node.next != Some(StepIdx::new(next)) {
        return Err(format!(
            "do next {:?} != expected Some({})",
            do_node.next, next
        ));
    }

    Ok(())
}

#[test]
fn baseline_guarded_layout() -> Result<(), String> {
    let wf = compile(BASELINE)?;
    let parts = wf.to_parts();

    if parts.nodes.len() != 6 {
        return Err(format!("expected 6 nodes, got {}", parts.nodes.len()));
    }

    check_guard(&parts, 1, 2, 2, 3, 1)?;
    check_guard(&parts, 3, 4, 4, 5, 2)?;

    let name1 = parts.step_names.get(1).ok_or("missing step name 1")?;
    if name1.as_ref() != "g1" {
        return Err(format!("step name 1 is {}, expected g1", name1));
    }
    let name2 = parts.step_names.get(2).ok_or("missing step name 2")?;
    if name2.as_ref() != "g1" {
        return Err(format!("step name 2 is {}, expected g1", name2));
    }
    let name3 = parts.step_names.get(3).ok_or("missing step name 3")?;
    if name3.as_ref() != "g2" {
        return Err(format!("step name 3 is {}, expected g2", name3));
    }
    let name4 = parts.step_names.get(4).ok_or("missing step name 4")?;
    if name4.as_ref() != "g2" {
        return Err(format!("step name 4 is {}, expected g2", name4));
    }

    Ok(())
}

#[test]
fn no_guards_flat_do() -> Result<(), String> {
    let source = BASELINE
        .replace("    if: \"true\"\n", "")
        .replace("    if: \"false\"\n", "");
    let wf = compile(&source)?;
    let parts = wf.to_parts();

    if parts.nodes.len() != 4 {
        return Err(format!("expected 4 nodes, got {}", parts.nodes.len()));
    }

    for (i, node) in parts.nodes.iter().enumerate() {
        if matches!(&node.kind, CompiledNodeKind::Choose { .. }) {
            return Err(format!("unexpected Choose at node {}", i));
        }
    }

    let do1 = parts.nodes.get(1).ok_or("missing do1")?;
    if !matches!(&do1.kind, CompiledNodeKind::Do { .. }) {
        return Err("do1 should be Do kind".to_string());
    }
    if do1.output != Some(SlotIdx::new(1)) {
        return Err(format!("do1 output wrong: {:?}", do1.output));
    }
    let do2 = parts.nodes.get(2).ok_or("missing do2")?;
    if !matches!(&do2.kind, CompiledNodeKind::Do { .. }) {
        return Err("do2 should be Do kind".to_string());
    }
    if do2.output != Some(SlotIdx::new(2)) {
        return Err(format!("do2 output wrong: {:?}", do2.output));
    }

    Ok(())
}

#[test]
fn vars_and_input_allocation() -> Result<(), String> {
    let source = BASELINE.replace(
        "steps:\n",
        "inputs: {amount: number}\nvars: {enabled: true}\nsteps:\n",
    );
    let source = source.replace("    if: \"true\"\n", "    if: \"$vars.enabled\"\n");
    let wf = compile(&source)?;
    let parts = wf.to_parts();

    if parts.nodes.len() != 7 {
        return Err(format!("expected 7 nodes, got {}", parts.nodes.len()));
    }

    check_guard(&parts, 2, 3, 3, 4, 1)?;
    check_guard(&parts, 4, 5, 5, 6, 2)?;

    let name2 = parts.step_names.get(2).ok_or("missing step name 2")?;
    if name2.as_ref() != "g1" {
        return Err(format!("step name 2 is {}, expected g1", name2));
    }
    let node2 = parts.nodes.get(2).ok_or("missing node 2")?;
    if !matches!(&node2.kind, CompiledNodeKind::Choose { .. }) {
        return Err("node 2 should be Choose".to_string());
    }

    Ok(())
}

#[test]
fn semantic_identity_differs_on_condition_change() -> Result<(), String> {
    let wf1 = compile(BASELINE)?;
    let parts1 = wf1.to_parts();

    let source2 = BASELINE.replace("    if: \"true\"\n", "    if: \"false\"\n");
    let wf2 = compile(&source2)?;
    let parts2 = wf2.to_parts();

    let id1 = parts1
        .semantic_source_identity
        .as_ref()
        .ok_or("missing identity 1")?;
    let id2 = parts2
        .semantic_source_identity
        .as_ref()
        .ok_or("missing identity 2")?;
    if id1 == id2 {
        return Err("semantic identities should differ".to_string());
    }

    if parts1.digest == parts2.digest {
        return Err("digests should differ".to_string());
    }

    Ok(())
}
