#![forbid(unsafe_code)]

use super::support::*;

#[test]
fn four_families_cache_exact_owner_and_exclusive_endpoints() -> TestResult {
    for family in FAMILIES {
        let plan = accept(paired(family)?)?;
        assert_region(&plan, 1, 2, 4, 5)?;
        for other in [0, 1, 2, 3, 5, u16::MAX] {
            assert!(plan.loop_region(step(other)).is_none());
        }
    }
    Ok(())
}

#[test]
fn collect_legacy_source_slot_fallback_is_a_real_owner() -> TestResult {
    let mut parts = paired(Family::Collect)?;
    let owner = edit(&mut parts, 1)?;
    owner.output = None;
    owner.kind = CompiledNodeKind::CollectStart {
        source: slot(1),
        limit: 2,
        page_size: 1,
        body: step(2),
        done: step(5),
    };
    assert_region(&accept(parts)?, 1, 2, 4, 5)
}

#[test]
fn cache_is_derived_without_changing_serialized_parts() -> TestResult {
    let parts = paired(Family::ForEach)?;
    let bytes = postcard::to_allocvec(&parts).map_err(|error| format!("encode: {error}"))?;
    let plan = accept(parts.clone())?;
    assert_eq!(plan.to_parts(), parts);
    assert_eq!(
        postcard::to_allocvec(&plan.to_parts()).map_err(|error| format!("encode: {error}"))?,
        bytes
    );
    let decoded = postcard::from_bytes(&bytes).map_err(|error| format!("decode: {error}"))?;
    assert_region(&accept(decoded)?, 1, 2, 4, 5)
}

#[test]
fn forward_only_controls_remain_uncached() -> TestResult {
    for family in FAMILIES {
        let mut parts = linear_parts(6)?;
        let node = edit(&mut parts, 1)?;
        node.kind = control_kind(family, 1, 2, 5);
        node.next = if matches!(family, Family::Repeat) {
            Some(step(2))
        } else {
            None
        };
        let plan = accept(parts)?;
        assert!(plan.loop_region(step(1)).is_none());
    }
    Ok(())
}

#[test]
fn sequential_regions_have_separate_cache_entries() -> TestResult {
    let mut parts = linear_parts(10)?;
    install_pair(&mut parts, Family::ForEach, 1, 2, 4, 5, 1)?;
    install_pair(&mut parts, Family::Collect, 5, 6, 8, 9, 2)?;
    let plan = accept(parts)?;
    assert_region(&plan, 1, 2, 4, 5)?;
    assert_region(&plan, 5, 6, 8, 9)
}

#[test]
fn wholly_nested_regions_cache_independently_validated_backedges() -> TestResult {
    let mut parts = linear_parts(9)?;
    install_pair(&mut parts, Family::ForEach, 1, 2, 7, 8, 1)?;
    install_pair(&mut parts, Family::Collect, 2, 3, 5, 6, 2)?;
    let plan = accept(parts)?;
    assert_region(&plan, 2, 3, 5, 6)?;
    assert_region(&plan, 1, 2, 7, 8)
}

#[test]
fn internal_forward_choose_and_jump_bypass_are_closed() -> TestResult {
    for expressions in [false, true] {
        let mut parts = linear_parts(8)?;
        install_pair(&mut parts, Family::ForEach, 1, 2, 6, 7, 1)?;
        edit(&mut parts, 2)?.kind = choose(3, 4, expressions);
        let jump = edit(&mut parts, 3)?;
        jump.kind = CompiledNodeKind::Jump { target: step(5) };
        jump.next = None;
        assert_region(&accept(parts)?, 1, 2, 6, 7)?;
    }
    Ok(())
}

#[test]
fn together_topology_inside_region_is_not_activation_permission() -> TestResult {
    let mut parts = linear_parts(9)?;
    install_pair(&mut parts, Family::ForEach, 1, 2, 7, 8, 1)?;
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
    assert_region(&accept(parts)?, 1, 2, 7, 8)
}

#[test]
fn asynchronous_body_continuations_are_checked_without_blanket_rejection() -> TestResult {
    let kinds = [
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: slot(0),
        },
        CompiledNodeKind::WaitUntil {
            deadline_slot: slot(0),
        },
        CompiledNodeKind::WaitEvent {
            event: slot(0),
            timeout_slot: Some(slot(2)),
        },
        CompiledNodeKind::Ask {
            prompt: slot(0),
            timeout_slot: Some(slot(2)),
        },
        CompiledNodeKind::AskResume { answer: slot(3) },
    ];
    for kind in kinds {
        let mut parts = paired(Family::ForEach)?;
        edit(&mut parts, 2)?.kind = kind;
        assert_region(&accept(parts)?, 1, 2, 4, 5)?;
    }
    Ok(())
}
