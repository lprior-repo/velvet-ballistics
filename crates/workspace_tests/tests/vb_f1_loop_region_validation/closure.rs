#![forbid(unsafe_code)]

use super::support::*;

#[test]
fn both_choose_forms_and_fallback_cannot_enter_body_or_latch() -> TestResult {
    for expressions in [false, true] {
        for target in [2, 4] {
            for fallback in [false, true] {
                let mut parts = paired(Family::ForEach)?;
                edit(&mut parts, 0)?.kind = if fallback {
                    choose(1, target, expressions)
                } else {
                    choose(target, 1, expressions)
                };
                reject(parts, crossing(4, 0, target))?;
            }
        }
    }
    Ok(())
}

#[test]
fn only_declared_start_body_edge_is_permitted_ingress() -> TestResult {
    for from in [0, 1] {
        let mut parts = paired(Family::ForEach)?;
        edit(&mut parts, from)?.on_error = Some(step(2));
        reject(parts, crossing(4, from, 2))?;
    }
    let mut redundant_next = paired(Family::ForEach)?;
    edit(&mut redundant_next, 1)?.next = Some(step(2));
    reject(redundant_next, crossing(4, 1, 2))
}

#[test]
fn jump_and_async_next_cannot_enter_an_owned_region() -> TestResult {
    let mut jump = paired(Family::ForEach)?;
    edit(&mut jump, 0)?.kind = CompiledNodeKind::Jump { target: step(4) };
    reject(jump, crossing(4, 0, 4))?;
    let mut resume = paired(Family::ForEach)?;
    let outside = edit(&mut resume, 0)?;
    outside.kind = CompiledNodeKind::AskResume { answer: slot(3) };
    outside.next = Some(step(2));
    outside.on_error = Some(step(1));
    reject(resume, crossing(4, 0, 2))
}

#[test]
fn auxiliary_kind_successors_cannot_enter_a_region() -> TestResult {
    let kinds = [
        CompiledNodeKind::CollectPage {
            collector_slot: slot(2),
            body: step(1),
            done: step(4),
        },
        CompiledNodeKind::RepeatAttempt {
            attempt_slot: slot(2),
            body: step(1),
            done: step(4),
        },
        CompiledNodeKind::RetryCheck {
            policy_slot: slot(2),
            body: step(1),
            exhausted: step(4),
        },
        CompiledNodeKind::ErrorHandler {
            body: step(1),
            handler: step(4),
            error_slot: None,
        },
        CompiledNodeKind::TogetherStart {
            branches: vec![step(1), step(4)].into_boxed_slice(),
            join: step(5),
        },
        CompiledNodeKind::TogetherBranch {
            branch: 0,
            entry: step(1),
            join: step(4),
            accumulator: slot(2),
        },
    ];
    for kind in kinds {
        let mut parts = paired(Family::ForEach)?;
        edit(&mut parts, 0)?.kind = kind;
        reject(parts, crossing(4, 0, 4))?;
    }
    Ok(())
}

#[test]
fn arbitrary_nonterminal_forward_escape_is_not_an_exit() -> TestResult {
    for expressions in [false, true] {
        let mut parts = linear_parts(8)?;
        install_pair(&mut parts, Family::ForEach, 1, 2, 4, 5, 1)?;
        edit(&mut parts, 2)?.kind = choose(3, 6, expressions);
        reject(parts, crossing(4, 2, 6))?;
    }
    Ok(())
}

#[test]
fn owning_done_and_actual_finish_are_permitted_exits() -> TestResult {
    for target in [5, 7] {
        let mut parts = linear_parts(8)?;
        install_pair(&mut parts, Family::ForEach, 1, 2, 4, 5, 1)?;
        edit(&mut parts, 2)?.kind = choose(3, target, false);
        assert_region(&accept(parts)?, 1, 2, 4, 5)?;
    }
    Ok(())
}

#[test]
fn internal_auxiliary_backedges_are_not_nested_owned_latches() -> TestResult {
    let kinds = [
        CompiledNodeKind::CollectPage {
            collector_slot: slot(2),
            body: step(2),
            done: step(5),
        },
        CompiledNodeKind::RepeatAttempt {
            attempt_slot: slot(2),
            body: step(2),
            done: step(5),
        },
        CompiledNodeKind::RetryCheck {
            policy_slot: slot(2),
            body: step(2),
            exhausted: step(5),
        },
        CompiledNodeKind::ErrorHandler {
            body: step(2),
            handler: step(5),
            error_slot: None,
        },
    ];
    for kind in kinds {
        let mut parts = paired(Family::ForEach)?;
        edit(&mut parts, 2)?.kind = kind;
        reject(parts, crossing(4, 2, 2))?;
    }
    Ok(())
}

#[test]
fn crossing_region_intervals_are_not_wholly_nested() -> TestResult {
    let mut parts = linear_parts(10)?;
    install_pair(&mut parts, Family::ForEach, 1, 3, 6, 9, 1)?;
    install_pair(&mut parts, Family::Collect, 2, 4, 7, 8, 2)?;
    edit(&mut parts, 0)?.kind = choose(1, 2, false);
    edit(&mut parts, 6)?.next = Some(step(7));
    reject(parts, crossing(7, 4, 7))
}

#[test]
fn shared_body_is_not_independent_nested_ownership() -> TestResult {
    let mut parts = linear_parts(9)?;
    install_pair(&mut parts, Family::ForEach, 1, 3, 5, 8, 1)?;
    install_pair(&mut parts, Family::Collect, 2, 3, 6, 7, 2)?;
    edit(&mut parts, 0)?.kind = choose(1, 2, false);
    edit(&mut parts, 5)?.next = Some(step(6));
    reject(parts, crossing(6, 3, 6))
}
