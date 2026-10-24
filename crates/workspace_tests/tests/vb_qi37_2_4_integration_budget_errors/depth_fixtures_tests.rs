//! Raw budget fixtures: sibling spans have depth one; contained spans have depth D.
//! These assertions do not claim runtime activation or workflow admission.

use vb_core::{
    budget::{BudgetError, WholeWorkflowBudget},
    ids::{SlotIdx, StepIdx},
    workflow::{CompiledNode, CompiledNodeKind},
};

fn u16_add(a: u16, b: u16) -> Result<u16, Box<dyn std::error::Error>> {
    a.checked_add(b).ok_or("u16 add overflow".into())
}

fn u16_mul(a: u16, b: u16) -> Result<u16, Box<dyn std::error::Error>> {
    a.checked_mul(b).ok_or("u16 mul overflow".into())
}

fn u16_sub(a: u16, b: u16) -> Result<u16, Box<dyn std::error::Error>> {
    a.checked_sub(b).ok_or("u16 sub overflow".into())
}

fn node(id: u16, kind: CompiledNodeKind, next: Option<u16>, output: Option<u16>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        kind,
        next: next.map(StepIdx::new),
        output: output.map(SlotIdx::new),
        on_error: None,
        error_slot: None,
    }
}

fn validate(nodes: &[CompiledNode], expected: u16) -> Result<(), Box<dyn std::error::Error>> {
    let count = u16::try_from(nodes.len()).map_err(|_| "count overflow")?;
    if count != expected {
        return Err(format!("expected {} nodes, got {}", expected, count).into());
    }
    for (i, n) in nodes.iter().enumerate() {
        let idx = u16::try_from(i).map_err(|_| "index overflow")?;
        if n.id.get() != idx {
            return Err(format!("node {} id mismatch", idx).into());
        }
    }
    Ok(())
}

pub(super) fn assert_depth_case(
    depth: u16,
    reject_limit: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    if !(1..=10).contains(&depth) || !(1..depth).contains(&reject_limit) {
        return Err("invalid depth fixture bounds".into());
    }
    let (nested, nested_count) = build_nested_graph(depth)?;
    let (sibling, sibling_count) = build_sibling_graph(depth)?;
    let depth_minus_1 = u16_sub(depth, 1)?;

    let slots = u16_add(u16_mul(depth, 3)?, 1)?;
    let nested_budget = WholeWorkflowBudget::compute(
        &nested,
        StepIdx::new(0),
        &super::test_contract(nested_count, slots),
    )
    .map_err(|e| format!("nested budget: {:?}", e))?;

    let sibling_budget = WholeWorkflowBudget::compute(
        &sibling,
        StepIdx::new(0),
        &super::test_contract(sibling_count, slots),
    )
    .map_err(|e| format!("sibling budget: {:?}", e))?;

    assert_eq!(sibling_budget.max_nesting_depth, 1);
    assert_eq!(nested_budget.max_nesting_depth, depth);

    super::tight_policy(1_000_000, 65_535, 64, reject_limit).validate(&sibling_budget)?;
    super::tight_policy(1_000_000, 65_535, 64, 1).validate(&sibling_budget)?;

    super::tight_policy(1_000_000, 65_535, 64, depth).validate(&nested_budget)?;

    let result = super::tight_policy(1_000_000, 65_535, 64, depth_minus_1).validate(&nested_budget);
    match result {
        Err(BudgetError::NestingDepthExceeded { actual, limit }) => {
            assert_eq!(actual, depth);
            assert_eq!(limit, depth_minus_1);
        }
        other => return Err(format!("expected depth exceeded at depth-1, got {:?}", other).into()),
    }

    let result = super::tight_policy(1_000_000, 65_535, 64, reject_limit).validate(&nested_budget);
    match result {
        Err(BudgetError::NestingDepthExceeded { actual, limit }) => {
            assert_eq!(actual, depth);
            assert_eq!(limit, reject_limit);
        }
        other => {
            return Err(format!("expected depth exceeded at reject_limit, got {:?}", other).into());
        }
    }

    Ok(())
}
fn build_nested_graph(depth: u16) -> Result<(Vec<CompiledNode>, u16), Box<dyn std::error::Error>> {
    let nested_count = u16_add(u16_mul(depth, 2)?, 2)?;
    let mut nested = Vec::new();
    nested
        .try_reserve(usize::from(nested_count))
        .map_err(|_| "reserve")?;

    for i in 0..depth {
        let body = u16_add(i, 1)?;
        let done = u16_sub(u16_mul(depth, 2)?, i)?;
        let iter_slot = u16_add(1, u16_mul(3, i)?)?;
        let item_slot = u16_add(2, u16_mul(3, i)?)?;
        nested.push(node(
            i,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(item_slot),
                limit: 2,
                body: StepIdx::new(body),
                done: StepIdx::new(done),
            },
            None,
            Some(iter_slot),
        ));
    }

    nested.push(node(
        depth,
        CompiledNodeKind::Nop,
        Some(u16_add(depth, 1)?),
        None,
    ));

    let start_join = u16_add(depth, 1)?;
    let max_join = u16_mul(depth, 2)?;
    for j in start_join..=max_join {
        let level = u16_sub(max_join, j)?;
        let join_slot = u16_add(3, u16_mul(3, level)?)?;
        nested.push(node(
            j,
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(join_slot),
            },
            Some(u16_add(j, 1)?),
            Some(join_slot),
        ));
    }

    nested.push(node(
        u16_add(max_join, 1)?,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    ));

    validate(&nested, nested_count)?;
    Ok((nested, nested_count))
}

fn build_sibling_graph(depth: u16) -> Result<(Vec<CompiledNode>, u16), Box<dyn std::error::Error>> {
    let sibling_count = u16_add(u16_mul(depth, 3)?, 1)?;
    let mut sibling = Vec::new();
    sibling
        .try_reserve(usize::from(sibling_count))
        .map_err(|_| "reserve")?;

    for i in 0..depth {
        let start_id = u16_mul(3, i)?;
        let body_id = u16_add(start_id, 1)?;
        let join_id = u16_add(start_id, 2)?;
        let iter_slot = u16_add(1, u16_mul(3, i)?)?;
        let item_slot = u16_add(2, u16_mul(3, i)?)?;
        let join_slot = u16_add(3, u16_mul(3, i)?)?;

        sibling.push(node(
            start_id,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(item_slot),
                limit: 2,
                body: StepIdx::new(body_id),
                done: StepIdx::new(join_id),
            },
            None,
            Some(iter_slot),
        ));

        sibling.push(node(body_id, CompiledNodeKind::Nop, Some(join_id), None));

        let next = Some(u16_add(join_id, 1)?);
        sibling.push(node(
            join_id,
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(join_slot),
            },
            next,
            Some(join_slot),
        ));
    }

    sibling.push(node(
        u16_mul(3, depth)?,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    ));

    validate(&sibling, sibling_count)?;
    Ok((sibling, sibling_count))
}
