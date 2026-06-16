#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};
use saphyr::Yaml;
use std::collections::HashMap;
use vb_core::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
    ExprIdx, ExprProgram, ResourceContract, SlotBranch, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowError, WorkflowParts,
};

pub(super) fn lower_canonical_aggregate(
    index: usize,
    id: StepIdx,
    input: &str,
    initial: &str,
    body: &[vb_yaml::ast::StepAst],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let input = slot_from_text(input, index, "aggregate.input")?;
    let accumulator = SlotIdx::new(1);
    let initial = parse_i64_field(initial, index, "reduce.initial")?;
    let initial = builder
        .push_constant(ConstValue::I64(initial))
        .map_err(|e| CompileErrors(vec![e]))?;
    let body_step =
        checked_step_offset(id, 1, "reduce", "body").map_err(|e| CompileErrors(vec![e]))?;
    let body_total_width = body_width(body, 0).map_err(|e| CompileErrors(vec![e]))?;
    let body_total_width_u16 = u16::try_from(body_total_width)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }]))?;
    let next_step = checked_step_offset(body_step, body_total_width_u16, "reduce", "next")
        .map_err(|e| CompileErrors(vec![e]))?;
    let done =
        checked_step_offset(next_step, 1, "reduce", "done").map_err(|e| CompileErrors(vec![e]))?;
    builder.record_slot(input);
    builder.record_slot(accumulator);
    builder.push_node(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body: body_step,
            done,
        },
    });
    emit_reduce_body_steps(
        body,
        body_step,
        index,
        accumulator,
        Some(next_step),
        builder,
    )?;
    builder.push_node(CompiledNode {
        id: next_step,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceNext {
            iterator_slot: accumulator,
            accumulator,
            body: body_step,
            done,
        },
    });
    builder.push_node(CompiledNode {
        id: done,
        output: None,
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceFinish { accumulator },
    });
    Ok(())
}

pub(super) fn lower_canonical_repeat(
    index: usize,
    id: StepIdx,
    max_attempts: u16,
    body: &[vb_yaml::ast::StepAst],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if max_attempts == 0 {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "repeat.max_attempts",
            expected: "non-empty primitive field",
        }]));
    }
    let body_step =
        checked_step_offset(id, 1, "repeat", "body").map_err(|e| CompileErrors(vec![e]))?;
    let attempt =
        checked_step_offset(id, 2, "repeat", "attempt").map_err(|e| CompileErrors(vec![e]))?;
    let done = checked_step_offset(id, 3, "repeat", "done").map_err(|e| CompileErrors(vec![e]))?;
    let attempt_slot = SlotIdx::new(1);
    builder.record_slot(attempt_slot);
    builder.push_node(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatStart {
            max_attempts,
            body: body_step,
            done,
        },
    });
    emit_single_body_set(
        body,
        body_step,
        index,
        SlotIdx::new(1),
        Some(attempt),
        builder,
        false,
    )?;
    builder.push_node(CompiledNode {
        id: attempt,
        output: Some(attempt_slot),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body: body_step,
            done,
        },
    });
    builder.push_node(CompiledNode {
        id: done,
        output: None,
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatFinish {
            result: attempt_slot,
        },
    });
    Ok(())
}

pub(super) fn lower_canonical_wait(
    index: usize,
    id: StepIdx,
    event: Option<&str>,
    timeout: Option<&str>,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut node = match (event, timeout) {
        (Some(event_text), timeout_text) => {
            let event = slot_from_text(event_text, index, "wait.event")?;
            let timeout = optional_slot_from_text(timeout_text, index, "wait.timeout")?;
            lower_wait(id, WaitKind::Event { event, timeout }, builder)
        }
        (None, Some(timeout_text)) => {
            let deadline = slot_from_text(timeout_text, index, "wait.timeout")?;
            lower_wait(id, WaitKind::Until { deadline }, builder)
        }
        (None, None) => {
            return Err(CompileErrors(vec![CompileError::StepFieldShape {
                step: index,
                field: "wait",
                expected: "event or timeout",
            }]));
        }
    };
    node.next = next;
    builder.push_node(node);
    Ok(())
}

