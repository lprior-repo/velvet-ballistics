use proptest::prelude::*;
use super::*;
use crate::workflow::{CompiledInputConstraint, CompiledInputSlot, InputSlotKind};

fn n(raw: u16, kind: K, out: Option<u16>, next: Option<u16>) -> CompiledNode {
    node(
        StepIdx::new(raw),
        kind,
        out.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

fn head_a() -> CompiledNode {
    n(
        0,
        K::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(2),
            limit: 4.into(),
            body: StepIdx::new(1),
            done: StepIdx::new(6),
        },
        Some(1),
        None,
    )
}

fn head_b() -> CompiledNode {
    n(
        1,
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(2)]),
            join: StepIdx::new(4),
        },
        None,
        None,
    )
}

fn head_c() -> CompiledNode {
    n(
        2,
        K::Copy {
            source: SlotIdx::new(2),
        },
        Some(1),
        Some(3),
    )
}

fn head_d() -> CompiledNode {
    n(
        3,
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(1),
            result: None,
        },
        None,
        Some(4),
    )
}

fn head() -> [CompiledNode; 4] {
    let a = head_a();
    let b = head_b();
    let c = head_c();
    let d = head_d();
    [a, b, c, d]
}

fn tail_e() -> CompiledNode {
    n(
        4,
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(1),
        },
        Some(3),
        Some(5),
    )
}

fn tail_f() -> CompiledNode {
    n(
        5,
        K::ForEachNext {
            iterator_slot: SlotIdx::new(1),
            body: StepIdx::new(1),
            done: StepIdx::new(6),
        },
        Some(2),
        None,
    )
}

fn tail_g() -> CompiledNode {
    n(
        6,
        K::ForEachJoin {
            output: SlotIdx::new(1),
        },
        Some(4),
        Some(7),
    )
}

fn tail_h() -> CompiledNode {
    n(
        7,
        K::Finish {
            result: SlotIdx::new(4),
        },
        None,
        None,
    )
}

fn tail() -> [CompiledNode; 4] {
    let e = tail_e();
    let f = tail_f();
    let g = tail_g();
    let h = tail_h();
    [e, f, g, h]
}

fn child_write_parts() -> crate::workflow::WorkflowParts {
    let mut p = build_parts();
    p.slot_count = 5;
    p.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::List,
        constraint: CompiledInputConstraint::required(),
    }]);
    let [a, b, c, d] = head();
    let [e, f, g, h] = tail();
    p.nodes = Box::new([a, b, c, d, e, f, g, h]);
    p
}

fn checked(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let regions = crate::workflow::concurrent_backedges::validated(parts)?;
    crate::workflow::cursor_ownership::validate(parts, &regions)
}

#[test]
fn child_copy_does_not_clobber_parent_cursor() -> Result<(), Box<dyn std::error::Error>> {
    let parts = child_write_parts();
    checked(&parts)?;
    crate::workflow::concurrent_analyze::analyze(&parts)?;
    Ok(())
}

#[test]
fn parent_join_cursor_write_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = child_write_parts();
    parts.nodes.get_mut(4).ok_or("fixture")?.output = Some(SlotIdx::new(1)).into();
    let expected = WorkflowError::LoopCursorForeignWrite {
        owner: StepIdx::new(0),
        writer: StepIdx::new(4),
        slot: SlotIdx::new(1),
    };
    assert_eq!(checked(&parts), Err(expected));
    Ok(())
}

#[test]
fn same_lane_foreign_cursor_write_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = super::legacy_fixtures::foreach_within_group();
    parts.nodes.get_mut(2).ok_or("fixture")?.output = Some(SlotIdx::new(1)).into();
    let expected = WorkflowError::LoopCursorForeignWrite {
        owner: StepIdx::new(1),
        writer: StepIdx::new(2),
        slot: SlotIdx::new(1),
    };
    assert_eq!(checked(&parts), Err(expected));
    Ok(())
}

proptest! {
    #[test]
    fn generated_lane_width_and_owner_preserve_exact_region_bounds(
        prefix in 0u16..=8,
        width in 1u16..=8,
    ) {
        let entry = prefix.checked_add(1).ok_or_else(|| TestCaseError::fail("entry overflow"))?;
        let lane_end = entry.checked_add(width).ok_or_else(|| TestCaseError::fail("lane end overflow"))?;
        let join = lane_end.checked_add(1).ok_or_else(|| TestCaseError::fail("join overflow"))?;
        let finish = join.checked_add(1).ok_or_else(|| TestCaseError::fail("finish overflow"))?;
        let mut parts = build_parts();
        parts.slot_count = 2;
        let mut nodes = Vec::with_capacity(usize::from(finish) + 1);
        for step in 0..prefix {
            let next = step.checked_add(1).ok_or_else(|| TestCaseError::fail("prefix overflow"))?;
            nodes.push(n(step, K::Copy { source: SlotIdx::new(0) }, Some(0), Some(next)));
        }
        nodes.push(n(prefix, K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(entry)]),
            join: StepIdx::new(join),
        }, None, None));
        for offset in 0..width {
            let step = entry.checked_add(offset).ok_or_else(|| TestCaseError::fail("body overflow"))?;
            let next = step.checked_add(1).ok_or_else(|| TestCaseError::fail("body edge overflow"))?;
            nodes.push(n(step, K::Copy { source: SlotIdx::new(0) }, Some(0), Some(next)));
        }
        nodes.push(n(lane_end, K::ConcurrentLaneEnd {
            owner: StepIdx::new(prefix),
            result: None,
        }, None, Some(join)));
        nodes.push(n(join, K::ConcurrentGroupJoin { owner: StepIdx::new(prefix) }, Some(1), Some(finish)));
        nodes.push(n(finish, K::Finish { result: SlotIdx::new(1) }, None, None));
        parts.nodes = nodes.into_boxed_slice();
        let regions = crate::workflow::concurrent_analyze::analyze(&parts)
            .map_err(|error| TestCaseError::fail(format!("{error:?}")))?;
        prop_assert_eq!(regions.len(), 1);
        let region = regions.first().ok_or_else(|| TestCaseError::fail("missing region"))?;
        prop_assert_eq!(region.owner, StepIdx::new(prefix));
        prop_assert_eq!(region.join, StepIdx::new(join));
        let lane = region.lanes.first().ok_or_else(|| TestCaseError::fail("missing lane"))?;
        prop_assert_eq!(lane.entry, StepIdx::new(entry));
        prop_assert_eq!(lane.exclusive_end, StepIdx::new(join));
        prop_assert_eq!(lane.ends.as_ref(), &[StepIdx::new(lane_end)]);
    }
}
