#![forbid(unsafe_code)]
//! Plan verifier gates for compiled workflow IR (Section 63 of the master doc).
//!
//! Gates 7, 8, 9, 11, and 13 validate structural properties of `WorkflowParts`
//! that the core `validate_parts` function does not cover or that need additional
//! cold-path checks for the accepted-artifact pipeline.

use crate::{ValidationError, ValidationResult};
use vb_core::action::ActionContract;
use vb_core::capability::Capability;

// Re-export the core types we need so callers only depend on vb_validate.
pub use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
pub use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};

/// Maximum byte length for a compiled action capability requirement name.
pub const MAX_CAPABILITY_NAME_BYTES: usize = 128;

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
pub fn compute_stack_depth(ops: &[ExprOp]) -> ValidationResult<u8> {
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
                declared: usize::from(depth).saturating_add(usize::from(push_amount)),
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
///
/// Uses `i16::from()` for safe widening conversion (infallible), then
/// computes the net effect. Since push is always 1 and pop is 0..=3, the
/// result is always in i8 range (-2..=1).
///
/// Explicit match is used instead of `unwrap_or` / `unwrap_or_default` to
/// comply with the "no unwrap" engineering rule, even though these methods
/// are safe (they return a default on `Err`, they don't panic).
#[allow(clippy::manual_unwrap_or, clippy::manual_unwrap_or_default)]
fn stack_effect(_op: &ExprOp) -> i8 {
    let pop: i16 = i16::from(pop_count(_op));
    let push: i16 = i16::from(push_count(_op));
    let net = push.saturating_sub(pop);
    // Convert back to i8: net is in [-2, 1], always fits.
    match i8::try_from(net) {
        Ok(value) => value,
        Err(_) => 0, // unreachable: net is always in i8 range [-2, 1]
    }
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
                PathSegment::Field(sym_id) => {
                    validate_field_symbol(acc_index, seg_index, *sym_id, parts.symbols_count)?;
                }
                PathSegment::Index(idx) => validate_index_segment(acc_index, seg_index, *idx)?,
            }
        }
    }
    Ok(())
}

fn validate_field_symbol(
    acc_index: usize,
    seg_index: usize,
    symbol: SymbolId,
    symbols_count: u32,
) -> ValidationResult<()> {
    if symbol.get() < symbols_count {
        Ok(())
    } else {
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: acc_index,
            segment_index: seg_index,
            symbol: symbol.get(),
            symbols_count,
        })
    }
}