pub(super) fn lower_canonical_ask(
    index: usize,
    id: StepIdx,
    prompt: &str,
    timeout: Option<&str>,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let prompt = slot_from_text(prompt, index, "ask.prompt")?;
    let timeout = optional_slot_from_text(timeout, index, "ask.timeout")?;
    let answer = SlotIdx::new(2);
    let mut nodes =
        lower_ask(id, prompt, answer, timeout, builder).map_err(|e| CompileErrors(vec![e]))?;
    let resume = nodes
        .get(1)
        .map(|node| node.id)
        .ok_or_else(|| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }]))?;
    if let Some(node) = nodes.get_mut(0) {
        node.next = Some(resume);
    }
    if let Some(node) = nodes.get_mut(1) {
        node.next = next;
    }
    for node in nodes {
        builder.push_node(node);
    }
    Ok(())
}

/// Emits a single body step (Set, Do, ForEach, or Together).
///
/// Specification:
/// - `body.len() == 1` required, otherwise `Err(StepFieldShape)`
/// - `Set` ⇒ emits 1 node (lower_set)
/// - `Do` ⇒ emits 1 node (CompiledNode::Do)
/// - `ForEach` ⇒ calls lower_canonical_for_each (recursive, width >= 2)
/// - `Together` ⇒ calls emit_single_body_together (recursive, width = 2 + body_width_sum)
/// - Other ⇒ Err(UnsupportedStepPrimitive)
///
/// # Node Count Property
/// For Set and Do, exactly 1 node is emitted. For ForEach, width >= 2.
/// For Together, emitted nodes == together_width(branches).
/// The `debug_assert_eq!` at line ~308 verifies this parity in debug builds.
pub(crate) fn emit_single_body_set(
    body: &[vb_yaml::ast::StepAst],
    id: StepIdx,
    diagnostic_step: usize,
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
    reuse_first_constant: bool,
) -> Result<(), CompileErrors> {
    if body.len() != 1 {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: "steps",
            expected: "exactly one set step",
        }]));
    }
    let step = body.first().ok_or_else(|| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: "steps",
            expected: "one set step",
        }])
    })?;
    match &step.primitive {
        vb_yaml::ast::StepPrimitive::Set { value, .. } => {
            let constant =
                body_constant_index(builder, value, diagnostic_step, reuse_first_constant)?;
            builder.record_slot(slot);
            builder.push_node(lower_set(id, slot, constant, next));
            Ok(())
        }
        vb_yaml::ast::StepPrimitive::Do { action, input } => {
            let action_value = action.parse::<i64>().map_err(|_| {
                CompileErrors(vec![CompileError::StepFieldShape {
                    step: diagnostic_step,
                    field: "do.action",
                    expected: "integer action id",
                }])
            })?;
            let action_id = u16::try_from(action_value).map_err(|_| {
                CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "do",
                    field: "action",
                    value: integer_error_value(action_value),
                    limit: usize::from(u16::MAX),
                }])
            })?;
            let input_value = input.parse::<i64>().map_err(|_| {
                CompileErrors(vec![CompileError::StepFieldShape {
                    step: diagnostic_step,
                    field: "do.input",
                    expected: "integer slot index",
                }])
            })?;
            let input_idx = u16::try_from(input_value).map_err(|_| {
                CompileErrors(vec![CompileError::SlotIndexOutOfRange {
                    value: input_value,
                }])
            })?;
            let input_slot = SlotIdx::new(input_idx);
            builder.record_slot(input_slot);
            builder.push_node(CompiledNode {
                id,
                output: None,
                next,
                error_slot: None,
                on_error: None,
                kind: CompiledNodeKind::Do {
                    action: vb_core::ActionId::new(action_id),
                    input: input_slot,
                },
            });
            Ok(())
        }
        vb_yaml::ast::StepPrimitive::ForEach {
            input,
            at_once,
            body,
            ..
        } => lower_canonical_for_each(diagnostic_step, id, input, *at_once, body, builder),
        vb_yaml::ast::StepPrimitive::Together { branches } => {
            // C-3: Width-node parity defense-in-depth (TH-1 mitigation).
            // debug_assert_eq! is supplemental documentation in debug builds;
            // the release path still relies on checked arithmetic and typed errors.
            let nodes_before = builder.nodes.len();
            let result =
                emit_single_body_together(branches, id, diagnostic_step, slot, next, builder);
            let nodes_after = builder.nodes.len();
            if result.is_ok()
                && let Ok(expected_width) = canonical_body_step_width(&step.primitive)
            {
                let emitted_width = nodes_after.saturating_sub(nodes_before);
                #[cfg(not(kani))]
                debug_assert_eq!(
                    expected_width, emitted_width,
                    "together width mismatch: computed {}, emitted {}",
                    expected_width, emitted_width,
                );
                #[cfg(kani)]
                let _ = (expected_width, emitted_width);
            }
            result
        }
        other => Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: diagnostic_step,
                primitive: canonical_primitive_name(other),
            },
        ])),
    }
}

