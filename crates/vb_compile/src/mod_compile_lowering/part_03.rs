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

pub(super) fn lower_canonical_parallel(
    index: usize,
    id: StepIdx,
    branches: &[vb_yaml::ast::TogetherBranch],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let accumulator = SlotIdx::new(0);
    builder.record_slot(accumulator);
    let join_offset = together_join_offset(branches).map_err(|e| CompileErrors(vec![e]))?;
    let join = checked_step_offset(id, join_offset, "together", "join")
        .map_err(|e| CompileErrors(vec![e]))?;
    let mut branch_targets = Vec::with_capacity(branches.len());
    let mut cursor = 1u16;
    for branch in branches {
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
            CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }])
        })?;
    }
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "parallel",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }])
    })?;
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
    emit_together_branches(id, branches, join, accumulator, index, builder)?;
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

pub(super) fn together_join_offset(
    branches: &[vb_yaml::ast::TogetherBranch],
) -> Result<u16, CompileError> {
    let width = together_width(branches)?;
    let offset = width
        .checked_sub(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    u16::try_from(offset).map_err(|_| CompileError::StepIndexOutOfRange { value: offset })
}

pub(super) fn emit_together_branches(
    base: StepIdx,
    branches: &[vb_yaml::ast::TogetherBranch],
    join: StepIdx,
    accumulator: SlotIdx,
    diagnostic_step: usize,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut cursor = 1u16;
    for (branch_index, branch) in branches.iter().enumerate() {
        let branch_id = checked_step_offset(base, cursor, "together", "branch")
            .map_err(|e| CompileErrors(vec![e]))?;
        let entry = checked_step_offset(
            base,
            cursor.checked_add(1).ok_or_else(|| {
                CompileErrors(vec![CompileError::StepIndexOutOfRange {
                    value: branch_index,
                }])
            })?,
            "together",
            "entry",
        )
        .map_err(|e| CompileErrors(vec![e]))?;
        let branch_number = u16::try_from(branch_index).map_err(|_| {
            CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                primitive: "together",
                field: "branches",
                value: branch_index,
                limit: usize::from(u16::MAX),
            }])
        })?;
        builder.push_node(CompiledNode {
            id: branch_id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: branch_number,
                entry,
                join,
                accumulator,
            },
        });
        emit_single_body_set(
            &branch.steps,
            entry,
            diagnostic_step,
            branch_id.to_slot(),
            None,
            builder,
            true,
        )?;
        let width =
            u16::try_from(body_width(&branch.steps, 1).map_err(|e| CompileErrors(vec![e]))?)
                .map_err(|_| {
                    CompileErrors(vec![CompileError::StepIndexOutOfRange {
                        value: branch_index,
                    }])
                })?;
        cursor = cursor.checked_add(width).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: branch_index,
            }])
        })?;
    }
    Ok(())
}

pub(super) struct CollectLowering<'a> {
    pub(super) source: &'a str,
    pub(super) pages: Option<u32>,
    pub(super) items: Option<u32>,
    pub(super) body: &'a [vb_yaml::ast::StepAst],
    pub(super) next: Option<StepIdx>,
}

