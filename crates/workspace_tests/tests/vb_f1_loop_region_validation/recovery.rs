#![forbid(unsafe_code)]

use super::support::*;

#[test]
fn internal_direct_recovery_routes_preserve_exact_loop_ownership() -> TestResult {
    for family in FAMILIES {
        let mut route = paired(family)?;
        edit(&mut route, 2)?.routing.error = ErrorRouting::Handler {
            handler: step(3),
            error_slot: None,
        };
        assert_region(&accept(route)?, 1, 2, 4, 5)?;
    }
    Ok(())
}

#[test]
fn internal_error_handler_wrappers_stay_typed_unsupported() -> TestResult {
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
        edit(&mut parts, 2)?.routing.error = ErrorRouting::Handler {
            handler: step(target),
            error_slot: None,
        };
        assert_region(&accept(parts)?, 1, 2, 4, 5)?;
    }
    Ok(())
}

#[test]
fn direct_error_to_internal_finish_does_not_recover_into_body() -> TestResult {
    let mut parts = linear_parts(7)?;
    install_pair(&mut parts, Family::ForEach, 1, 2, 5, 6, 1)?;
    let body = edit(&mut parts, 2)?;
    body.routing.next = NodeContinuation::Step(step(4));
    body.routing.error = ErrorRouting::Handler {
        handler: step(3),
        error_slot: None,
    };
    let terminal = edit(&mut parts, 3)?;
    terminal.kind = CompiledNodeKind::Finish { result: slot(0) };
    terminal.routing.next = NodeContinuation::End;
    assert_region(&accept(parts)?, 1, 2, 5, 6)
}

#[test]
fn forward_error_exit_preserves_ownership_and_the_declared_handler() -> TestResult {
    for family in FAMILIES {
        let mut parts = linear_parts(8)?;
        install_pair(&mut parts, family, 1, 2, 4, 5, 1)?;
        edit(&mut parts, 2)?.routing.error = ErrorRouting::Handler {
            handler: step(6),
            error_slot: None,
        };
        let plan = accept(parts)?;
        assert_region(&plan, 1, 2, 4, 5)?;
        assert_eq!(
            plan.node(step(2)).ok_or("body")?.routing.error,
            ErrorRouting::Handler {
                handler: step(6),
                error_slot: None,
            }
        );
    }
    Ok(())
}

#[test]
fn forward_error_exit_cannot_enter_a_sibling_body_or_latch() -> TestResult {
    for target in [6, 8] {
        let mut parts = linear_parts(10)?;
        install_pair(&mut parts, Family::ForEach, 1, 2, 4, 5, 1)?;
        install_pair(&mut parts, Family::Collect, 5, 6, 8, 9, 2)?;
        edit(&mut parts, 2)?.routing.error = ErrorRouting::Handler {
            handler: step(target),
            error_slot: None,
        };
        reject(parts, crossing(8, 2, target))?;
    }
    Ok(())
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
    action.routing.next = NodeContinuation::Step(step(3));
    edit(&mut parts, 2)?.kind = CompiledNodeKind::AskResume { answer: slot(3) };
    let plan = accept(parts)?;
    for id in 0..4 {
        assert!(plan.loop_region(step(id)).is_none());
    }
    Ok(())
}
