#![forbid(unsafe_code)]

use super::support::*;

#[test]
fn missing_wrong_family_and_wrong_state_owners_reject() -> TestResult {
    for mismatch in 0..3 {
        let mut parts = paired(Family::ForEach)?;
        let owner = edit(&mut parts, 1)?;
        match mismatch {
            0 => {
                owner.kind = CompiledNodeKind::Nop;
                owner.next = Some(step(2));
            }
            1 => owner.kind = start_kind(Family::Collect, 2, 5),
            _ => owner.output = Some(slot(2)),
        }
        reject(parts, unpaired(4))?;
    }
    Ok(())
}

#[test]
fn mismatched_body_done_and_reduce_accumulator_reject() -> TestResult {
    let mut body = paired(Family::ForEach)?;
    edit(&mut body, 4)?.kind = control_kind(Family::ForEach, 1, 3, 5);
    reject(body, unpaired(4))?;
    let mut done = linear_parts(7)?;
    install_pair(&mut done, Family::ForEach, 1, 2, 4, 5, 1)?;
    edit(&mut done, 4)?.kind = control_kind(Family::ForEach, 1, 2, 6);
    reject(done, unpaired(4))?;
    let mut accumulator = paired(Family::Reduce)?;
    edit(&mut accumulator, 4)?.kind = CompiledNodeKind::ReduceNext {
        iterator_slot: slot(1),
        accumulator: slot(3),
        body: step(2),
        done: step(5),
    };
    reject(accumulator, unpaired(4))
}

#[test]
fn ambiguous_matching_starts_reject() -> TestResult {
    let mut parts = linear_parts(7)?;
    install_pair(&mut parts, Family::ForEach, 1, 3, 5, 6, 1)?;
    let other = edit(&mut parts, 2)?;
    other.kind = start_kind(Family::ForEach, 3, 6);
    other.output = Some(slot(1));
    other.next = None;
    edit(&mut parts, 0)?.kind = choose(1, 2, false);
    reject(parts, unpaired(5))
}

#[test]
fn two_enabled_latches_cannot_share_one_owner() -> TestResult {
    let mut parts = linear_parts(7)?;
    install_pair(&mut parts, Family::ForEach, 1, 2, 4, 6, 1)?;
    edit(&mut parts, 4)?.next = Some(step(5));
    edit(&mut parts, 5)?.kind = control_kind(Family::ForEach, 1, 2, 6);
    reject(parts, unpaired(5))
}

#[test]
fn self_control_is_not_a_forward_only_control() -> TestResult {
    let mut parts = paired(Family::ForEach)?;
    let owner = edit(&mut parts, 1)?;
    owner.kind = start_kind(Family::ForEach, 4, 5);
    owner.next = Some(step(2));
    edit(&mut parts, 4)?.kind = control_kind(Family::ForEach, 1, 4, 5);
    reject(parts, unpaired(4))
}

#[test]
fn workflow_entry_inside_body_or_latch_preserves_unreachable_precedence() -> TestResult {
    for entry in [2, 4] {
        let mut parts = paired(Family::ForEach)?;
        parts.entry = step(entry);
        reject(parts, WorkflowError::UnreachableNode { step: step(0) })?;
    }
    Ok(())
}

#[test]
fn jump_backward_guard_keeps_its_earlier_error() -> TestResult {
    let mut parts = paired(Family::ForEach)?;
    edit(&mut parts, 3)?.kind = CompiledNodeKind::Jump { target: step(2) };
    reject(
        parts,
        WorkflowError::BackwardEdge {
            from: step(3),
            to: step(2),
        },
    )
}

#[test]
fn numeric_bounds_keep_their_earlier_error() -> TestResult {
    let mut parts = paired(Family::ForEach)?;
    edit(&mut parts, 2)?.next = Some(step(99));
    reject(parts, WorkflowError::StepOutOfBounds { step: step(99) })
}