pub(super) fn emit_reduce_body_steps(
    body: &[vb_yaml::ast::StepAst],
    body_step: StepIdx,
    diagnostic_step: usize,
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if body.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: "steps",
            expected: "at least one body step",
        }]));
    }
    let step_count = body.len();
    let mut cumulative_offset = 0u16;
    for (i, step) in body.iter().enumerate() {
        let step_id = checked_step_offset(body_step, cumulative_offset, "reduce", "body")
            .map_err(|e| CompileErrors(vec![e]))?;
        let step_width =
            canonical_body_step_width(&step.primitive).map_err(|e| CompileErrors(vec![e]))?;
        let step_width_u16 = u16::try_from(step_width).map_err(|_| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: diagnostic_step,
            }])
        })?;
        let next_index = i.checked_add(1).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: diagnostic_step,
            }])
        })?;
        let step_next = if next_index < step_count {
            let next_offset = cumulative_offset
                .checked_add(step_width_u16)
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::StepIndexOutOfRange {
                        value: diagnostic_step,
                    }])
                })?;
            Some(
                checked_step_offset(body_step, next_offset, "reduce", "body")
                    .map_err(|e| CompileErrors(vec![e]))?,
            )
        } else {
            next
        };
        match &step.primitive {
            vb_yaml::ast::StepPrimitive::Set { value, .. } => {
                let constant = body_constant_index(builder, value, diagnostic_step, false)?;
                builder.record_slot(slot);
                builder.push_node(lower_set(step_id, slot, constant, step_next));
            }
            vb_yaml::ast::StepPrimitive::Do { action, input } => {
                let action_value = action.parse::<i64>().map_err(|_| {
                    CompileErrors(vec![CompileError::StepFieldShape {
                        step: diagnostic_step,
                        field: "do.action",
                        expected: "integer action id",
                    }])
                })?;
                let action_id = u16::try_from(action_value).map_err(|_| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "do",
                        field: "action",
                        value: integer_error_value(action_value),
                        limit: usize::from(u16::MAX),
                    }])
                })?;
                let input_value = input.parse::<i64>().map_err(|_| {
                    CompileErrors(vec![CompileError::StepFieldShape {
                        step: diagnostic_step,
                        field: "do.input",
                        expected: "integer slot index",
                    }])
                })?;
                let input_idx = u16::try_from(input_value).map_err(|_| {
                    CompileErrors(vec![CompileError::SlotIndexOutOfRange {
                        value: input_value,
                    }])
                })?;
                let input_slot = SlotIdx::new(input_idx);
                builder.record_slot(input_slot);
                builder.push_node(CompiledNode {
                    id: step_id,
                    output: None,
                    next: step_next,
                    error_slot: None,
                    on_error: None,
                    kind: CompiledNodeKind::Do {
                        action: vb_core::ActionId::new(action_id),
                        input: input_slot,
                    },
                });
            }
            vb_yaml::ast::StepPrimitive::ForEach {
                input,
                at_once,
                body: foreach_body,
                ..
            } => lower_canonical_for_each(
                diagnostic_step,
                step_id,
                input,
                *at_once,
                foreach_body,
                builder,
            )?,
            vb_yaml::ast::StepPrimitive::Reduce {
                input,
                initial,
                body: reduce_inner_body,
                ..
            } => lower_canonical_aggregate(
                diagnostic_step,
                step_id,
                input,
                initial,
                reduce_inner_body,
                step_next,
                builder,
            )?,
            other => {
                return Err(CompileErrors(vec![
                    CompileError::UnsupportedStepPrimitive {
                        step: diagnostic_step,
                        primitive: canonical_primitive_name(other),
                    },
                ]));
            }
        }
        cumulative_offset = cumulative_offset
            .checked_add(step_width_u16)
            .ok_or_else(|| {
                CompileErrors(vec![CompileError::StepIndexOutOfRange {
                    value: diagnostic_step,
                }])
            })?;
    }
    Ok(())
}

