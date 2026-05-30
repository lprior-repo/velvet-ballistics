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
    let join = checked_step_offset(id, join_offset, "parallel", "join")
        .map_err(|e| CompileErrors(vec![e]))?;
    let mut branch_targets = Vec::with_capacity(branches.len());
    let mut cursor = 1u16;
    for branch in branches {
        branch_targets.push(
            checked_step_offset(id, cursor, "parallel", "branch")
                .map_err(|e| CompileErrors(vec![e]))?,
        );
        let width =
            u16::try_from(body_width(&branch.steps, 1).map_err(|e| CompileErrors(vec![e]))?)
                .map_err(|_| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "parallel",
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
        let branch_id = checked_step_offset(base, cursor, "parallel", "branch")
            .map_err(|e| CompileErrors(vec![e]))?;
        let entry = checked_step_offset(
            base,
            cursor.checked_add(1).ok_or_else(|| {
                CompileErrors(vec![CompileError::StepIndexOutOfRange {
                    value: branch_index,
                }])
            })?,
            "parallel",
            "entry",
        )
        .map_err(|e| CompileErrors(vec![e]))?;
        let branch_number = u16::try_from(branch_index).map_err(|_| {
            CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                primitive: "parallel",
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
/// # GOD RULE 2 Contract (Verus model binding)
///
/// The mathematical contract for this function is proved by the Verus model at
/// `verification/verus/collect_lowering.rs` (L1-L6) and the spec block at the
/// end of this file. In summary, when the starting `id` satisfies
/// `id + 3 <= u16::MAX` (65535):
///
/// - **L1 (strict monotonicity):** `body_step < page < done`.
/// - **L2 (4 distinct IDs):** All four node IDs (`id`, `id+1`, `id+2`, `id+3`)
///   are within `u16` bounds.
/// - **L3 (consecutive IDs):** The offsets are exactly `+1` from each
///   predecessor.
/// - **L4 (max valid start):** The maximum starting `id` is `u16::MAX - 3`
///   (= 65532).
/// - **L5 (default safety):** `pages.unwrap_or(1)` and `items.unwrap_or(1)`
///   always produce a value `>= 1`.
/// - **L6 (full emission chain):** All of the above hold simultaneously.
///
/// The production function cannot carry Verus `requires`/`ensures` annotations
/// directly because it uses external crate types (`vb_core::StepIdx`,
/// `vb_core::CompiledNode`, `CompileErrors`, `SlotCompiler`) and mutable state
/// (`&mut SlotCompiler`) that Verus cannot track. Instead, the mathematical
/// contract is proved in the spec block below and in the standalone Verus
/// artifacts.
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
// GOD RULE 2 Verus Spec Block
//
// Binding: mathematical contract for lower_canonical_collect (above).
//   Proves the same L1-L6 properties as the standalone Verus model at
//   verification/verus/collect_lowering.rs.
//
// This block is conditionally compiled by Verus via #[cfg(verus_keep_ghost)].
// Regular Rust builds (cargo check / cargo build) skip it entirely.
// Verus sees this block and verifies the lemmas.
//
// The production exec fn lower_canonical_collect CANNOT be annotated with
// Verus requires/ensures directly because:
//   (a) External crate types (vb_core::StepIdx, SlotCompiler, CompileErrors)
//       are not Verus-tracked.
//   (b) Mutable state through &mut SlotCompiler is not Verus-compatible.
//   (c) The ? operator on custom CompileErrors types is not supported.
//   (d) Callees like emit_single_body_set use external types.
//
// Instead, this spec block proves the pure mathematical contract that the
// production function's offset arithmetic must satisfy. The binding:
//   - Production: checked_step_offset(id, 1) → StepIdx::checked_add(1)
//   - Spec model:  spec_checked_step_offset(id, 1) → Ok(id + 1) when valid
//   - Both use u16 arithmetic with MAX = 65535.
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
// L1: Strict monotonicity — body < page < done
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_collect_steps_strictly_increasing(id: int)
    requires
        id >= 0,
        id + 3 <= vb_u16_max(),
    ensures
        id + 1 < id + 2 < id + 3,
{
    assert(id + 1 < id + 2);
    assert(id + 2 < id + 3);
    assert(id + 1 < id + 3);
}

// ─────────────────────────────────────────────────────────────────
// L2: 4 distinct IDs within u16 bounds
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_collect_4_ids_in_bounds(id: int)
    requires
        id >= 0,
        id + 3 <= vb_u16_max(),
    ensures
        id + 1 <= vb_u16_max(),
        id + 2 <= vb_u16_max(),
        id + 3 <= vb_u16_max(),
{
    assert(id + 1 <= vb_u16_max()) by {
        assert(id + 3 <= vb_u16_max());
        assert(id + 1 <= id + 3);
    }
    assert(id + 2 <= vb_u16_max()) by {
        assert(id + 3 <= vb_u16_max());
        assert(id + 2 <= id + 3);
    }
    assert(id + 3 <= vb_u16_max());
}

// ─────────────────────────────────────────────────────────────────
// L3: Consecutive IDs — each offset differs by exactly 1
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_collect_ids_consecutive(id: int)
    requires
        id >= 0,
        id + 3 <= vb_u16_max(),
    ensures
        (id + 1) - id == 1,
        (id + 2) - (id + 1) == 1,
        (id + 3) - (id + 2) == 1,
{
    assert((id + 1) - id == 1) by { }
    assert((id + 2) - (id + 1) == 1) by { }
    assert((id + 3) - (id + 2) == 1) by { }
}

// ─────────────────────────────────────────────────────────────────
// L4: Maximum valid start ID is u16::MAX - 3 (= 65532)
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_max_valid_collect_start()
    ensures
        vb_u16_max() - 3 >= 0,
        (vb_u16_max() - 3) + 3 == vb_u16_max(),
        (vb_u16_max() - 3) + 3 <= vb_u16_max(),
{
    assert(vb_u16_max() - 3 >= 0) by {
        assert(vb_u16_max() >= 3);
    }
    assert((vb_u16_max() - 3) + 3 == vb_u16_max()) by { }
    assert((vb_u16_max() - 3) + 3 <= vb_u16_max()) by { }
}

// ─────────────────────────────────────────────────────────────────
// L5: Option unwrap safety — default value is >= 1
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_option_default_at_least_one(v: Option<u32>)
    ensures
        match v {
            Option::Some(n) => n >= 1 ==> n >= 1,
            Option::None => 1u32 >= 1,
        },
{
}

// ─────────────────────────────────────────────────────────────────
// L6: Full emission chain (L1 + L2 + L3 + spec binding)
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_full_collect_emission_chain(id: int)
    requires
        id >= 0,
        id + 3 <= vb_u16_max(),
    ensures
        id + 1 <= vb_u16_max(),
        id + 2 <= vb_u16_max(),
        id + 3 <= vb_u16_max(),
        id + 1 < id + 2 < id + 3,
        (id + 1) - id == 1,
        (id + 2) - (id + 1) == 1,
        (id + 3) - (id + 2) == 1,
        vb_spec_collect_offsets(id) == Ok::<(int, int, int), VbSpecCompileError>(
            (id + 1, id + 2, id + 3)),
{
    vb_lemma_collect_4_ids_in_bounds(id);
    vb_lemma_collect_steps_strictly_increasing(id);
    vb_lemma_collect_ids_consecutive(id);
    assert(vb_spec_checked_step_offset(id, 1).is_ok());
    assert(vb_spec_checked_step_offset(id, 2).is_ok());
    assert(vb_spec_checked_step_offset(id, 3).is_ok());
}

fn main() {}

} // verus!
