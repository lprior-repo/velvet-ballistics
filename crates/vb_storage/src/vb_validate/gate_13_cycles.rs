#![forbid(unsafe_code)]
//! Gate 13: No circular references in slot dependency graph.

#![allow(unreachable_pub)]
#![allow(clippy::collapsible_if)]

use crate::vb_validate::{ValidationError, ValidationResult};
use vb_core::ids::SlotIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, WorkflowParts};

pub fn validate_gate_13_no_slot_cycles(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    if slot_count == 0 {
        return Ok(());
    }

    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); slot_count];

    for node in parts.nodes.iter() {
        let reads = node_reads(node, &parts.expressions);
        if let Some(output) = node.output {
            let out_usize = output.as_usize();
            if out_usize < slot_count {
                for read_slot in reads {
                    let read_usize = read_slot.as_usize();
                    if read_usize < slot_count && read_usize != out_usize {
                        if let Some(list) = adjacency.get_mut(out_usize) {
                            if !list.contains(&read_usize) {
                                list.push(read_usize);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut visited: Vec<u8> = vec![0; slot_count];
    for slot in 0..slot_count {
        if visited.get(slot) == Some(&0) {
            detect_cycle_dfs(slot, &adjacency, &mut visited)?;
        }
    }
    Ok(())
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
            if let Some(ep) = expressions.get(expr.as_usize()) {
                for op in ep.ops.iter() {
                    if let ExprOp::LoadSlot(s) = op {
                        reads.push(*s);
                    }
                }
            }
        }
        CompiledNodeKind::BuildObject { fields } => {
            for (_, s) in fields.iter() {
                reads.push(*s);
            }
        }
        CompiledNodeKind::BuildList { items } => {
            for s in items.iter() {
                reads.push(*s);
            }
        }
        CompiledNodeKind::Do { input, .. } => {
            reads.push(*input);
        }
        CompiledNodeKind::Choose { branches, .. } => {
            for b in branches.iter() {
                if let Some(ep) = expressions.get(b.condition.as_usize()) {
                    for op in ep.ops.iter() {
                        if let ExprOp::LoadSlot(s) = op {
                            reads.push(*s);
                        }
                    }
                }
            }
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            for b in branches.iter() {
                reads.push(b.condition);
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
            if let Some(t) = timeout_slot {
                reads.push(*t);
            }
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            reads.push(*prompt);
            if let Some(t) = timeout_slot {
                reads.push(*t);
            }
        }
        CompiledNodeKind::AskResume { answer } => {
            reads.push(*answer);
        }
        CompiledNodeKind::RetryCheck { policy_slot, .. } => {
            reads.push(*policy_slot);
        }
        CompiledNodeKind::ErrorHandler { .. } | CompiledNodeKind::Jump { .. } => {}
        CompiledNodeKind::Finish { result } => {
            reads.push(*result);
        }
        _ => {}
    }
    reads
}

#[cfg(test)]
#[path = "gate_13_cycles/tests.rs"]
mod tests;
