#![forbid(unsafe_code)]
use super::*;

struct Emission<'a> {
    base: StepIdx,
    join: StepIdx,
    slots: TogetherSlots,
    diagnostic_step: usize,
    scope: CanonicalScope<'a>,
}

struct Start {
    id: StepIdx,
    entry: StepIdx,
    number: u16,
}

struct Tail {
    following: StepIdx,
    following_offset: u16,
}

pub(in crate::mod_compile_lowering) fn emit_together_branches(
    base: StepIdx,
    branches: &[crate::TogetherBranch],
    join: StepIdx,
    slots: TogetherSlots,
    diagnostic_step: usize,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let emission = Emission {
        base,
        join,
        slots,
        diagnostic_step,
        scope,
    };
    let mut cursor = 1u16;
    for (index, branch) in branches.iter().enumerate() {
        cursor = emit_one(&emission, branch, cursor, index, builder)?;
    }
    Ok(())
}

fn emit_one(
    emission: &Emission<'_>,
    branch: &crate::TogetherBranch,
    cursor: u16,
    index: usize,
    builder: &mut SlotCompiler,
) -> Result<u16, CompileErrors> {
    let start = branch_start(emission, cursor, index)?;
    let tail = branch_tail(emission, branch, cursor, index)?;
    emit_header(emission, &start, builder);
    emit_body(emission, &start, &tail, branch, builder)?;
    Ok(tail.following_offset)
}

fn branch_start(
    emission: &Emission<'_>,
    cursor: u16,
    index: usize,
) -> Result<Start, CompileErrors> {
    let id = checked_step_offset(emission.base, cursor, "parallel", "branch")?;
    let entry_offset = add_offset(cursor, 1, index)?;
    let entry = checked_step_offset(emission.base, entry_offset, "parallel", "entry")?;
    let number = branch_number(index)?;
    Ok(Start { id, entry, number })
}

fn branch_tail(
    emission: &Emission<'_>,
    branch: &crate::TogetherBranch,
    cursor: u16,
    index: usize,
) -> Result<Tail, CompileErrors> {
    let following_offset = add_offset(cursor, branch_width(branch, index)?, index)?;
    let following = checked_step_offset(emission.base, following_offset, "parallel", "next")?;
    Ok(Tail {
        following,
        following_offset,
    })
}

fn add_offset(cursor: u16, width: u16, index: usize) -> Result<u16, CompileErrors> {
    cursor
        .checked_add(width)
        .ok_or_else(|| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }]))
}

fn branch_width(branch: &crate::TogetherBranch, index: usize) -> Result<u16, CompileErrors> {
    u16::try_from(body_width(&branch.steps, 2)?)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }]))
}

fn branch_number(index: usize) -> Result<u16, CompileErrors> {
    u16::try_from(index).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "parallel",
            field: "branches",
            value: index,
            limit: usize::from(u16::MAX),
        }])
    })
}

fn emit_header(emission: &Emission<'_>, start: &Start, builder: &mut SlotCompiler) {
    builder.push_node(node(
        start.id,
        CompiledNodeKind::TogetherBranch {
            branch: start.number.into(),
            entry: start.entry,
            join: emission.join,
            accumulator: emission.slots.accumulator,
        },
        (start.number != 0).then_some(emission.slots.result),
        None,
    ));
}

fn copy_step(emission: &Emission<'_>, tail: &Tail) -> Result<StepIdx, CompileErrors> {
    let offset = tail.following_offset.checked_sub(1).ok_or_else(|| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: emission.diagnostic_step,
        }])
    })?;
    Ok(checked_step_offset(
        emission.base,
        offset,
        "together",
        "result",
    )?)
}

fn emit_body(
    emission: &Emission<'_>,
    start: &Start,
    tail: &Tail,
    branch: &crate::TogetherBranch,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let copy = copy_step(emission, tail)?;
    let result = super::super::part_14::emit_scoped_flat_body(
        &branch.steps,
        start.entry,
        copy,
        emission.scope,
        builder,
        "together",
    )?;
    super::super::part_14::emit_body_result(
        copy,
        result,
        emission.slots.result,
        tail.following,
        builder,
    )
}
