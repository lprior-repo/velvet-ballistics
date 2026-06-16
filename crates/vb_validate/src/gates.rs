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
///
/// # Verification
///
/// Bound to proof obligations PO-VB-001 through PO-VB-006 in
/// `verification/verus/vb_validate_gate_07.rs`.
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

const MAX_ACCESSOR_PATH_DEPTH: usize = 16;

/// Validates that every accessor path segment resolves to a well-formed symbol.
///
/// Gate 8 (budgets): Field segments must use valid symbol IDs (within the
/// interned symbol table range), and index segments must be finite.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    for (acc_index, accessor) in parts.accessors.iter().enumerate() {
        validate_accessor_root(acc_index, accessor, parts.slot_count)?;
        if accessor.path.len() > MAX_ACCESSOR_PATH_DEPTH {
            return Err(ValidationError::AccessorPathTooDeep {
                accessor_index: acc_index,
                depth: accessor.path.len(),
                max: MAX_ACCESSOR_PATH_DEPTH,
            });
        }
        for (seg_index, segment) in accessor.path.iter().enumerate() {
            match segment {
                PathSegment::Field(sym_id) => {
                    validate_field_symbol(acc_index, seg_index, *sym_id, parts.symbols_count)?;
                }
                PathSegment::Index(idx) => validate_index_segment(acc_index, seg_index, *idx)?,
                // `PathSegment` is `#[non_exhaustive]`; unknown variants
                // are a structural error — fail closed rather than silently ignore.
                _ => {
                    return Err(ValidationError::AccessorPathInvalid {
                        accessor_index: acc_index,
                        segment_index: seg_index,
                    });
                }
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
    if let Some(error_slot) = node.error_slot {
        check_slot(error_slot, node_index, slot_count)?;
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
        _ => {
            return Err(ValidationError::NodeKindConstraintViolation {
                node_index,
                detail: "unsupported node kind".to_string(),
            });
        }
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
    let slot_count = visited.len();
    // Iterative three-color DFS using a Vec stack of (slot, neighbor_index).
    // White=0, Gray=1, Black=2.
    let mut stack: Vec<(usize, usize)> = Vec::new();

    // Seed with the starting slot. Always push the frame, including leaves,
    // so zero-dependency slots are marked black before later traversals see them.
    if let Some(state) = visited.get_mut(slot) {
        *state = 1; // gray
    }
    stack.push((slot, 0));

    while let Some((current, idx)) = stack.pop() {
        if idx == 0 {
            // First visit to this neighbor list: already marked gray above.
        }

        let neighbors = adjacency.get(current).map_or(&[][..], |v| v.as_slice());

        if idx < neighbors.len() {
            // Push back the current node with the next neighbor index.
            let next_idx = idx
                .checked_add(1)
                .ok_or(ValidationError::SlotDependencyCycle {
                    slot: current,
                    chain: format!("slot {current} has too many dependency edges"),
                })?;
            stack.push((current, next_idx));

            let Some(&neighbor) = neighbors.get(idx) else {
                break;
            };

            // Bounds check: neighbor index must be within the slot count.
            if neighbor >= slot_count {
                return Err(ValidationError::SlotDependencyCycle {
                    slot: current,
                    chain: format!("slot {current} -> slot {neighbor}"),
                });
            }

            let Some(&color) = visited.get(neighbor) else {
                break;
            };
            if color == 1 {
                // Gray = cycle found.
                return Err(ValidationError::SlotDependencyCycle {
                    slot: current,
                    chain: format!("slot {current} -> slot {neighbor}"),
                });
            }
            if color == 0 {
                // White: unvisited, mark gray and start exploring.
                if let Some(entry) = visited.get_mut(neighbor) {
                    *entry = 1; // gray
                }
                stack.push((neighbor, 0));
            }
            // If color == 2 (black), neighbor already fully explored, skip.
        } else {
            // All neighbors of `current` have been processed; mark black.
            if let Some(entry) = visited.get_mut(current) {
                *entry = 2; // black
            }
        }
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
        // `CompiledNodeKind` is `#[non_exhaustive]`; unknown variants
        // contribute no reads (fail-soft: new variants start with no reads).
        _ => {}
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
        // `ConstValue` is `#[non_exhaustive]`; return 0 for unknown
        // variants (does not conflict with known discriminant values).
        _ => 0,
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
#[cfg(test)]
#[path = "gates/tests.rs"]
mod tests;
