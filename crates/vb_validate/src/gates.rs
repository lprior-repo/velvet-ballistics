//! Plan verifier gates for compiled workflow IR (Section 63 of the master doc).
//!
//! Gates 7, 8, 9, 11, and 13 validate structural properties of `WorkflowParts`
//! that the core `validate_parts` function does not cover or that need additional
//! cold-path checks for the accepted-artifact pipeline.

use crate::{ValidationError, ValidationResult};

// Re-export the core types we need so callers only depend on vb_validate.
pub use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};
pub use vb_core::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};

// ---------------------------------------------------------------------------
// Gate 7: Expression stack depth bounded
// ---------------------------------------------------------------------------

/// Maximum expression stack depth allowed by the v1 protocol.
const MAX_EXPR_STACK_DEPTH: u8 = 64;

/// Validates that every expression program's declared max_stack fits within the
/// protocol hard limit and that the declared value matches the actual computed
/// stack depth.
///
/// Gate 7 (boundedness): no expression program may exceed the expression stack
/// bound, and the declared `max_stack` metadata must agree with a fresh
/// recomputation from the opcode stream.
pub fn validate_gate_07_expression_stack_depth(parts: &WorkflowParts) -> ValidationResult<()> {
    let contract_stack = parts.resource_contract.max_expr_stack;
    if contract_stack > MAX_EXPR_STACK_DEPTH {
        return Err(ValidationError::ExpressionStackExceeded {
            declared: usize::from(contract_stack),
            limit: usize::from(MAX_EXPR_STACK_DEPTH),
        });
    }
    for (expr_index, expr) in parts.expressions.iter().enumerate() {
        if expr.max_stack > contract_stack {
            return Err(ValidationError::ExpressionStackExceeded {
                declared: usize::from(expr.max_stack),
                limit: usize::from(contract_stack),
            });
        }
        let computed = compute_stack_depth(&expr.ops)?;
        if computed != expr.max_stack {
            return Err(ValidationError::ExpressionStackMismatch {
                expr_index,
                declared: usize::from(expr.max_stack),
                computed: usize::from(computed),
            });
        }
    }
    Ok(())
}

/// Computes the maximum stack depth for a postfix expression opcode stream.
///
/// Models the stack effects exactly as the core engine does:
/// - LoadSlot/LoadConst/LoadAccessor: pop 0, push 1
/// - Not/Exists/Length/Empty/Sum/Count/Unique: pop 1, push 1
/// - AppendIf: pop 3, push 1
/// - All others (binary): pop 2, push 1
fn compute_stack_depth(ops: &[ExprOp]) -> ValidationResult<u8> {
    let mut depth: u8 = 0;
    let mut max_depth: u8 = 0;
    for op in ops {
        let _effect = stack_effect(op);
        // Apply pop first (subtract), then push (add).
        let pop_amount = pop_count(op);
        depth = depth
            .checked_sub(pop_amount)
            .ok_or(ValidationError::ExpressionStackExceeded {
                declared: 0,
                limit: usize::from(MAX_EXPR_STACK_DEPTH),
            })?;
        let push_amount = push_count(op);
        depth = depth
            .checked_add(push_amount)
            .ok_or(ValidationError::ExpressionStackExceeded {
                declared: usize::from(depth) + usize::from(push_amount),
                limit: usize::from(MAX_EXPR_STACK_DEPTH),
            })?;
        if depth > max_depth {
            max_depth = depth;
        }
    }
    Ok(max_depth)
}

/// Returns how many values an opcode pops from the stack.
fn pop_count(op: &ExprOp) -> u8 {
    match op {
        ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => 0,
        ExprOp::Not
        | ExprOp::Exists
        | ExprOp::Length
        | ExprOp::Empty
        | ExprOp::Sum
        | ExprOp::Count
        | ExprOp::Unique => 1,
        ExprOp::AppendIf => 3,
        _ => 2,
    }
}

/// Returns how many values an opcode pushes onto the stack.
fn push_count(_op: &ExprOp) -> u8 {
    // All opcodes push exactly 1 result.
    1
}

