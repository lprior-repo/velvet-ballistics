#![forbid(unsafe_code)]
//! Gate 13: Slot dependency cycle detection

use crate::{ValidationError, ValidationResult};
use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};

pub fn validate_gate_13_no_slot_cycles(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    proptest::prop_assume!(slot_count  != 0 );

    let adjacency = build_slot_adjacency(parts, slot_count);
    let mut visited: Vec<u8> = vec![0; slot_count];
    for slot in 0..slot_count {
        if visited.get(slot) == Some(&0) {
            detect_cycle_dfs(slot, &adjacency, &mut visited)?;
        }
    }
    Ok(())
}

fn build_slot_adjacency(parts: &WorkflowParts, slot_count: usize) -> Vec<Vec<usize>> {
    let mut adjacency = vec![Vec::new(); slot_count];
    parts.nodes.iter().for_each(|node| {
        append_node_edges(&mut adjacency, node, &parts.expressions, slot_count);
    });
    adjacency
}

fn append_node_edges(
    adjacency: &mut [Vec<usize>],
    node: &CompiledNode,
    expressions: &[ExprProgram],
    slot_count: usize,
) {
    if let Some(output) = node.output.filter(|output| output.as_usize() < slot_count) {
        node_reads(node, expressions)
            .into_iter()
            .for_each(|read_slot| {
                add_unique_edge(
                    adjacency,
                    output.as_usize(),
                    read_slot.as_usize(),
                    slot_count,
                );
            });
    }
}

fn add_unique_edge(
    adjacency: &mut [Vec<usize>],
    output: usize,
    read_slot: usize,
    slot_count: usize,
) {
    match adjacency.get_mut(output) {
        Some(list)
            if read_slot < slot_count && read_slot != output && !list.contains(&read_slot) =>
        {
            list.push(read_slot);
        }
        _ => {}
    }
}

fn detect_cycle_dfs(
    slot: usize,
    adjacency: &[Vec<usize>],
    visited: &mut [u8],
) -> ValidationResult<()> {
    if let Some(state) = visited.get_mut(slot) {
        *state = 1;
    }

    let neighbors = adjacency.get(slot).map_or(&[][..], |v| v.as_slice());
    for &neighbor in neighbors {
        let color = visited
            .get(neighbor)
            .copied()
            .ok_or(ValidationError::SlotDependencyCycle {
                slot,
                chain: format!("slot {slot} -> slot {neighbor}"),
            })?;
        if color == 1 {
            return Err(ValidationError::SlotDependencyCycle {
                slot,
                chain: format!("slot {slot} -> slot {neighbor}"),
            });
        }
        if color == 0 {
            detect_cycle_dfs(neighbor, adjacency, visited)?;
        }
    }

    if let Some(state) = visited.get_mut(slot) {
        *state = 2;
    }
    Ok(())
}

fn node_reads(node: &CompiledNode, expressions: &[ExprProgram]) -> Vec<SlotIdx> {
    let mut reads = Vec::new();
    match &node.kind {
        CompiledNodeKind::Nop | CompiledNodeKind::SetConst { .. } => {}
        CompiledNodeKind::Copy { source } => {
            reads.push(*source);
        }
        CompiledNodeKind::EvalExpr { expr } => {
            if let Some(expr_program) = expressions.get(expr.as_usize()) {
                for op in expr_program.ops.iter() {
                    if let ExprOp::LoadSlot(slot) = op {
                        reads.push(*slot);
                    }
                }
            }
        }
        CompiledNodeKind::BuildObject { fields } => {
            for (_, slot) in fields.iter() {
                reads.push(*slot);
            }
        }
        CompiledNodeKind::BuildList { items } => {
            for slot in items.iter() {
                reads.push(*slot);
            }
        }
        CompiledNodeKind::Do { .. } => {}
        CompiledNodeKind::Choose { branches, .. } => {
            for branch in branches.iter() {
                if let Some(expr_program) = expressions.get(branch.condition.as_usize()) {
                    for op in expr_program.ops.iter() {
                        if let ExprOp::LoadSlot(slot) = op {
                            reads.push(*slot);
                        }
                    }
                }
            }
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            for branch in branches.iter() {
                reads.push(branch.condition);
            }
        }
        CompiledNodeKind::ForEachStart { input, .. } => {
            reads.push(*input);
        }
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
            reads.push(*iterator_slot);
        }
        CompiledNodeKind::ForEachJoin { .. } => {}
        CompiledNodeKind::TogetherStart { .. } => {}
        CompiledNodeKind::TogetherBranch { accumulator, .. } => {
            reads.push(*accumulator);
        }
        CompiledNodeKind::TogetherJoin { accumulator, .. } => {
            reads.push(*accumulator);
        }
        CompiledNodeKind::CollectStart { source, .. } => {
            reads.push(*source);
        }
        CompiledNodeKind::CollectPage { collector_slot, .. } => {
            reads.push(*collector_slot);
        }
        CompiledNodeKind::CollectNext { collector_slot, .. } => {
            reads.push(*collector_slot);
        }
        CompiledNodeKind::CollectFinish { collector_slot } => {
            reads.push(*collector_slot);
        }
        CompiledNodeKind::ReduceStart {
            input, accumulator, ..
        } => {
            reads.push(*input);
            reads.push(*accumulator);
        }
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            ..
        } => {
            reads.push(*iterator_slot);
            reads.push(*accumulator);
        }
        CompiledNodeKind::ReduceFinish { accumulator } => {
            reads.push(*accumulator);
        }
        CompiledNodeKind::RepeatStart { .. } => {}
        CompiledNodeKind::RepeatAttempt { .. } => {}
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => {
            reads.push(*attempt_slot);
        }
        CompiledNodeKind::RepeatFinish { result } => {
            reads.push(*result);
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            reads.push(*deadline_slot);
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            reads.push(*event);
            if let Some(timeout) = timeout_slot {
                reads.push(*timeout);
            }
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            reads.push(*prompt);
            if let Some(timeout) = timeout_slot {
                reads.push(*timeout);
            }
        }
        CompiledNodeKind::AskResume { answer } => {
            reads.push(*answer);
        }
        CompiledNodeKind::RetryCheck { policy_slot, .. } => {
            reads.push(*policy_slot);
        }
        CompiledNodeKind::ErrorHandler { .. } => {}
        CompiledNodeKind::Jump { .. } => {}
        CompiledNodeKind::Finish { result } => {
            reads.push(*result);
        }
    }
    reads
}