/// Lowers a nested `Together` primitive appearing inside a compound body position.
///
/// Specification:
/// 1. `branches` must be non-empty, otherwise `Err(StepFieldShape)`
/// 2. Emits exactly `together_width(branches)` nodes:
///    - 1 TogetherStart node
///    - For each branch: 1 TogetherBranch node + body_width(branch.steps, 1) nodes
///    - 1 TogetherJoin node
/// 3. Total emitted = 2 + sum(body_width(b.steps, 1) for b in branches)
/// 4. Branch target StepIdx values are strictly increasing from `id`
/// 5. TogetherJoin has StepIdx = id + width - 1
/// 6. Inner Together nodes are contiguous (depth-first recursion, no interleaving)
///
/// # Width-Node Parity (TH-1 defense)
/// The emitted node count equals the value returned by `canonical_body_step_width(Together{..})`.
/// This is verified by the debug_assert_eq! in the caller (`emit_single_body_set`).
///
/// # Ordering Invariant
/// TogetherStart < TogetherBranch[0] < ... < TogetherBranch[n-1] < TogetherJoin
/// This is guaranteed by sequential for-loop emission with monotonically increasing StepIdx.
///
/// # Recursion Depth
/// Nested Together primitives recurse through emit_single_body_set → emit_single_body_together.
/// Each nested Together reduces the remaining YAML depth by at least 1.
/// Termination is guaranteed by YAML parser's depth limit (default 128).
pub(super) fn emit_single_body_together(
    branches: &[vb_yaml::ast::TogetherBranch],
    id: StepIdx,
    diagnostic_step: usize,
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if branches.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: "together.branches",
            expected: "at least one branch",
        }]));
    }
    let accumulator = slot;
    builder.record_slot(accumulator);
    let join_offset = together_join_offset(branches).map_err(|e| CompileErrors(vec![e]))?;
    let join = checked_step_offset(id, join_offset, "together", "join")
        .map_err(|e| CompileErrors(vec![e]))?;

    // Collect branch target StepIdx values.
    let mut branch_targets = Vec::with_capacity(branches.len());
    let mut cursor = 1u16;
    for branch in branches.iter() {
        branch_targets.push(
            checked_step_offset(id, cursor, "together", "branch")
                .map_err(|e| CompileErrors(vec![e]))?,
        );
        let width =
            u16::try_from(body_width(&branch.steps, 1).map_err(|e| CompileErrors(vec![e]))?)
                .map_err(|_| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "together",
                        field: "branches",
                        value: branches.len(),
                        limit: usize::from(u16::MAX),
                    }])
                })?;
        cursor = cursor.checked_add(width).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: diagnostic_step,
            }])
        })?;
    }

    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "together",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }])
    })?;

    // TogetherStart
    builder.push_node(CompiledNode {
        id,
        output: Some(accumulator),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: branch_targets.into_boxed_slice(),
            join,
        },
    });

    // Emit each branch with its body.
    emit_together_branches(id, branches, join, accumulator, diagnostic_step, builder)?;

    // TogetherJoin
    builder.push_node(CompiledNode {
        id: join,
        output: Some(accumulator),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        },
    });

    Ok(())
}

pub(super) fn body_constant_index(
    builder: &mut SlotCompiler,
    value: &str,
    step: usize,
    reuse_first_constant: bool,
) -> Result<ConstIdx, CompileErrors> {
    if reuse_first_constant && !builder.constants.is_empty() {
        return Ok(ConstIdx::new(0));
    }
    let constant = parse_i64_field(value, step, "set.value")?;
    builder
        .push_constant(ConstValue::I64(constant))
        .map_err(|e| CompileErrors(vec![e]))
}