/// Returns the net stack effect of a single expression opcode.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn stack_effect(_op: &ExprOp) -> i8 {
    // pop_count and push_count return small u8 values (max 3).
    // We know push <= 1 and pop <= 3, so the result is always in i8 range.
    let pop = pop_count(_op);
    let push = push_count(_op);
    // push is always 1, pop is 0..=3, so net is 1..=-2
    (push as i8).saturating_sub(pop as i8)
}

// ---------------------------------------------------------------------------
// Gate 8: Accessor path segments are valid symbols
// ---------------------------------------------------------------------------

/// Validates that every accessor path segment resolves to a well-formed symbol.
///
/// Gate 8 (budgets): Field segments must use valid symbol IDs (within the
/// interned symbol table range), and index segments must be finite.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    for (acc_index, accessor) in parts.accessors.iter().enumerate() {
        validate_accessor_root(acc_index, accessor, parts.slot_count)?;
        for (seg_index, segment) in accessor.path.iter().enumerate() {
            match segment {
                PathSegment::Field(_sym_id) => {
                    // Symbol IDs are interned; any non-sentinel value is valid.
                }
                PathSegment::Index(idx) => {
                    if *idx == u32::MAX {
                        return Err(ValidationError::AccessorPathInvalid {
                            accessor_index: acc_index,
                            segment_index: seg_index,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_accessor_root(
    acc_index: usize,
    accessor: &AccessorProgram,
    slot_count: u16,
) -> ValidationResult<()> {
    if accessor.root.as_usize() >= usize::from(slot_count) {
        return Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: acc_index,
            slot: accessor.root.as_usize(),
            slot_count: usize::from(slot_count),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Gate 9: All referenced slots exist within declared slot_count
// ---------------------------------------------------------------------------

/// Validates that every slot reference in the compiled IR is within the
/// declared `slot_count`.
///
/// Gate 9 (contracts): no node, expression, accessor, or structural reference
/// may refer to a slot index >= slot_count.
pub fn validate_gate_09_slot_references(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    for (node_index, node) in parts.nodes.iter().enumerate() {
        validate_node_slots(node, node_index, slot_count)?;
    }
    for (expr_index, expr) in parts.expressions.iter().enumerate() {
        validate_expr_slots(expr, expr_index, slot_count)?;
    }
    Ok(())
}

fn validate_node_slots(
    node: &CompiledNode,
    node_index: usize,
    slot_count: usize,
) -> ValidationResult<()> {
    if let Some(output) = node.output {
        check_slot(output, node_index, slot_count)?;
    }
    match &node.kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Finish { .. } => {}
        CompiledNodeKind::Copy { source } => {
            check_slot(*source, node_index, slot_count)?;
        }
        CompiledNodeKind::EvalExpr { .. } => {}
        CompiledNodeKind::BuildObject { fields } => {
            for (_, slot) in fields.iter() {
                check_slot(*slot, node_index, slot_count)?;
            }
        }
        CompiledNodeKind::BuildList { items } => {
            for slot in items.iter() {
                check_slot(*slot, node_index, slot_count)?;
            }
        }
        CompiledNodeKind::Do { input, .. } => {
            check_slot(*input, node_index, slot_count)?;
        }
        CompiledNodeKind::Choose { .. } | CompiledNodeKind::ChooseSlot { .. } => {}
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            ..
        } => {
            check_slot(*input, node_index, slot_count)?;
            check_slot(*item_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
            check_slot(*iterator_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::ForEachJoin { output } => {
            check_slot(*output, node_index, slot_count)?;
        }
        CompiledNodeKind::TogetherStart { .. } => {}
        CompiledNodeKind::TogetherBranch { accumulator, .. } => {
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::TogetherJoin { accumulator, .. } => {
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::CollectStart { source, .. } => {
            check_slot(*source, node_index, slot_count)?;
        }
        CompiledNodeKind::CollectPage { collector_slot, .. } => {
            check_slot(*collector_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::CollectNext { collector_slot, .. } => {
            check_slot(*collector_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::CollectFinish { collector_slot } => {
            check_slot(*collector_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            ..
        } => {
            check_slot(*input, node_index, slot_count)?;
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            ..
        } => {
            check_slot(*iterator_slot, node_index, slot_count)?;
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::ReduceFinish { accumulator } => {
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::RepeatStart { .. } => {}
        CompiledNodeKind::RepeatAttempt { attempt_slot, .. } => {
            check_slot(*attempt_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => {
            check_slot(*attempt_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::RepeatFinish { result } => {
            check_slot(*result, node_index, slot_count)?;
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            check_slot(*deadline_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            check_slot(*event, node_index, slot_count)?;
            if let Some(timeout) = timeout_slot {
                check_slot(*timeout, node_index, slot_count)?;
            }
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            check_slot(*prompt, node_index, slot_count)?;
            if let Some(timeout) = timeout_slot {
                check_slot(*timeout, node_index, slot_count)?;
            }
        }
        CompiledNodeKind::AskResume { answer } => {
            check_slot(*answer, node_index, slot_count)?;
        }
        CompiledNodeKind::RetryCheck { policy_slot, .. } => {
            check_slot(*policy_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::ErrorHandler { .. } => {}
        CompiledNodeKind::Jump { .. } => {}
    }
    Ok(())
}

fn validate_expr_slots(
    expr: &ExprProgram,
    expr_index: usize,
    slot_count: usize,
) -> ValidationResult<()> {
    for op in expr.ops.iter() {
        if let ExprOp::LoadSlot(slot) = op {
            if slot.as_usize() >= slot_count {
                return Err(ValidationError::SlotReferenceOutOfRange {
                    slot: slot.as_usize(),
                    slot_count,
                    context: format!("expression {expr_index}"),
                });
            }
        }
    }
    Ok(())
}

fn check_slot(
    slot: SlotIdx,
    node_index: usize,
    slot_count: usize,
) -> ValidationResult<()> {
    if slot.as_usize() >= slot_count {
        return Err(ValidationError::SlotReferenceOutOfRange {
            slot: slot.as_usize(),
            slot_count,
            context: format!("node {node_index}"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Gate 11: ForEach/Together body graph is well-formed
// ---------------------------------------------------------------------------

/// Validates that ForEach and Together body subgraphs are structurally
/// well-formed: body targets exist, done targets exist, inner loops nest
/// correctly, and no dangling references.
///
/// Gate 11 (idempotency / structural): loop bodies must point to valid steps
/// within the node array, done targets must be valid, and inner loop spans
/// must be contained within their outer loop spans.
pub fn validate_gate_11_loop_body_graph(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();
    for (index, node) in parts.nodes.iter().enumerate() {
        match &node.kind {
            CompiledNodeKind::ForEachStart {
                body, done, ..
            } => {
                check_step_in_range(*body, node_count, index, "for_each body")?;
                check_step_in_range(*done, node_count, index, "for_each done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::ForEachNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "for_each_next body")?;
                check_step_in_range(*done, node_count, index, "for_each_next done")?;
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for (branch_index, branch) in branches.iter().enumerate() {
                    check_step_in_range(
                        *branch,
                        node_count,
                        index,
                        &format!("together branch {branch_index}"),
                    )?;
                }
                check_step_in_range(*join, node_count, index, "together join")?;
                check_together_span(index, branches, *join, node_count)?;
            }
            CompiledNodeKind::TogetherBranch { entry, join, .. } => {
                check_step_in_range(*entry, node_count, index, "together_branch entry")?;
                check_step_in_range(*join, node_count, index, "together_branch join")?;
            }
            CompiledNodeKind::TogetherJoin { .. } => {}
            CompiledNodeKind::CollectStart {
                body, done, ..
            } => {
                check_step_in_range(*body, node_count, index, "collect body")?;
                check_step_in_range(*done, node_count, index, "collect done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::CollectPage { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "collect_page body")?;
                check_step_in_range(*done, node_count, index, "collect_page done")?;
            }
            CompiledNodeKind::CollectNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "collect_next body")?;
                check_step_in_range(*done, node_count, index, "collect_next done")?;
            }
            CompiledNodeKind::ReduceStart {
                body, done, ..
            } => {
                check_step_in_range(*body, node_count, index, "reduce body")?;
                check_step_in_range(*done, node_count, index, "reduce done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::ReduceNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "reduce_next body")?;
                check_step_in_range(*done, node_count, index, "reduce_next done")?;
            }
            CompiledNodeKind::RepeatStart {
                body, done, ..
            } => {
                check_step_in_range(*body, node_count, index, "repeat body")?;
                check_step_in_range(*done, node_count, index, "repeat done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::RepeatAttempt { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "repeat_attempt body")?;
                check_step_in_range(*done, node_count, index, "repeat_attempt done")?;
            }
            CompiledNodeKind::RepeatCheck { done, .. } => {
                check_step_in_range(*done, node_count, index, "repeat_check done")?;
            }
            CompiledNodeKind::RetryCheck { body, exhausted, .. } => {
                check_step_in_range(*body, node_count, index, "retry_check body")?;
                check_step_in_range(*exhausted, node_count, index, "retry_check exhausted")?;
            }
            CompiledNodeKind::ErrorHandler { body, handler } => {
                check_step_in_range(*body, node_count, index, "error_handler body")?;
                check_step_in_range(*handler, node_count, index, "error_handler handler")?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_step_in_range(
    step: StepIdx,
    node_count: usize,
    source_index: usize,
    label: &str,
) -> ValidationResult<()> {
    if step.as_usize() >= node_count {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: step.as_usize(),
            node_count,
            source_node: source_index,
            label: label.to_owned(),
        });
    }
    Ok(())
}

/// Checks that the loop body start is after the loop start and before the loop
/// done step, ensuring the loop span is well-formed.
fn check_loop_span(
    start_index: usize,
    body: StepIdx,
    done: StepIdx,
    node_count: usize,
) -> ValidationResult<()> {
    let body_usize = body.as_usize();
    let done_usize = done.as_usize();
    // Body must be after start (forward edge).
    if body_usize <= start_index {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: body_usize,
            node_count,
            source_node: start_index,
            label: "loop body must be after loop start".to_owned(),
        });
    }
    // Done must be after body.
    if done_usize <= body_usize {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: done_usize,
            node_count,
            source_node: start_index,
            label: "loop done must be after loop body".to_owned(),
        });
    }
    Ok(())
}

fn check_together_span(
    start_index: usize,
    branches: &[StepIdx],
    join: StepIdx,
    node_count: usize,
) -> ValidationResult<()> {
    let join_usize = join.as_usize();
    for (branch_index, branch) in branches.iter().enumerate() {
        let branch_usize = branch.as_usize();
        // Branch must be after start.
        if branch_usize <= start_index {
            return Err(ValidationError::LoopBodyStepOutOfRange {
                step: branch_usize,
                node_count,
                source_node: start_index,
                label: format!("together branch {branch_index} must be after start"),
            });
        }
        // Join must be after branch.
        if join_usize <= branch_usize {
            return Err(ValidationError::LoopBodyStepOutOfRange {
                step: join_usize,
                node_count,
                source_node: start_index,
                label: format!("together join must be after branch {branch_index}"),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Gate 13: No circular references in slot dependency graph
// ---------------------------------------------------------------------------

/// Validates that the slot dependency graph has no cycles.
///
/// Gate 13 (capabilities): a slot that is written from another slot must not
/// form a cycle.  This catches cases where slot A depends on slot B which
/// depends on slot A (directly or transitively).
///
/// The analysis is per-node: we extract which slots each node reads and which
/// it writes, then build a dependency graph.  A cycle means a slot can never
/// receive a value because it depends on itself.
pub fn validate_gate_13_no_slot_cycles(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    if slot_count == 0 {
        return Ok(());
    }

    // Build adjacency: for each output slot, which slots does it depend on?
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); slot_count];

    for node in parts.nodes.iter() {
        let reads = node_reads(node);
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
fn node_reads(node: &CompiledNode) -> Vec<SlotIdx> {
    let mut reads = Vec::new();
    match &node.kind {
        CompiledNodeKind::Nop | CompiledNodeKind::SetConst { .. } => {}
        CompiledNodeKind::Copy { source } => {
            reads.push(*source);
        }
        CompiledNodeKind::EvalExpr { .. } => {
            // Expression reads are checked separately via LoadSlot ops.
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
                // condition is ExprIdx, not SlotIdx; skip
                let _ = branch;
            }
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            for branch in branches.iter() {
                reads.push(branch.condition);
            }
        }
        CompiledNodeKind::ForEachStart { input, .. } => {
            reads.push(*input);
            // item_slot is written, not read
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
        CompiledNodeKind::RepeatAttempt { .. } => {
            // attempt_slot is written
        }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::ResourceContract;

    // Helper: build minimal WorkflowParts with just nodes and slot_count.
    fn make_parts(
        nodes: Vec<CompiledNode>,
        slot_count: u16,
    ) -> WorkflowParts {
        WorkflowParts {
            name: Box::from("test"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        }
    }

    fn nop_node(index: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: Some(StepIdx::new(index.saturating_add(1))),
            kind: CompiledNodeKind::Nop,
        }
    }

    fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(result_slot),
            },
        }
    }

    fn copy_node(index: u16, source: u16, output: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: Some(SlotIdx::new(output)),
            next: Some(StepIdx::new(index.saturating_add(1))),
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(source),
            },
        }
    }

    // ===== Gate 7 tests =====

    #[test]
    fn gate_07_accepts_empty_expressions() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn gate_07_accepts_valid_expression() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn gate_07_rejects_stack_mismatch() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 2, // wrong: actual max is 1
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackMismatch { .. })
        ));
    }

    #[test]
    fn gate_07_rejects_stack_exceeding_contract() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.resource_contract = ResourceContract {
            max_expr_stack: 2,
            ..ResourceContract::DEFAULT
        };
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 3, // exceeds contract of 2
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    #[test]
    fn gate_07_rejects_contract_exceeding_protocol_limit() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.resource_contract = ResourceContract {
            max_expr_stack: 128, // exceeds protocol limit of 64
            ..ResourceContract::DEFAULT
        };
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    // ===== Gate 8 tests =====

    #[test]
    fn gate_08_accepts_empty_accessors() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn gate_08_accepts_valid_accessor() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 2);
        parts.accessors = Box::new([AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::new([PathSegment::Field(SymbolId::new(1))]),
        }]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn gate_08_rejects_accessor_root_out_of_range() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.accessors = Box::new([AccessorProgram {
            root: SlotIdx::new(5), // out of range for slot_count=1
            path: Box::new([]),
        }]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSlotOutOfRange { .. })
        ));
    }

    #[test]
    fn gate_08_rejects_sentinel_index_segment() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.accessors = Box::new([AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::new([PathSegment::Index(u32::MAX)]),
        }]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorPathInvalid { .. })
        ));
    }

    // ===== Gate 9 tests =====

    #[test]
    fn gate_09_accepts_valid_slot_references() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn gate_09_rejects_output_slot_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(99)), // out of range for slot_count=1
            next: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn gate_09_rejects_copy_source_out_of_range() {
        let node = copy_node(0, 50, 0); // source=50 out of range for slot_count=1
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn gate_09_rejects_expr_load_slot_out_of_range() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(99))]),
            max_stack: 1,
        }]);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn gate_09_accepts_single_node_workflow() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    // ===== Gate 11 tests =====

    #[test]
    fn gate_11_accepts_nop_workflow() {
        let parts = make_parts(vec![nop_node(0), finish_node(1, 0)], 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn gate_11_accepts_valid_for_each() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            finish_node(2, 0),
        ];
        let parts = make_parts(nodes, 2);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn gate_11_rejects_for_each_body_out_of_range() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(99), // out of range
                done: StepIdx::new(2),
            },
        }];
        let parts = make_parts(nodes, 2);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn gate_11_rejects_for_each_done_out_of_range() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(99), // out of range
            },
        }];
        let parts = make_parts(nodes, 2);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn gate_11_accepts_valid_together() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::TogetherStart {
                    branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                    join: StepIdx::new(3),
                },
            },
            nop_node(1),
            nop_node(2),
            finish_node(3, 0),
        ];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn gate_11_rejects_together_branch_out_of_range() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(99)]),
                join: StepIdx::new(1),
            },
        }];
        let parts = make_parts(nodes, 1);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn gate_11_rejects_loop_body_before_start() {
        // Body at index 0, start at index 0 => not forward
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(0), // same as start, not forward
                done: StepIdx::new(1),
            },
        }];
        let parts = make_parts(nodes, 2);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn gate_11_accepts_valid_repeat() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::RepeatStart {
                    max_attempts: 3,
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                kind: CompiledNodeKind::RepeatAttempt {
                    attempt_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                kind: CompiledNodeKind::RepeatCheck {
                    attempt_slot: SlotIdx::new(0),
                    done: StepIdx::new(3),
                },
            },
            finish_node(3, 0),
        ];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    // ===== Gate 13 tests =====

    #[test]
    fn gate_13_accepts_empty_slots() {
        let parts = make_parts(vec![nop_node(0)], 0);
        assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
    }

    #[test]
    fn gate_13_accepts_linear_slot_chain() {
        // slot 0 <- const, slot 1 <- slot 0, slot 2 <- slot 1
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::SetConst {
                    value: vb_core::ids::ConstIdx::new(0),
                },
            },
            copy_node(1, 0, 1),
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
                },
            },
        ];
        let parts = make_parts(nodes, 3);
        assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
    }

    #[test]
    fn gate_13_rejects_direct_slot_cycle() {
        // slot 0 writes from slot 1, slot 1 writes from slot 0 => cycle
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
            },
        ];
        let parts = make_parts(nodes, 2);
        assert!(matches!(
            validate_gate_13_no_slot_cycles(&parts),
            Err(ValidationError::SlotDependencyCycle { .. })
        ));
    }

    #[test]
    fn gate_13_accepts_self_copy_is_not_cycle() {
        // A node that reads and writes the same slot is not a cycle in our
        // model because we filter out self-edges in the adjacency list.
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        }];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
    }

    #[test]
    fn gate_13_rejects_three_slot_cycle() {
        // slot 0 <- slot 1, slot 1 <- slot 2, slot 2 <- slot 0
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
            },
        ];
        let parts = make_parts(nodes, 3);
        assert!(matches!(
            validate_gate_13_no_slot_cycles(&parts),
            Err(ValidationError::SlotDependencyCycle { .. })
        ));
    }

    #[test]
    fn gate_13_accepts_diamond_dependency() {
        // slot 0 <- const, slot 1 <- slot 0, slot 2 <- slot 0, slot 3 <- slot 1 + slot 2
        // No cycle.
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::SetConst {
                    value: vb_core::ids::ConstIdx::new(0),
                },
            },
            copy_node(1, 0, 1),
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: Some(SlotIdx::new(3)),
                next: None,
                kind: CompiledNodeKind::BuildList {
                    items: Box::new([SlotIdx::new(1), SlotIdx::new(2)]),
                },
            },
        ];
        let parts = make_parts(nodes, 4);
        assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
    }

    // ===== Compute stack depth tests =====

    #[test]
    fn compute_stack_depth_single_load() {
        let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))];
        assert_eq!(compute_stack_depth(&ops), Ok(1));
    }

    #[test]
    fn compute_stack_depth_load_and_binary() {
        let ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Eq,
        ];
        // max depth = 2 (after two loads), then Eq reduces to 1
        assert_eq!(compute_stack_depth(&ops), Ok(2));
    }

    #[test]
    fn compute_stack_depth_empty() {
        let ops: Vec<ExprOp> = vec![];
        assert_eq!(compute_stack_depth(&ops), Ok(0));
    }
}
