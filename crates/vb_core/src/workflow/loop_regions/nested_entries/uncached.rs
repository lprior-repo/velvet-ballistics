#![forbid(unsafe_code)]

use super::builders::{control_kind, outer_parts, start_kind};
use super::support::*;

#[test]
fn unpaired_nested_loop_starts_are_flagged_without_inventing_bodies() -> TestResult {
    for family in FAMILIES {
        let mut parts = outer_parts(9, 7, 8)?;
        let start = edit(&mut parts, 2)?;
        start.kind = start_kind(family, 3, 6);
        start.output = NodeOutput::Slot(slot(2));
        start.routing.next = NodeContinuation::End;
        check_flags(parts, &[2])?;
    }
    Ok(())
}

#[test]
fn forward_only_nested_controls_do_not_expand_start_only_flags() -> TestResult {
    for family in FAMILIES {
        let mut parts = outer_parts(10, 8, 9)?;
        parts.input_slots = Box::new([
            crate::workflow::CompiledInputSlot {
                slot: slot(0),
                kind: crate::workflow::InputSlotKind::Any,
                constraint: crate::workflow::CompiledInputConstraint::required(),
            },
            crate::workflow::CompiledInputSlot {
                slot: slot(5),
                kind: if matches!(family, Family::Repeat) {
                    crate::workflow::InputSlotKind::I64
                } else {
                    crate::workflow::InputSlotKind::List
                },
                constraint: crate::workflow::CompiledInputConstraint::required(),
            },
        ]);
        let start = edit(&mut parts, 2)?;
        start.kind = start_kind(family, 3, 7);
        start.output = NodeOutput::Slot(slot(2));
        start.routing.next = NodeContinuation::End;
        let control = edit(&mut parts, 3)?;
        // A forward-only control is not the Start's validated counter latch.
        control.kind = control_kind(family, 5, 4, 7);
        control.output =
            NodeOutput::from(matches!(family, Family::ForEach | Family::Reduce).then_some(slot(3)));
        control.routing.next =
            NodeContinuation::from(matches!(family, Family::Repeat).then_some(step(4)));
        check_flags(parts, &[2])?;
    }
    Ok(())
}

#[test]
fn nested_together_flags_start_only_not_branch_body_or_join() -> TestResult {
    let mut parts = outer_parts(9, 7, 8)?;
    edit(&mut parts, 2)?.kind = CompiledNodeKind::TogetherStart {
        branches: vec![step(3)].into_boxed_slice(),
        join: step(6),
    };
    edit(&mut parts, 2)?.output = NodeOutput::Slot(slot(2));
    edit(&mut parts, 3)?.kind = CompiledNodeKind::TogetherBranch {
        branch: 0.into(),
        entry: step(4),
        join: step(6),
        accumulator: slot(2),
    };
    edit(&mut parts, 6)?.kind = CompiledNodeKind::TogetherJoin {
        branch_count: 1.into(),
        accumulator: slot(2),
    };
    edit(&mut parts, 6)?.output = NodeOutput::Slot(slot(0));
    check_flags(parts, &[2])
}

#[test]
fn uncached_starts_outside_owned_regions_are_not_blocked() -> TestResult {
    for family in FAMILIES {
        let mut parts = linear_parts(6)?;
        let start = edit(&mut parts, 1)?;
        start.kind = start_kind(family, 2, 5);
        start.output = NodeOutput::Slot(slot(2));
        start.routing.next = NodeContinuation::End;
        check_flags(parts, &[])?;
    }
    Ok(())
}
