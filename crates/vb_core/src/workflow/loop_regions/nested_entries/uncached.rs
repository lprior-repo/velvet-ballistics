#![forbid(unsafe_code)]

use super::builders::{control_kind, outer_parts, start_kind};
use super::support::*;

#[test]
fn unpaired_nested_loop_starts_are_flagged_without_inventing_bodies() -> TestResult {
    for family in FAMILIES {
        let mut parts = outer_parts(9, 7, 8)?;
        let start = edit(&mut parts, 2)?;
        start.kind = start_kind(family, 3, 6);
        start.output = Some(slot(2));
        start.next = None;
        check_flags(parts, &[2])?;
    }
    Ok(())
}

#[test]
fn forward_only_nested_controls_do_not_expand_start_only_flags() -> TestResult {
    for family in FAMILIES {
        let mut parts = outer_parts(10, 8, 9)?;
        let start = edit(&mut parts, 2)?;
        start.kind = start_kind(family, 3, 7);
        start.output = Some(slot(2));
        start.next = None;
        let control = edit(&mut parts, 3)?;
        // A forward-only control is not the Start's validated counter latch.
        control.kind = control_kind(family, 5, 4, 7);
        control.next = matches!(family, Family::Repeat).then_some(step(4));
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
    edit(&mut parts, 3)?.kind = CompiledNodeKind::TogetherBranch {
        branch: 0,
        entry: step(4),
        join: step(6),
        accumulator: slot(2),
    };
    edit(&mut parts, 6)?.kind = CompiledNodeKind::TogetherJoin {
        branch_count: 1,
        accumulator: slot(2),
    };
    check_flags(parts, &[2])
}

#[test]
fn uncached_starts_outside_owned_regions_are_not_blocked() -> TestResult {
    for family in FAMILIES {
        let mut parts = linear_parts(6)?;
        let start = edit(&mut parts, 1)?;
        start.kind = start_kind(family, 2, 5);
        start.output = Some(slot(2));
        start.next = None;
        check_flags(parts, &[])?;
    }
    Ok(())
}
