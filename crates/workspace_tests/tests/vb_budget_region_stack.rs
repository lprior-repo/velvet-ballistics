#![forbid(unsafe_code)]

// Hand-written budget fixtures, not canonical reachability coverage.

use vb_core::budget::WholeWorkflowBudget;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

fn step(n: u16) -> StepIdx {
    StepIdx::new(n)
}
fn slot(n: u16) -> SlotIdx {
    SlotIdx::new(n)
}

fn node(id: u16, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: step(id),
        kind,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
    }
}

fn nop(id: u16, next: u16) -> CompiledNode {
    let mut n = node(id, CompiledNodeKind::Nop);
    n.next = Some(step(next));
    n
}

fn finish(id: u16, result: u16) -> CompiledNode {
    node(
        id,
        CompiledNodeKind::Finish {
            result: slot(result),
        },
    )
}

fn repeat(id: u16, max: u16, body: u16, done: u16) -> CompiledNode {
    node(
        id,
        CompiledNodeKind::RepeatStart {
            max_attempts: max,
            body: step(body),
            done: step(done),
        },
    )
}

fn together(id: u16, branches: Vec<u16>, join: u16) -> CompiledNode {
    let b: Box<[StepIdx]> = branches.into_iter().map(step).collect();
    node(
        id,
        CompiledNodeKind::TogetherStart {
            branches: b,
            join: step(join),
        },
    )
}

fn compute(nodes: Vec<CompiledNode>) -> Result<WholeWorkflowBudget, String> {
    let entry = step(0);
    WholeWorkflowBudget::compute(&nodes, entry, &ResourceContract::DEFAULT)
        .map_err(|e| e.to_string())
}

#[test]
fn nested_multiplication() -> Result<(), String> {
    let nodes = vec![
        repeat(0, 2, 1, 5),
        repeat(1, 3, 2, 4),
        nop(2, 3),
        nop(3, 4),
        nop(4, 5),
        finish(5, 0),
    ];
    let b = compute(nodes)?;
    if b.max_total_steps != 18 {
        return Err(format!("expected 18 steps, got {}", b.max_total_steps));
    }
    if b.max_nesting_depth != 2 {
        return Err(format!("expected depth 2, got {}", b.max_nesting_depth));
    }
    Ok(())
}

#[test]
fn same_body_distinct_done() -> Result<(), String> {
    let nodes = vec![
        repeat(0, 1, 1, 10),
        together(1, vec![2, 3], 9),
        repeat(2, 1, 4, 7),
        repeat(3, 1, 4, 8),
        nop(4, 5),
        nop(5, 6),
        nop(6, 7),
        nop(7, 8),
        nop(8, 9),
        nop(9, 10),
        finish(10, 0),
    ];
    let b = compute(nodes)?;
    if b.max_total_steps != 15 {
        return Err(format!("expected 15 steps, got {}", b.max_total_steps));
    }
    Ok(())
}

#[test]
fn deep_nested_region_stack() -> Result<(), String> {
    let depths = [1_u16, 32_u16, 1024_u16];
    for &depth in &depths {
        let done = match depth.checked_add(1) {
            Some(d) => d,
            None => return Err("depth overflow".to_string()),
        };
        let mut nodes = Vec::new();
        for i in 0..depth {
            let body = match i.checked_add(1) {
                Some(b) => b,
                None => return Err("body index overflow".to_string()),
            };
            nodes.push(repeat(i, 1, body, done));
        }
        nodes.push(nop(depth, done));
        nodes.push(finish(done, 0));
        let b = compute(nodes)?;
        let expect_steps = match u64::from(depth).checked_add(2) {
            Some(s) => s,
            None => return Err("expect overflow".to_string()),
        };
        if b.max_total_steps != expect_steps {
            return Err(format!(
                "depth {}: expected {} steps, got {}",
                depth, expect_steps, b.max_total_steps
            ));
        }
        if b.max_nesting_depth != depth {
            return Err(format!(
                "depth {}: expected depth {}, got {}",
                depth, depth, b.max_nesting_depth
            ));
        }
    }
    Ok(())
}
