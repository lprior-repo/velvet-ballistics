#![forbid(unsafe_code)]

use super::support::*;

#[test]
fn internal_recovered_error_routes_are_typed_unsupported() -> TestResult {
    let mut route = paired(Family::ForEach)?;
    edit(&mut route, 2)?.on_error = Some(step(3));
    reject(
        route,
        WorkflowError::LoopRegionUnsupported {
            control: step(4),
            step: step(2),
        },
    )?;
    let mut handler = paired(Family::ForEach)?;
    edit(&mut handler, 2)?.kind = CompiledNodeKind::ErrorHandler {
        body: step(3),
        handler: step(5),
        error_slot: None,
    };
    reject(
        handler,
        WorkflowError::LoopRegionUnsupported {
            control: step(4),
            step: step(2),
        },
    )
}

#[test]
fn direct_error_exits_to_done_or_finish_are_supported() -> TestResult {
    for target in [5, 7] {
        let mut parts = linear_parts(8)?;
        install_pair(&mut parts, Family::ForEach, 1, 2, 4, 5, 1)?;
        edit(&mut parts, 2)?.on_error = Some(step(target));
        assert_region(&accept(parts)?, 1, 2, 4, 5)?;
    }
    Ok(())
}

#[test]
fn direct_error_to_internal_finish_does_not_recover_into_body() -> TestResult {
    let mut parts = linear_parts(7)?;
    install_pair(&mut parts, Family::ForEach, 1, 2, 5, 6, 1)?;
    let body = edit(&mut parts, 2)?;
    body.next = Some(step(4));
    body.on_error = Some(step(3));
    let terminal = edit(&mut parts, 3)?;
    terminal.kind = CompiledNodeKind::Finish { result: slot(0) };
    terminal.next = None;
    assert_region(&accept(parts)?, 1, 2, 5, 6)
}

#[test]
fn crossing_error_exit_stays_a_geometric_error() -> TestResult {
    let mut parts = linear_parts(8)?;
    install_pair(&mut parts, Family::ForEach, 1, 2, 4, 5, 1)?;
    edit(&mut parts, 2)?.on_error = Some(step(6));
    reject(parts, crossing(4, 2, 6))
}

#[test]
fn unrelated_error_and_suspend_topology_is_not_blanket_rejected() -> TestResult {
    let mut parts = linear_parts(4)?;
    edit(&mut parts, 0)?.kind = CompiledNodeKind::ErrorHandler {
        body: step(1),
        handler: step(2),
        error_slot: None,
    };
    let action = edit(&mut parts, 1)?;
    action.kind = CompiledNodeKind::Do {
        action: ActionId::new(0),
        input: slot(0),
    };
    action.next = Some(step(3));
    edit(&mut parts, 2)?.kind = CompiledNodeKind::AskResume { answer: slot(3) };
    let plan = accept(parts)?;
    for id in 0..4 {
        assert!(plan.loop_region(step(id)).is_none());
    }
    Ok(())
}