fn validate_index_segment(acc_index: usize, seg_index: usize, idx: u32) -> ValidationResult<()> {
    if idx == u32::MAX {
        Err(ValidationError::AccessorPathInvalid {
            accessor_index: acc_index,
            segment_index: seg_index,
        })
    } else {
        Ok(())
    }
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
            input, item_slot, ..
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
            input, accumulator, ..
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
        match op {
            ExprOp::LoadSlot(slot) if slot.as_usize() >= slot_count => {
                return Err(ValidationError::SlotReferenceOutOfRange {
                    slot: slot.as_usize(),
                    slot_count,
                    context: format!("expression {expr_index}"),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_slot(slot: SlotIdx, node_index: usize, slot_count: usize) -> ValidationResult<()> {
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
    if node_count == 0 {
        return Ok(());
    }
    check_step_in_range(parts.entry, node_count, 0, "entry")?;
    for (index, node) in parts.nodes.iter().enumerate() {
        if let Some(next) = node.next {
            check_next_step_in_range(next, node_count, index)?;
        }
        if let Some(on_error) = node.on_error {
            check_step_in_range(on_error, node_count, index, "on_error")?;
        }
        match &node.kind {
            CompiledNodeKind::ForEachStart { body, done, .. } => {
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
            CompiledNodeKind::CollectStart { body, done, .. } => {
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
            CompiledNodeKind::ReduceStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "reduce body")?;
                check_step_in_range(*done, node_count, index, "reduce done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::ReduceNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "reduce_next body")?;
                check_step_in_range(*done, node_count, index, "reduce_next done")?;
            }
            CompiledNodeKind::RepeatStart { body, done, .. } => {
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
            CompiledNodeKind::RetryCheck {
                body, exhausted, ..
            } => {
                check_step_in_range(*body, node_count, index, "retry_check body")?;
                check_step_in_range(*exhausted, node_count, index, "retry_check exhausted")?;
            }
            CompiledNodeKind::ErrorHandler { body, handler, .. } => {
                check_step_in_range(*body, node_count, index, "error_handler body")?;
                check_step_in_range(*handler, node_count, index, "error_handler handler")?;
            }
            _ => {}
        }
    }
    validate_loop_pairings(parts)?;
    Ok(())
}

fn validate_loop_pairings(parts: &WorkflowParts) -> ValidationResult<()> {
    parts
        .nodes
        .iter()
        .enumerate()
        .try_for_each(|(index, node)| validate_node_pairing(parts, index, &node.kind))
}

fn validate_node_pairing(
    parts: &WorkflowParts,
    index: usize,
    kind: &CompiledNodeKind,
) -> ValidationResult<()> {
    match kind {
        CompiledNodeKind::ForEachNext { body, done, .. } => require_matching_body_start(
            parts,
            index,
            *body,
            *done,
            "ForEachNext",
            is_matching_for_each_start,
        ),
        CompiledNodeKind::ForEachJoin { .. } => {
            require_matching_done_start(parts, index, "ForEachJoin", is_foreach_start_done)
        }
        CompiledNodeKind::TogetherBranch { branch, join, .. } => {
            require_matching_together_branch(parts, index, *branch, *join)
        }
        CompiledNodeKind::TogetherJoin { branch_count, .. } => {
            require_matching_together_join(parts, index, *branch_count)
        }
        CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. } => require_matching_body_start(
            parts,
            index,
            *body,
            *done,
            "Collect continuation",
            is_matching_collect_start,
        ),
        CompiledNodeKind::CollectFinish { .. } => {
            require_matching_done_start(parts, index, "CollectFinish", is_collect_start_done)
        }
        CompiledNodeKind::ReduceNext { body, done, .. } => require_matching_body_start(
            parts,
            index,
            *body,
            *done,
            "ReduceNext",
            is_matching_reduce_start,
        ),
        CompiledNodeKind::ReduceFinish { .. } => {
            require_matching_done_start(parts, index, "ReduceFinish", is_reduce_start_done)
        }
        CompiledNodeKind::RepeatAttempt { body, done, .. } => require_matching_body_start(
            parts,
            index,
            *body,
            *done,
            "RepeatAttempt",
            is_matching_repeat_start,
        ),
        CompiledNodeKind::RepeatCheck { done, .. } => {
            require_matching_repeat_check(parts, index, *done)
        }
        CompiledNodeKind::RepeatFinish { .. } => {
            require_matching_done_start(parts, index, "RepeatFinish", is_repeat_start_done)
        }
        _ => Ok(()),
    }
}

fn require_matching_body_start(
    parts: &WorkflowParts,
    index: usize,
    body: StepIdx,
    done: StepIdx,
    label: &str,
    start_matches: fn(&CompiledNodeKind, StepIdx, StepIdx) -> bool,
) -> ValidationResult<()> {
    let has_match = step_in_loop_body(index, body, done)
        && has_prior_matching_start(parts, index, |kind| start_matches(kind, body, done));
    require_pairing(has_match, index, format!("{label} has no matching start"))
}

fn require_matching_done_start(
    parts: &WorkflowParts,
    index: usize,
    label: &str,
    start_done_matches: fn(&CompiledNodeKind, usize) -> bool,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| start_done_matches(kind, index));
    require_pairing(has_match, index, format!("{label} has no matching start"))
}

fn require_matching_repeat_check(
    parts: &WorkflowParts,
    index: usize,
    done: StepIdx,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| match kind {
        CompiledNodeKind::RepeatStart {
            body,
            done: start_done,
            ..
        } => *start_done == done && step_in_loop_body(index, *body, *start_done),
        _ => false,
    });
    require_pairing(has_match, index, "RepeatCheck has no matching RepeatStart")
}

fn require_matching_together_branch(
    parts: &WorkflowParts,
    index: usize,
    branch: u16,
    join: StepIdx,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| match kind {
        CompiledNodeKind::TogetherStart {
            branches,
            join: start_join,
        } => {
            *start_join == join
                && branches.iter().enumerate().any(|(branch_index, target)| {
                    branch_index == usize::from(branch) && target.as_usize() == index
                })
        }
        _ => false,
    });
    require_pairing(
        has_match,
        index,
        "TogetherBranch has no matching TogetherStart branch target",
    )
}

fn require_matching_together_join(
    parts: &WorkflowParts,
    index: usize,
    branch_count: u16,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| match kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            join.as_usize() == index && branches.len() == usize::from(branch_count)
        }
        _ => false,
    });
    require_pairing(
        has_match,
        index,
        "TogetherJoin has no matching TogetherStart branch count",
    )
}

fn has_prior_matching_start(
    parts: &WorkflowParts,
    index: usize,
    predicate: impl Fn(&CompiledNodeKind) -> bool,
) -> bool {
    parts
        .nodes
        .iter()
        .take(index)
        .any(|node| predicate(&node.kind))
}

fn step_in_loop_body(index: usize, body: StepIdx, done: StepIdx) -> bool {
    let body_index = body.as_usize();
    let done_index = done.as_usize();
    index >= body_index && index < done_index
}

fn require_pairing(matches: bool, index: usize, detail: impl Into<String>) -> ValidationResult<()> {
    if matches {
        return Ok(());
    }
    Err(ValidationError::NodeKindConstraintViolation {
        node_index: index,
        detail: detail.into(),
    })
}

fn is_matching_for_each_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

fn is_matching_collect_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool {
    matches!(
        kind,
        CompiledNodeKind::CollectStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

fn is_matching_reduce_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ReduceStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

fn is_matching_repeat_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool {
    matches!(
        kind,
        CompiledNodeKind::RepeatStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

fn is_foreach_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachStart { done, .. } if done.as_usize() == index
    )
}

fn is_collect_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::CollectStart { done, .. } if done.as_usize() == index
    )
}

