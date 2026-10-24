#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx, WorkflowParts};

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

fn build_guard_source(count: u16) -> String {
    let mut source = String::from(
        "version: velvet-ballistics/v1\nname: guard_table\nwhen: { manual: {} }\nsteps:\n",
    );
    source.push_str("  - id: seed\n    set: { output: payload, value: \"42\" }\n");
    for i in 1..=count {
        source.push_str(&format!(
            "  - id: g{}\n    if: \"true\"\n    do: {{ action: \"1\", input: \"$seed\" }}\n",
            i
        ));
    }
    source.push_str("  - id: done\n    finish: { result: \"$seed\" }\n");
    source
}

#[test]
fn bounded_guard_table() -> Result<(), String> {
    let table = [(1022_u16, 2046_usize, 1023_u16), (1023, 2048, 1024)];
    for (count, expected_nodes, expected_slots) in table {
        let source = build_guard_source(count);
        let wf = compile(&source)?;
        let parts = wf.to_parts();

        let budget = vb_core::budget::WholeWorkflowBudget::compute(
            &parts.nodes,
            parts.entry,
            &parts.resource_contract,
        )
        .map_err(|e| format!("budget compute failed: {e:?}"))?;
        let expected_total: u64 = u64::try_from(expected_nodes)
            .map_err(|e| format!("expected_nodes conversion failed: {e:?}"))?;
        if budget.max_total_steps != expected_total {
            return Err(format!(
                "count {}: budget max_total_steps {} != expected {}",
                count, budget.max_total_steps, expected_total
            ));
        }

        if parts.nodes.len() != expected_nodes {
            return Err(format!(
                "count {}: expected {} nodes, got {}",
                count,
                expected_nodes,
                parts.nodes.len()
            ));
        }
        if parts.slot_count != expected_slots {
            return Err(format!(
                "count {}: expected {} slots, got {}",
                count, expected_slots, parts.slot_count
            ));
        }

        check_guard(&parts, 1, 2, 2, 3, 1)?;

        if count == 1023 {
            check_guard(&parts, 2045, 2046, 2046, 2047, 1023)?;
        }
    }

    let source = build_guard_source(1024);
    let result = compile_workflow(source.as_bytes());
    match result {
        Err(errors) => {
            if errors.0
                != vec![vb_compile::CompileError::Workflow(
                    vb_core::WorkflowError::ResourceContractExceeded {
                        resource: "max_slots",
                    },
                )]
            {
                return Err(format!("expected max_slots error, got: {:?}", errors.0));
            }
        }
        Ok(_) => return Err("expected compile error for 1024 guards".to_string()),
    }

    Ok(())
}

#[test]
fn mixed_guards_and_repeat() -> Result<(), String> {
    let source = r#"version: velvet-ballistics/v1
name: mixed_layout
when: { manual: {} }
steps:
  - id: seed
    set: { output: payload, value: "42" }
  - id: g1
    if: "true"
    do: { action: "1", input: "$seed" }
  - id: bounded_repeat
    repeat:
      max_attempts: 2
      steps:
        - id: a_step
          set: { output: a, value: "7" }
        - id: b_step
          set: { output: b, value: "11" }
  - id: g2
    if: "false"
    do: { action: "2", input: "$seed" }
  - id: done
    finish: { result: "$seed" }
"#;

    let wf = compile(source)?;
    let parts = wf.to_parts();

    if parts.nodes.len() != 11 {
        return Err(format!("expected 11 nodes, got {}", parts.nodes.len()));
    }

    check_guard(&parts, 1, 2, 2, 3, 1)?;
    check_guard(&parts, 8, 9, 9, 10, 3)?;

    let repeat_start = parts.nodes.get(3).ok_or("missing repeat start")?;
    match &repeat_start.kind {
        CompiledNodeKind::RepeatStart {
            body,
            done,
            max_attempts,
        } => {
            if *body != StepIdx::new(4) {
                return Err(format!("repeat body {:?} != expected 4", body));
            }
            if *done != StepIdx::new(7) {
                return Err(format!("repeat done {:?} != expected 7", done));
            }
            if *max_attempts != 2 {
                return Err(format!(
                    "repeat max_attempts {} != expected 2",
                    max_attempts
                ));
            }
        }
        other => return Err(format!("node 3 is not RepeatStart: {:?}", other)),
    }

    let repeat_check = parts.nodes.get(6).ok_or("missing repeat check")?;
    match &repeat_check.kind {
        CompiledNodeKind::RepeatCheck { done, .. } => {
            if *done != StepIdx::new(7) {
                return Err(format!("repeat check done {:?} != expected 7", done));
            }
        }
        other => return Err(format!("node 6 is not RepeatCheck: {:?}", other)),
    }

    let repeat_finish = parts.nodes.get(7).ok_or("missing repeat finish")?;
    if repeat_finish.next != Some(StepIdx::new(8)) {
        return Err(format!(
            "repeat finish next {:?} != expected Some(8)",
            repeat_finish.next
        ));
    }

    let name1 = parts.step_names.get(1).ok_or("missing step name 1")?;
    if name1.as_ref() != "g1" {
        return Err(format!("step name 1 is {}, expected g1", name1));
    }
    let name2 = parts.step_names.get(2).ok_or("missing step name 2")?;
    if name2.as_ref() != "g1" {
        return Err(format!("step name 2 is {}, expected g1", name2));
    }
    let name8 = parts.step_names.get(8).ok_or("missing step name 8")?;
    if name8.as_ref() != "g2" {
        return Err(format!("step name 8 is {}, expected g2", name8));
    }
    let name9 = parts.step_names.get(9).ok_or("missing step name 9")?;
    if name9.as_ref() != "g2" {
        return Err(format!("step name 9 is {}, expected g2", name9));
    }

    Ok(())
}
