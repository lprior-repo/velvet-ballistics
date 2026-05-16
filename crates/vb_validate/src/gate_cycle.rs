//! Gate 13: No circular references in slot dependency graph
//!
//! Validates that the slot dependency graph has no cycles. A slot that is
//! written from another slot must not form a cycle. This catches cases where
//! slot A depends on slot B which depends on slot A (directly or transitively).

use crate::{ValidationError, ValidationResult};

pub use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, WorkflowParts,
};
pub use vb_core::ids::SlotIdx;

/// Validates that the slot dependency graph has no cycles.
///
/// Gate 13 (capabilities): a slot that is written from another slot must not
/// form a cycle. This catches cases where slot A depends on slot B which
/// depends on slot A (directly or transitively).
///
/// The analysis is per-node: we extract which slots each node reads and which
/// it writes, then build a dependency graph. A cycle means a slot can never
/// receive a value because it depends on itself.
pub fn validate_gate_13_no_slot_cycles(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    if slot_count == 0 {
        return Ok(());
    }

    // Build adjacency: for each output slot, which slots does it depend on?
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

    // Detect cycles via DFS with three-color marking.
    let mut visited: Vec<u8> = vec![0; slot_count]; // 0 = white, 1 = gray, 2 = black
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
        *state = 1; // gray
    }

    let neighbors = adjacency.get(slot).map_or(&[][..], |v| v.as_slice());
    for &neighbor in neighbors {
        let color = visited.get(neighbor).copied().ok_or(
            ValidationError::SlotDependencyCycle {
                slot,
                chain: format!("slot {slot} -> slot {neighbor}"),
            },
        )?;
        if color == 1 {
            // Gray = cycle found.
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
        *state = 2; // black
    }
    Ok(())
}

/// Extracts all slot indices read by a node.
pub fn node_reads(node: &CompiledNode, expressions: &[ExprProgram]) -> Vec<SlotIdx> {
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
        CompiledNodeKind::Do { input, .. } => {
            reads.push(*input);
        }
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
            input,
            accumulator,
            ..
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
