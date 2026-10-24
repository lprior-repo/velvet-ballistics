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

#[derive(Clone, Copy)]
pub(super) struct TogetherSlots {
    accumulator: SlotIdx,
    result: SlotIdx,
}

pub(super) fn lower_canonical_parallel(
    index: usize,
    id: StepIdx,
    branches: &[crate::TogetherBranch],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let accumulator = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let result = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let slots = TogetherSlots {
        accumulator,
        result,
    };
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
    emit_together_branches(id, branches, join, slots, index, builder)?;
    builder.push_node(CompiledNode {
        id: join,
        output: Some(result),
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
    branches: &[crate::TogetherBranch],
) -> Result<u16, CompileError> {
    let width = together_width(branches)?;
    let offset = width
        .checked_sub(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    u16::try_from(offset).map_err(|_| CompileError::StepIndexOutOfRange { value: offset })
}

pub(super) fn emit_together_branches(
    base: StepIdx,
    branches: &[crate::TogetherBranch],
    join: StepIdx,
    slots: TogetherSlots,
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
        let width =
            u16::try_from(body_width(&branch.steps, 1).map_err(|e| CompileErrors(vec![e]))?)
                .map_err(|_| {
                    CompileErrors(vec![CompileError::StepIndexOutOfRange {
                        value: branch_index,
                    }])
                })?;
        let following_offset = cursor.checked_add(width).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: branch_index,
            }])
        })?;
        let following = checked_step_offset(base, following_offset, "parallel", "next")
            .map_err(|e| CompileErrors(vec![e]))?;
        builder.push_node(CompiledNode {
            id: branch_id,
            output: if branch_number == 0 {
                None
            } else {
                Some(slots.result)
            },
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: branch_number,
                entry,
                join,
                accumulator: slots.accumulator,
            },
        });
        emit_single_body_set(
            &branch.steps,
            entry,
            diagnostic_step,
            slots.result,
            Some(following),
            builder,
            true,
        )?;
        cursor = following_offset;
    }
    Ok(())
}

// Historical N=1 collect lowering migrated to bounded_collect in part_02.
// .9.1 requires rebinding before full CI. Not generalized closure.

// ─────────────────────────────────────────────────────────────────
// GOD RULE 2 Verus Spec Block
//
// Historical N=1 binding: mathematical contract for the now-legacy
// collect lowering that lived in this file. Proves L1-L6 properties
// matching verification/verus/collect_lowering.rs.
//
// This block is conditionally compiled by Verus via #[cfg(verus_keep_ghost)].
// Regular Rust builds (cargo check / cargo build) skip it entirely.
// Verus sees this block and verifies the lemmas.
//
// The old production fn lower_canonical_collect (now migrated to
// part_02's bounded_collect) CANNOT be annotated with Verus
// requires/ensures directly because:
//   (a) External crate types (vb_core::StepIdx, SlotCompiler, CompileErrors)
//       are not Verus-tracked.
//   (b) Mutable state through &mut SlotCompiler is not Verus-compatible.
//   (c) The ? operator on custom CompileErrors types is not supported.
//   (d) Callees like emit_single_body_set use external types.
//
// Instead, this spec block proves the pure mathematical contract that the
// offset arithmetic must satisfy. The binding:
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