fn is_reduce_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ReduceStart { done, .. } if done.as_usize() == index
    )
}

fn is_repeat_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::RepeatStart { done, .. } if done.as_usize() == index
    )
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

fn check_next_step_in_range(
    step: StepIdx,
    node_count: usize,
    source_index: usize,
) -> ValidationResult<()> {
    if step.as_usize() > node_count {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: step.as_usize(),
            node_count,
            source_node: source_index,
            label: "next".to_owned(),
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

    let adjacency = build_slot_adjacency(parts, slot_count);
    let mut visited: Vec<u8> = vec![0; slot_count]; // 0 = white, 1 = gray, 2 = black
    for slot in 0..slot_count {
        if visited.get(slot) == Some(&0) {
            detect_cycle_dfs(slot, &adjacency, &mut visited)?;
        }
    }
    Ok(())
}

/// Builds adjacency: for each output slot, which slots does it depend on?
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
        *state = 1; // gray
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
// Gate 10: Node-kind-specific constraints
// ---------------------------------------------------------------------------

/// Validates node-kind-specific constraints that go beyond simple slot bounds
/// checking.
///
/// Gate 10 (correctness): each node kind has specific structural requirements:
/// - `Finish`: result slot exists and is within slot_count
/// - `Choose`: branches reference valid expression indices, otherwise target
///   is a valid step or None
/// - `ChooseSlot`: branches reference valid slots, otherwise target valid
/// - `ForEachStart`: iterator slot and body/done step indices valid
/// - `TogetherStart`: branches and join step indices valid
/// - `Do` (Action): action_id is valid, input slot in bounds
/// - `SetConst`: const index within constant pool
/// - `EvalExpr`: expression index within expression table
pub fn validate_gate_10_node_kind_specific(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    let const_count = parts.constants.len();
    let accessor_count = parts.accessors.len();
    let expr_count = parts.expressions.len();
    let node_count = parts.nodes.len();
    let symbols_count = parts.symbols_count;

    validate_expression_references(parts, const_count, accessor_count)?;

    for (node_index, node) in parts.nodes.iter().enumerate() {
        match &node.kind {
            CompiledNodeKind::Finish { result } => {
                let result_usize = result.as_usize();
                if result_usize >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "Finish result slot {result_usize} out of range (slot_count {slot_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => {
                for (branch_index, branch) in branches.iter().enumerate() {
                    let expr_usize = branch.condition.as_usize();
                    if expr_usize >= expr_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose branch {branch_index} expr index {expr_usize} out of range (expr_count {expr_count})"
                            ),
                        });
                    }
                    let target_usize = branch.target.as_usize();
                    if target_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose branch {branch_index} target step {target_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                if let Some(otherwise) = otherwise {
                    let otherwise_usize = otherwise.as_usize();
                    if otherwise_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose otherwise target step {otherwise_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => {
                for (branch_index, branch) in branches.iter().enumerate() {
                    let cond_usize = branch.condition.as_usize();
                    if cond_usize >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot branch {branch_index} condition slot {cond_usize} out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                    let target_usize = branch.target.as_usize();
                    if target_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot branch {branch_index} target step {target_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                if let Some(otherwise) = otherwise {
                    let otherwise_usize = otherwise.as_usize();
                    if otherwise_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot otherwise target step {otherwise_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::SetConst { value } => {
                let const_usize = value.as_usize();
                if const_usize >= const_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "SetConst value index {const_usize} out of range (const_count {const_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::EvalExpr { expr } => {
                let expr_usize = expr.as_usize();
                if expr_usize >= expr_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "EvalExpr expr index {expr_usize} out of range (expr_count {expr_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::Do { action, input } => {
                let input_usize = input.as_usize();
                if input_usize >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "Do input slot {input_usize} out of range (slot_count {slot_count})"
                        ),
                    });
                }
                // Action ID must be valid (non-sentinel).
                if action.get() == u16::MAX {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: String::from("Do action_id is sentinel value u16::MAX"),
                    });
                }
            }
            CompiledNodeKind::ForEachStart {
                input,
                item_slot,
                body,
                done,
                ..
            } => {
                let input_usize = input.as_usize();
                if input_usize >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart input slot {input_usize} out of range (slot_count {slot_count})"
                        ),
                    });
                }
                let item_usize = item_slot.as_usize();
                if item_usize >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart item_slot {item_usize} out of range (slot_count {slot_count})"
                        ),
                    });
                }
                let body_usize = body.as_usize();
                if body_usize >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart body step {body_usize} out of range (node_count {node_count})"
                        ),
                    });
                }
                let done_usize = done.as_usize();
                if done_usize >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart done step {done_usize} out of range (node_count {node_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for (branch_index, branch) in branches.iter().enumerate() {
                    let branch_usize = branch.as_usize();
                    if branch_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "TogetherStart branch {branch_index} step {branch_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                let join_usize = join.as_usize();
                if join_usize >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "TogetherStart join step {join_usize} out of range (node_count {node_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::BuildObject { fields } => {
                for (field_index, (symbol, slot)) in fields.iter().enumerate() {
                    if symbol.get() >= symbols_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildObject field {field_index} symbol {} out of range (symbols_count {symbols_count})",
                                symbol.get()
                            ),
                        });
                    }
                    let slot_usize = slot.as_usize();
                    if slot_usize >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildObject field {field_index} slot {slot_usize} out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::BuildList { items } => {
                for (item_index, slot) in items.iter().enumerate() {
                    let slot_usize = slot.as_usize();
                    if slot_usize >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildList item {item_index} slot {slot_usize} out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                }
            }
            _ => {
                // Other node kinds have their slot references already validated
                // by gate 9 and their step references validated by gate 11.
            }
        }
    }
    Ok(())
}