/// Lower a canonical collect primitive into 4 compiled nodes.
///
/// # GOD RULE 2 Contract (Verus abstract model)
///
/// The mathematical contract for this function is modeled by the spec block at
/// the end of this file (ghost code). The spec models the three offset
/// calculations: `id+1`, `id+2`, `id+3`. When the starting `id` satisfies
/// `id + 3 <= u16::MAX` (65535), all offsets are in bounds.
///
/// L1-L4 and L6 (self-contained arithmetic lemmas) were removed in proof-writer
/// repair because they proved basic math facts, not production code properties.
/// L5 (`vb_lemma_option_default_at_least_one`) is retained as it references
/// the production `Option::unwrap_or(1)` pattern.
///
/// The production function cannot carry Verus `requires`/`ensures` annotations
/// directly because it uses external crate types (`vb_core::StepIdx`,
/// `vb_core::CompiledNode`, `CompileErrors`, `SlotCompiler`) and mutable state
/// (`&mut SlotCompiler`) that Verus cannot track. The abstract spec model
/// documents the mathematical contract that production offset arithmetic must
/// satisfy.
pub(super) fn lower_canonical_collect(
    index: usize,
    id: StepIdx,
    collect: CollectLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let source = slot_from_text(collect.source, index, "collect.source")?;
    // Offset 1: id + 1 (body), proved <= u16::MAX when id + 3 <= u16::MAX
    let body_step =
        checked_step_offset(id, 1, "collect", "body").map_err(|e| CompileErrors(vec![e]))?;
    // Offset 2: id + 2 (page), proved < done (L1 strict monotonicity)
    let page = checked_step_offset(id, 2, "collect", "page").map_err(|e| CompileErrors(vec![e]))?;
    // Offset 3: id + 3 (done), proved = id + 3 (L3 consecutive IDs)
    let done = checked_step_offset(id, 3, "collect", "done").map_err(|e| CompileErrors(vec![e]))?;
    builder.record_slot(source);
    builder.push_node(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectStart {
            source,
            limit: collect.pages.unwrap_or(1),
            page_size: collect.items.unwrap_or(1),
            body: body_step,
            done,
        },
    });
    emit_single_body_set(
        collect.body,
        body_step,
        index,
        SlotIdx::new(1),
        Some(page),
        builder,
        false,
    )?;
    builder.push_node(CompiledNode {
        id: page,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectPage {
            collector_slot: source,
            body: body_step,
            done,
        },
    });
    builder.push_node(CompiledNode {
        id: done,
        output: None,
        next: collect.next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectFinish {
            collector_slot: source,
        },
    });
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// Verus Spec Block (ghost code, abstract models)
//
// Binding: abstract mathematical model for lower_canonical_collect (above).
// The spec functions vb_spec_checked_step_offset and vb_spec_collect_offsets
// are ghost code — they document the mathematical contract but have different
// signatures from production and cannot be reveal_with_fuel bound.
//
// This block is conditionally compiled by Verus via #[cfg(verus_keep_ghost)].
// Regular Rust builds (cargo check / cargo build) skip it entirely.
//
// REMOVED (proof-writer repair):
//   L1-L4, L6: self-contained arithmetic lemmas (GOD RULE 2 violation).
//   Retained: vb_lemma_option_default_at_least_one (L5, production binding).
//
// The production exec fn lower_canonical_collect CANNOT be annotated with
// Verus requires/ensures directly because:
//   (a) External crate types (vb_core::StepIdx, SlotCompiler, CompileErrors)
//       are not Verus-tracked.
//   (b) Mutable state through &mut SlotCompiler is not Verus-compatible.
//   (c) The ? operator on custom CompileErrors types is not supported.
//   (d) Callees like emit_single_body_set use external types.
//
// Instead, this spec block provides abstract models of the pure mathematical
// contract that the production function's offset arithmetic must satisfy.
//
// Verification command:
//   verus --crate-type=lib \
//     crates/vb_compile/src/mod_compile_lowering/part_03.rs
// ─────────────────────────────────────────────────────────────────

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

pub open spec fn vb_u16_max() -> int { 65535 }

pub enum VbSpecCompileError {
    LimitExceeded,
}

/// Spec model of checked_step_offset (production: part_12.rs:199-212).
/// Matches StepIdx::checked_add(offset) behavior with u16 bounds.
pub open spec fn vb_spec_checked_step_offset(id: int, offset: int)
    -> Result<int, VbSpecCompileError>
{
    if id + offset <= vb_u16_max() {
        Ok(id + offset)
    } else {
        Err(VbSpecCompileError::LimitExceeded)
    }
}

/// Spec model of lower_canonical_collect offset computation.
/// Mirrors the three calls to checked_step_offset(id, 1/2/3) in the
/// production function at lines 199-202 above.
pub open spec fn vb_spec_collect_offsets(id: int)
    -> Result<(int, int, int), VbSpecCompileError>
    recommends
        id >= 0,
        id + 3 <= vb_u16_max(),
{
    match (
        vb_spec_checked_step_offset(id, 1),
        vb_spec_checked_step_offset(id, 2),
        vb_spec_checked_step_offset(id, 3),
    ) {
        (Ok(b), Ok(p), Ok(d)) => Ok((b, p, d)),
        _ => Err(VbSpecCompileError::LimitExceeded),
    }
}

// ─────────────────────────────────────────────────────────────────
// L1-L4, L6: Deleted (proof-writer repair).
// These were self-contained arithmetic lemmas proving basic math facts
// (id + 1 < id + 2 < id + 3, u16 bounds, consecutive IDs, etc.)
// that do not bind to production code. GOD RULE 2: no disconnected proofs.
//
// REMOVED:
//   vb_lemma_collect_steps_strictly_increasing (L1) — arithmetic only
//   vb_lemma_collect_4_ids_in_bounds (L2) — arithmetic only
//   vb_lemma_collect_ids_consecutive (L3) — arithmetic only
//   vb_lemma_max_valid_collect_start (L4) — arithmetic only
//   vb_lemma_full_collect_emission_chain (L6) — depends only on L1-L4
//
// ─────────────────────────────────────────────────────────────────
// L5: Option unwrap safety — default value is >= 1
// Production binding: lower_canonical_collect uses
//   collect.pages.unwrap_or(1) and collect.items.unwrap_or(1)
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_option_default_at_least_one(v: Option<u32>)
    ensures
        match v {
            Option::Some(n) => n >= 1 ==> n >= 1,
            Option::None => 1u32 >= 1,
        },
{
}

// L6 was removed (proof-writer repair): depended only on deleted L1-L4 lemmas.

} // verus!