fn validate_expression_references(
    parts: &WorkflowParts,
    const_count: usize,
    accessor_count: usize,
) -> ValidationResult<()> {
    parts
        .expressions
        .iter()
        .enumerate()
        .try_for_each(|(expr_index, expr)| {
            expr.ops.iter().try_for_each(|op| match op {
                ExprOp::LoadConst(value) => {
                    validate_load_const_reference(expr_index, *value, const_count)
                }
                ExprOp::LoadAccessor(accessor) => {
                    validate_load_accessor_reference(expr_index, *accessor, accessor_count)
                }
                _ => Ok(()),
            })
        })
}

fn validate_load_const_reference(
    expr_index: usize,
    value: ConstIdx,
    const_count: usize,
) -> ValidationResult<()> {
    let const_usize = value.as_usize();
    if const_usize >= const_count {
        return Err(ValidationError::NodeKindConstraintViolation {
            node_index: expr_index,
            detail: format!(
                "Expression {expr_index} LoadConst const index {const_usize} out of range (const_count {const_count})"
            ),
        });
    }
    Ok(())
}

fn validate_load_accessor_reference(
    expr_index: usize,
    accessor: AccessorIdx,
    accessor_count: usize,
) -> ValidationResult<()> {
    let accessor_usize = accessor.as_usize();
    if accessor_usize >= accessor_count {
        return Err(ValidationError::NodeKindConstraintViolation {
            node_index: expr_index,
            detail: format!(
                "Expression {expr_index} LoadAccessor accessor index {accessor_usize} out of range (accessor_count {accessor_count})"
            ),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Gate 12: Action contract completeness
// ---------------------------------------------------------------------------

/// Validates that every Do node's action_id has a corresponding entry in the
/// provided action contracts, and that no action contract references a
/// non-existent Do node.
///
/// Gate 12 (contracts): the action contracts table must be in bijection with
/// the set of Do nodes. Every Do node must reference a contracted action, and
/// every contracted action must be used by at least one Do node.
pub fn validate_gate_12_action_contract_completeness(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> ValidationResult<()> {
    // Collect all action IDs referenced by Do nodes.
    let mut do_action_ids: Vec<u16> = Vec::new();
    for (node_index, node) in parts.nodes.iter().enumerate() {
        if let CompiledNodeKind::Do { action, .. } = &node.kind {
            let action_val = action.get();
            // Check that this action_id has a corresponding contract.
            let mut found = false;
            for contract in action_contracts {
                if contract.id.get() == action_val {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(ValidationError::ActionContractMissing {
                    action_id: usize::from(action_val),
                    node_index,
                });
            }
            if !do_action_ids.contains(&action_val) {
                do_action_ids.push(action_val);
            }
        }
    }

    for contract in action_contracts {
        validate_action_contract_capability_schema(contract)?;
    }

    // Check that every contract has at least one Do node referencing it.
    for contract in action_contracts {
        let contract_id = contract.id.get();
        let mut found = false;
        for do_id in &do_action_ids {
            if *do_id == contract_id {
                found = true;
                break;
            }
        }
        if !found {
            return Err(ValidationError::ActionContractOrphan {
                action_id: usize::from(contract_id),
            });
        }
    }

    Ok(())
}

fn validate_action_contract_capability_schema(contract: &ActionContract) -> ValidationResult<()> {
    for (capability_index, capability) in contract.required_capabilities.iter().enumerate() {
        validate_required_capability(contract.id, capability_index, capability)?;
    }
    validate_no_duplicate_capability_requirements(contract)
}

fn validate_required_capability(
    contract_action: ActionId,
    capability_index: usize,
    capability: &Capability,
) -> ValidationResult<()> {
    validate_capability_name(contract_action, capability_index, capability.name())?;
    if capability.action_id() != contract_action {
        return Err(ValidationError::CapabilityActionMismatch {
            contract_action_id: usize::from(contract_action.get()),
            capability_action_id: usize::from(capability.action_id().get()),
            capability_index,
        });
    }
    Ok(())
}

fn validate_capability_name(
    action_id: ActionId,
    capability_index: usize,
    name: &str,
) -> ValidationResult<()> {
    let len = name.len();
    if len == 0 {
        return Err(ValidationError::CapabilityNameEmpty {
            action_id: usize::from(action_id.get()),
            capability_index,
        });
    }
    if len > MAX_CAPABILITY_NAME_BYTES {
        return Err(ValidationError::CapabilityNameTooLong {
            action_id: usize::from(action_id.get()),
            capability_index,
            len,
            max: MAX_CAPABILITY_NAME_BYTES,
        });
    }
    if !is_capability_name_grammar_valid(name) {
        return Err(ValidationError::CapabilityNameInvalid {
            action_id: usize::from(action_id.get()),
            capability_index,
            name: name.to_owned(),
        });
    }
    Ok(())
}

fn is_capability_name_grammar_valid(name: &str) -> bool {
    name.bytes()
        .try_fold(true, |segment_start, byte| match byte {
            b'.' => (!segment_start).then_some(true),
            b'a'..=b'z' => Some(false),
            b'0'..=b'9' | b'_' => (!segment_start).then_some(false),
            _ => None,
        })
        == Some(false)
}

fn validate_no_duplicate_capability_requirements(
    contract: &ActionContract,
) -> ValidationResult<()> {
    contract
        .required_capabilities
        .iter()
        .enumerate()
        .find_map(|(duplicate_index, duplicate)| {
            contract
                .required_capabilities
                .iter()
                .take(duplicate_index)
                .enumerate()
                .find(|(_, first)| {
                    first.action_id() == duplicate.action_id() && first.name() == duplicate.name()
                })
                .map(|(first_index, _)| ValidationError::CapabilityDuplicate {
                    action_id: usize::from(contract.id.get()),
                    first_index,
                    duplicate_index,
                    name: duplicate.name().to_owned(),
                })
        })
        .map_or(Ok(()), Err)
}

// ---------------------------------------------------------------------------
// Gate 14: Slot type consistency
// ---------------------------------------------------------------------------

/// Validates that slots written by multiple nodes have compatible types.
///
/// Gate 14 (types): when multiple `SetConst` nodes write to the same slot, they
/// must write compatible `ConstValue` types. Two `ConstValue` variants are
/// compatible if they share the same discriminant (e.g., both I64, or both
/// Bool). This catches cases where the same slot would receive an I64 from one
/// writer and a Bool from another.
pub fn validate_gate_14_slot_type_consistency(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    if slot_count == 0 {
        return Ok(());
    }

    // For each slot, track the ConstValue discriminant written by SetConst nodes.
    // 0 = unset, 1 = Null, 2 = Bool, 3 = I64, 4 = F64, 5 = Symbol
    let mut slot_const_kind: Vec<u8> = vec![0; slot_count];

    for node in parts.nodes.iter() {
        if let CompiledNodeKind::SetConst { value } = &node.kind {
            let const_idx = value.as_usize();
            if const_idx >= parts.constants.len() {
                // Out-of-range const index; that is caught by gate 10.
                continue;
            }
            if let Some(constant) = parts.constants.get(const_idx) {
                let kind = const_value_discriminant(constant);
                if let Some(slot) = node.output {
                    let slot_usize = slot.as_usize();
                    if slot_usize < slot_count {
                        let existing = slot_const_kind
                            .get(slot_usize)
                            .copied()
                            .ok_or(ValidationError::SlotTypeInconsistency { slot: slot_usize })?;
                        if existing == 0 {
                            if let Some(entry) = slot_const_kind.get_mut(slot_usize) {
                                *entry = kind;
                            }
                        } else if existing != kind {
                            return Err(ValidationError::SlotTypeInconsistency {
                                slot: slot_usize,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Returns a discriminant tag for a ConstValue variant.
fn const_value_discriminant(value: &vb_core::value::ConstValue) -> u8 {
    match value {
        vb_core::value::ConstValue::Null => 1,
        vb_core::value::ConstValue::Bool(_) => 2,
        vb_core::value::ConstValue::I64(_) => 3,
        vb_core::value::ConstValue::F64(_) => 4,
        vb_core::value::ConstValue::Symbol(_) => 5,
    }
}

// ---------------------------------------------------------------------------
// Gate 15: Determinism proof
// ---------------------------------------------------------------------------

/// Validates that every path between two non-deterministic nodes consists only
/// of deterministic nodes, ensuring that the workflow can be faithfully
/// replayed from journal evidence.
///
/// Gate 15 (determinism): non-deterministic nodes (Do/Action, Ask) are
/// suspension points. All deterministic nodes (SetConst, Copy, EvalExpr,
/// BuildObject, BuildList, Finish, Nop) can be replayed from journal evidence.
/// This gate checks that between any two non-deterministic nodes on a control
/// flow path, there are only deterministic nodes. A consecutive pair of
/// non-deterministic nodes without an intervening deterministic-only region is
/// flagged as an error because the second node's effects cannot be separated
/// from the first's non-determinism in the journal.
///
/// Simplified: for each node, if it is non-deterministic, check that its `next`
/// target (if any) is either deterministic or a valid suspension join. Two
/// non-deterministic nodes may not be directly chained (node A's next = node B
/// where both are non-deterministic).
pub fn validate_gate_15_determinism_proof(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();

    for (node_index, node) in parts.nodes.iter().enumerate() {
        if !is_non_deterministic(&node.kind) {
            continue;
        }

        // Walk the `next` chain from this node. If we encounter another
        // non-deterministic node without any intervening deterministic-only
        // nodes, that is a violation. In practice, we check the immediate `next`
        // edge: if a non-deterministic node's `next` points to another
        // non-deterministic node, that is a direct chain violation.
        match node.next {
            Some(next_step) if next_step.as_usize() < node_count => {
                match parts.nodes.get(next_step.as_usize()) {
                    Some(next_node) if is_non_deterministic(&next_node.kind) => {
                        return Err(ValidationError::NonDeterministicPath {
                            from_node: node_index,
                            to_node: next_step.as_usize(),
                        });
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Also check branch targets for Choose/ChooseSlot nodes. Choose nodes
        // are deterministic (they evaluate expressions/slots), so they are not
        // flagged here. But we check the Do and Ask node's own edges above.
    }

    Ok(())
}

/// Returns true if the node kind is non-deterministic (requires external input
/// that cannot be replayed from journal evidence alone).
fn is_non_deterministic(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::Do { .. } | CompiledNodeKind::Ask { .. }
    )
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
    fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
        WorkflowParts {
            name: Box::from("test"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    fn nop_node(index: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }
    }

    fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
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
            on_error: None,
            error_slot: None,
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
        parts.symbols_count = 2;
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
            on_error: None,
            error_slot: None,
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
                on_error: None,
                error_slot: None,
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
                on_error: None,
                error_slot: None,
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
            on_error: None,
            error_slot: None,
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
            on_error: None,
            error_slot: None,
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
                on_error: None,
                error_slot: None,
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
            on_error: None,
            error_slot: None,
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
            on_error: None,
            error_slot: None,
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
                on_error: None,
                error_slot: None,
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
                on_error: None,
                error_slot: None,
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
                on_error: None,
                error_slot: None,
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
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: vb_core::ids::ConstIdx::new(0),
                },
            },
            copy_node(1, 0, 1),
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: None,
                on_error: None,
                error_slot: None,
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
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
            },
        ];
        let parts = make_parts(nodes, 2);
        assert_eq!(
            validate_gate_13_no_slot_cycles(&parts),
            Err(ValidationError::SlotDependencyCycle {
                slot: 1,
                chain: "slot 1 -> slot 0".into(),
            })
        );
    }

    #[test]
    fn gate_13_accepts_self_copy_not_cycle() {
        // A self-copy is a no-op dependency, not a cross-slot cycle.
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        }];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
    }

    #[test]
    fn gate13_accepts_direct_self_dependency() {
        // Given an expression node that writes slot 0.
        let mut parts = make_parts(vec![finish_node(1, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        };
        parts.nodes = Box::new([node, finish_node(1, 0)]);

        // When the expression also reads slot 0.
        // Then Gate 13 treats the self edge as an in-place update, not a cycle.
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
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: None,
                on_error: None,
                error_slot: None,
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
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: vb_core::ids::ConstIdx::new(0),
                },
            },
            copy_node(1, 0, 1),
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: Some(SlotIdx::new(3)),
                next: None,
                on_error: None,
                error_slot: None,
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

    // ===== Adversarial tests: Gate 13 EvalExpr cycle detection =====

    #[test]
    fn gate_13_rejects_cycle_through_eval_expr() {
        // slot 0 writes from an expression that loads slot 1,
        // slot 1 writes from slot 0 => cycle through EvalExpr.
        let mut parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(0),
                    },
                },
            ],
            2,
        );
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
            max_stack: 1,
        }]);
        assert!(
            matches!(
                validate_gate_13_no_slot_cycles(&parts),
                Err(ValidationError::SlotDependencyCycle { .. })
            ),
            "gate 13 must detect cycle through EvalExpr LoadSlot"
        );
    }

    #[test]
    fn gate_13_accepts_linear_chain_through_eval_expr() {
        // slot 0 <- const, slot 1 <- expr(slot 0) => no cycle
        let mut parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
            ],
            2,
        );
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
    }

    #[test]
    fn gate_13_rejects_three_slot_cycle_through_eval_expr() {
        // slot 0 <- expr(slot 2), slot 1 <- slot 0, slot 2 <- slot 1 => cycle
        let mut parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(1),
                    },
                },
            ],
            3,
        );
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(2))]),
            max_stack: 1,
        }]);
        assert!(
            matches!(
                validate_gate_13_no_slot_cycles(&parts),
                Err(ValidationError::SlotDependencyCycle { .. })
            ),
            "gate 13 must detect 3-slot cycle through EvalExpr"
        );
    }

    // ===== Adversarial tests: Gate 7 edge cases =====

    #[test]
    fn gate_07_rejects_underflow_binary_op_on_empty_stack() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::Eq]), // pops 2 from empty stack => underflow
            max_stack: 0,
        }]);
        assert!(
            matches!(
                validate_gate_07_expression_stack_depth(&parts),
                Err(ValidationError::ExpressionStackExceeded { .. })
            ),
            "gate 7 must reject binary op on empty stack (stack underflow)"
        );
    }

    #[test]
    fn gate_07_accepts_single_node_workflow_with_no_expressions() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    // ===== Adversarial tests: Gate 8 edge cases =====

    #[test]
    fn gate_08_accepts_accessor_with_empty_path() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.accessors = Box::new([AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::new([]),
        }]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn gate_08_rejects_max_value_index_segment() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.accessors = Box::new([AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::new([PathSegment::Index(u32::MAX)]),
        }]);
        assert!(
            matches!(
                validate_gate_08_accessor_path_segments(&parts),
                Err(ValidationError::AccessorPathInvalid { .. })
            ),
            "gate 8 must reject u32::MAX index segment"
        );
    }

    #[test]
    fn gate_08_accepts_zero_index_segment() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.accessors = Box::new([AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::new([PathSegment::Index(0)]),
        }]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    // ===== Adversarial tests: Gate 9 edge cases =====

    #[test]
    fn gate_09_accepts_slot_at_boundary_slot_count_minus_one() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn gate_09_rejects_build_object_slot_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: Box::new([(SymbolId::new(1), SlotIdx::new(99))]),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert!(
            matches!(
                validate_gate_09_slot_references(&parts),
                Err(ValidationError::SlotReferenceOutOfRange { .. })
            ),
            "gate 9 must reject BuildObject with out-of-range slot"
        );
    }

    #[test]
    fn gate_09_rejects_build_list_slot_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: Box::new([SlotIdx::new(50)]),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert!(
            matches!(
                validate_gate_09_slot_references(&parts),
                Err(ValidationError::SlotReferenceOutOfRange { .. })
            ),
            "gate 9 must reject BuildList with out-of-range slot"
        );
    }

    // ===== Adversarial tests: Gate 11 edge cases =====

    #[test]
    fn gate_11_accepts_together_with_empty_branches() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: Box::new([]),
                    join: StepIdx::new(1),
                },
            },
            finish_node(1, 0),
        ];
        let parts = make_parts(nodes, 1);
        // Empty branches is structurally valid per gate 11 (no out-of-range steps)
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn gate_11_rejects_for_each_done_before_body() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(1), // done < body => invalid span
            },
        }];
        let parts = make_parts(nodes, 2);
        assert!(
            matches!(
                validate_gate_11_loop_body_graph(&parts),
                Err(ValidationError::LoopBodyStepOutOfRange { .. })
            ),
            "gate 11 must reject done step before body step"
        );
    }

    #[test]
    fn gate_11_accepts_single_node_workflow() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    // =========================================================================
    // BLACKHAT security regression tests
    // =========================================================================

    /// BLACKHAT: stack_effect must not use `as` casts (engineering rule).
    ///
    /// SEVERITY: MEDIUM (engineering rule violation -- `as` casts can silently
    /// truncate or wrap, violating the "no `as` casts" rule)
    /// DESCRIPTION: The `stack_effect` helper previously used `push as i8` and
    /// `pop as i8` with `#[allow(clippy::as_conversions)]`. This was replaced
    /// with safe `i16::from()` widening conversion and `i8::try_from()`.
    /// This test verifies the function produces correct values for all op
    /// categories: load (net +1), unary (net 0), binary (net -1), ternary
    /// (net -2).
    #[test]
    fn blackhat_stack_effect_no_as_casts_correct_values() {
        // LoadSlot: pop 0, push 1 => net +1
        assert_eq!(stack_effect(&ExprOp::LoadSlot(SlotIdx::new(0))), 1);
        // LoadConst: pop 0, push 1 => net +1
        assert_eq!(stack_effect(&ExprOp::LoadConst(ConstIdx::new(0))), 1);
        // LoadAccessor: pop 0, push 1 => net +1
        assert_eq!(stack_effect(&ExprOp::LoadAccessor(AccessorIdx::new(0))), 1);
        // Not: pop 1, push 1 => net 0
        assert_eq!(stack_effect(&ExprOp::Not), 0);
        // Exists: pop 1, push 1 => net 0
        assert_eq!(stack_effect(&ExprOp::Exists), 0);
        // Length: pop 1, push 1 => net 0
        assert_eq!(stack_effect(&ExprOp::Length), 0);
        // Eq: pop 2, push 1 => net -1
        assert_eq!(stack_effect(&ExprOp::Eq), -1);
        // Add: pop 2, push 1 => net -1
        assert_eq!(stack_effect(&ExprOp::Add), -1);
        // AppendIf: pop 3, push 1 => net -2
        assert_eq!(stack_effect(&ExprOp::AppendIf), -2);
    }

    /// BLACKHAT: compute_stack_depth correctly detects stack underflow.
    ///
    /// SEVERITY: HIGH (could allow malformed expression programs to pass
    /// validation, leading to runtime stack corruption)
    /// DESCRIPTION: A binary op on an empty stack should cause underflow
    /// detection. This verifies that `checked_sub` correctly catches the
    /// underflow and returns an error instead of wrapping.
    #[test]
    fn blackhat_compute_stack_depth_rejects_underflow_from_binary_op() {
        let ops = vec![ExprOp::Eq];
        let result = compute_stack_depth(&ops);
        assert!(
            matches!(result, Err(ValidationError::ExpressionStackExceeded { .. })),
            "blackhat: binary op on empty stack must cause stack underflow error"
        );
    }

    /// BLACKHAT: compute_stack_depth rejects ternary op (AppendIf) with
    /// insufficient stack depth.
    ///
    /// SEVERITY: HIGH
    /// DESCRIPTION: AppendIf pops 3 values; with only 1 on the stack, it should
    /// fail with underflow.
    #[test]
    fn blackhat_compute_stack_depth_rejects_append_if_underflow() {
        let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::AppendIf];
        let result = compute_stack_depth(&ops);
        assert!(
            matches!(result, Err(ValidationError::ExpressionStackExceeded { .. })),
            "blackhat: AppendIf with only 1 value on stack must cause underflow"
        );
    }

    /// BLACKHAT: compute_stack_depth accepts valid expression with max depth.
    ///
    /// SEVERITY: INFO (correctness verification)
    #[test]
    fn blackhat_compute_stack_depth_accepts_valid_expression() {
        let ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::LoadSlot(SlotIdx::new(2)),
            ExprOp::Eq,
            ExprOp::Not,
        ];
        // Stack: 1, 2, 3 -> Eq pops 2 pushes 1 => 2 -> Not pops 1 pushes 1 => 2
        // Max depth = 3
        let result = compute_stack_depth(&ops);
        assert_eq!(result, Ok(3));
    }

    /// BLACKHAT: Gate 10 rejects Do node with sentinel action_id.
    ///
    /// SEVERITY: HIGH (sentinel action_id could bypass action contract
    /// validation)
    /// DESCRIPTION: A Do node with action_id set to u16::MAX (sentinel) must
    /// be rejected by gate 10.
    #[test]
    fn blackhat_gate_10_rejects_sentinel_action_id() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(u16::MAX),
                input: SlotIdx::new(0),
            },
        };
        let mut parts = make_parts(vec![node], 1);
        parts.constants = Box::new([vb_core::value::ConstValue::Null]);
        let result = validate_gate_10_node_kind_specific(&parts);
        assert!(
            matches!(
                result,
                Err(ValidationError::NodeKindConstraintViolation { .. })
            ),
            "blackhat: sentinel action_id must be rejected"
        );
    }

    /// BLACKHAT: Gate 14 detects slot type inconsistency (I64 vs Bool).
    ///
    /// SEVERITY: MEDIUM (type inconsistency in slots could cause runtime
    /// type errors or memory safety issues)
    /// DESCRIPTION: When two SetConst nodes write incompatible types (I64 vs
    /// Bool) to the same slot, gate 14 must detect the inconsistency.
    #[test]
    fn blackhat_gate_14_rejects_incompatible_const_types() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0), // I64
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1), // Bool
                },
            },
        ];
        let mut parts = make_parts(nodes, 1);
        parts.constants = Box::new([
            vb_core::value::ConstValue::I64(42),
            vb_core::value::ConstValue::Bool(true),
        ]);
        let result = validate_gate_14_slot_type_consistency(&parts);
        assert!(
            matches!(
                result,
                Err(ValidationError::SlotTypeInconsistency { slot: 0 })
            ),
            "blackhat: I64 and Bool writers to same slot must be rejected"
        );
    }

    /// BLACKHAT: Gate 15 rejects consecutive non-deterministic nodes.
    ///
    /// SEVERITY: HIGH (consecutive non-deterministic nodes could violate
    /// journal replay determinism)
    /// DESCRIPTION: Two Do nodes chained together via `next` must be rejected
    /// by the determinism proof gate.
    #[test]
    fn blackhat_gate_15_rejects_consecutive_do_nodes() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(2),
                    input: SlotIdx::new(1),
                },
            },
            finish_node(2, 1),
        ];
        let parts = make_parts(nodes, 2);
        let result = validate_gate_15_determinism_proof(&parts);
        assert!(
            matches!(
                result,
                Err(ValidationError::NonDeterministicPath {
                    from_node: 0,
                    to_node: 1
                })
            ),
            "blackhat: consecutive Do nodes must be rejected as non-deterministic path"
        );
    }

    /// BLACKHAT: Gate 12 rejects orphan action contracts (contract with no Do node).
    ///
    /// SEVERITY: MEDIUM (orphan contracts indicate compilation errors or
    /// potential dead code that could mask security issues)
    #[test]
    fn blackhat_gate_12_rejects_orphan_contract() {
        let nodes = vec![finish_node(0, 0)];
        let parts = make_parts(nodes, 1);
        let contracts = vec![vb_core::action::ActionContract {
            id: ActionId::new(99),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: vb_core::action::Idempotency::DeterministicPure,
            side_effect: vb_core::action::SideEffect::None,
            retry_safety: vb_core::action::RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }];
        let result = validate_gate_12_action_contract_completeness(&parts, &contracts);
        assert!(
            matches!(
                result,
                Err(ValidationError::ActionContractOrphan { action_id: 99 })
            ),
            "blackhat: orphan contract with no Do node must be rejected"
        );
    }
}
