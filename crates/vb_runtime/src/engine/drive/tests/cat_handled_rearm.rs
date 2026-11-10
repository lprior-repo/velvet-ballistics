#![forbid(unsafe_code)]

use super::common::{cn, fin, mkr, mkwfc, nop};
use crate::engine::drive::loop_rearm::loop_body_rearm;
use crate::engine::types::RuntimeEngineError;
use vb_core::workflow::{CompiledWorkflow, ErrorRouting};
use vb_core::{CompiledNodeKind as K, CoreError, RunFrame, SlotIdx, StepIdx, StepState};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const CONTROL: StepIdx = StepIdx::new(4);
const BODY: StepIdx = StepIdx::new(1);

fn fixture(
    first_handler: Option<u16>,
    second_handler: Option<u16>,
) -> TestResult<CompiledWorkflow> {
    let mut nodes = vec![
        cn(
            0,
            Some(1),
            None,
            K::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(2),
                limit: 2,
                body: BODY,
                done: StepIdx::new(5),
            },
        ),
        nop(1, 2),
        nop(2, 3),
        nop(3, 4),
        cn(
            4,
            Some(2),
            None,
            K::ForEachNext {
                iterator_slot: SlotIdx::new(1),
                body: BODY,
                done: StepIdx::new(5),
            },
        ),
        fin(5, 0),
    ];
    for (id, handler) in [(1, first_handler), (2, second_handler)] {
        if let Some(handler) = handler {
            nodes.get_mut(id).ok_or("node")?.routing.error = ErrorRouting::Handler {
                handler: StepIdx::new(handler),
                error_slot: None,
            };
        }
    }
    Ok(mkwfc(nodes, 4, vec![], &[0])?)
}

fn frame(handler: StepState) -> TestResult<RunFrame> {
    let mut frame = mkr(6, 4)?;
    frame.mark_succeeded(StepIdx::new(0))?;
    frame.mark_failed(BODY)?;
    match handler {
        StepState::Succeeded => frame.mark_succeeded(StepIdx::new(2))?,
        StepState::Failed => frame.mark_failed(StepIdx::new(2))?,
        StepState::Pending => {}
        _ => return Err("unsupported test state".into()),
    }
    frame.mark_succeeded(StepIdx::new(3))?;
    frame.mark_succeeded(CONTROL)?;
    frame.set_pc(BODY)?;
    Ok(frame)
}

#[test]
fn handled_failure_rearm_requires_owned_backedge_and_completed_declared_handler() -> TestResult {
    for (handler, state) in [
        (None, StepState::Succeeded),
        (Some(2), StepState::Pending),
        (Some(2), StepState::Failed),
        (Some(5), StepState::Succeeded),
    ] {
        let plan = fixture(handler, None)?;
        let mut frame = frame(state)?;
        frame.mark_succeeded(StepIdx::new(5))?;
        let before = frame.clone();
        assert_eq!(
            frame.rearm_handled_failure(&plan, CONTROL, BODY),
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "handled failure lacks successful owned recovery",
            }),
        );
        assert_eq!(frame, before);
        assert_eq!(
            loop_body_rearm(&plan, &mut frame, CONTROL, BODY),
            Err(RuntimeEngineError::Core(
                CoreError::InvalidCompiledWorkflow {
                    reason: "loop body step not in rearmable state",
                }
            )),
        );
        assert_eq!(frame, before);
    }
    Ok(())
}

#[test]
fn general_failed_to_pending_transition_stays_forbidden() -> TestResult {
    let mut frame = frame(StepState::Succeeded)?;
    let before = frame.clone();
    assert_eq!(
        frame.mark_pending(BODY),
        Err(CoreError::InternalInvariantViolation {
            reason: "invalid_state_transition",
        })
    );
    assert_eq!(frame, before);
    for (pc, control, start) in [
        (StepIdx::new(2), CONTROL, true),
        (BODY, StepIdx::new(3), true),
        (BODY, CONTROL, false),
    ] {
        let plan = fixture(Some(2), None)?;
        let mut candidate = frame.clone();
        candidate.set_pc(pc)?;
        if !start {
            candidate.mark_pending(StepIdx::new(0))?;
        }
        let before = candidate.clone();
        assert_eq!(
            candidate.rearm_handled_failure(&plan, control, BODY),
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "handled failure lacks successful owned recovery",
            })
        );
        assert_eq!(candidate, before);
    }
    Ok(())
}

#[test]
fn chained_recovery_rearms_failed_nodes_before_their_handler_witnesses() -> TestResult {
    let plan = fixture(Some(2), Some(3))?;
    let mut frame = frame(StepState::Failed)?;
    loop_body_rearm(&plan, &mut frame, CONTROL, BODY)?;
    for id in 1..=4 {
        assert_eq!(frame.step_state(StepIdx::new(id))?, StepState::Pending);
    }
    assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Succeeded);
    assert_eq!(frame.step_state(StepIdx::new(5))?, StepState::Pending);
    assert_eq!(frame.pc(), BODY);
    Ok(())
}

#[test]
fn later_unhandled_failure_leaves_earlier_handled_failure_unchanged() -> TestResult {
    let plan = fixture(Some(3), None)?;
    let mut frame = frame(StepState::Failed)?;
    frame.write_slot_with_taint(
        SlotIdx::new(0),
        vb_core::SlotValue::I64(99),
        vb_core::Taint::Secret,
    )?;
    let before = frame.clone();
    assert_eq!(
        loop_body_rearm(&plan, &mut frame, CONTROL, BODY),
        Err(RuntimeEngineError::Core(
            CoreError::InvalidCompiledWorkflow {
                reason: "loop body step not in rearmable state",
            }
        ))
    );
    assert_eq!(frame, before);
    Ok(())
}

#[test]
fn pending_latch_never_authorizes_failed_reactivation() -> TestResult {
    let plan = fixture(Some(2), None)?;
    let mut frame = frame(StepState::Succeeded)?;
    frame.mark_pending(CONTROL)?;
    let before = frame.clone();
    assert_eq!(
        frame.rearm_handled_failure(&plan, CONTROL, BODY),
        Err(CoreError::InvalidCompiledWorkflow {
            reason: "handled failure lacks successful owned recovery",
        })
    );
    assert_eq!(frame, before);
    assert_eq!(
        loop_body_rearm(&plan, &mut frame, CONTROL, BODY),
        Err(RuntimeEngineError::Core(
            CoreError::InvalidCompiledWorkflow {
                reason: "loop rearm requires successful owned backedge",
            }
        ))
    );
    assert_eq!(frame, before);
    Ok(())
}

fn nested_fixture(handler: u16) -> TestResult<CompiledWorkflow> {
    let mut failed = nop(2, 3);
    failed.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(handler),
        error_slot: None,
    };
    Ok(mkwfc(
        vec![
            cn(
                0,
                Some(4),
                None,
                K::RepeatStart {
                    max_attempts: 2,
                    body: BODY,
                    done: StepIdx::new(8),
                },
            ),
            cn(
                1,
                Some(1),
                None,
                K::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(2),
                    limit: 2,
                    body: StepIdx::new(2),
                    done: StepIdx::new(5),
                },
            ),
            failed,
            nop(3, 4),
            cn(
                4,
                Some(2),
                None,
                K::ForEachNext {
                    iterator_slot: SlotIdx::new(1),
                    body: StepIdx::new(2),
                    done: StepIdx::new(5),
                },
            ),
            nop(5, 6),
            nop(6, 7),
            cn(
                7,
                None,
                Some(1),
                K::RepeatCheck {
                    attempt_slot: SlotIdx::new(4),
                    done: StepIdx::new(8),
                },
            ),
            fin(8, 0),
        ],
        5,
        vec![],
        &[0],
    )?)
}

fn nested_frame(failed: bool) -> TestResult<RunFrame> {
    let mut frame = mkr(9, 5)?;
    for raw in [0, 1, 6, 7] {
        frame.mark_succeeded(StepIdx::new(raw))?;
    }
    if failed {
        frame.mark_failed(StepIdx::new(2))?;
    } else {
        frame.mark_succeeded(StepIdx::new(2))?;
    }
    frame.set_pc(BODY)?;
    Ok(frame)
}

#[test]
fn abandoned_child_requires_actual_failed_exit_not_an_unrelated_successful_handler() -> TestResult {
    for (handler, failed) in [(6, false), (3, true)] {
        let plan = nested_fixture(handler)?;
        let mut frame = nested_frame(failed)?;
        if handler == 3 {
            frame.mark_succeeded(StepIdx::new(3))?;
        }
        let before = frame.clone();
        assert_eq!(
            loop_body_rearm(&plan, &mut frame, StepIdx::new(7), BODY),
            Err(RuntimeEngineError::Core(
                CoreError::InvalidCompiledWorkflow {
                    reason: "nested loop region not consistently completed",
                }
            ))
        );
        assert_eq!(frame, before);
    }
    Ok(())
}

#[test]
fn actual_completed_exit_rearms_abandoned_child_without_touching_pending_nodes_or_payload()
-> TestResult {
    let plan = nested_fixture(6)?;
    let mut frame = nested_frame(true)?;
    frame.write_slot_with_taint(
        SlotIdx::new(0),
        vb_core::SlotValue::I64(99),
        vb_core::Taint::Secret,
    )?;
    loop_body_rearm(&plan, &mut frame, StepIdx::new(7), BODY)?;
    for raw in 1..=8 {
        assert_eq!(frame.step_state(StepIdx::new(raw))?, StepState::Pending);
    }
    assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Succeeded);
    assert_eq!(
        frame.read_slot(SlotIdx::new(0))?,
        &vb_core::SlotValue::I64(99)
    );
    assert_eq!(frame.read_taint(SlotIdx::new(0))?, vb_core::Taint::Secret);
    assert_eq!(frame.pc(), BODY);
    Ok(())
}

#[test]
fn failed_child_start_without_its_own_declared_exit_cannot_borrow_another_nodes_handler()
-> TestResult {
    let plan = nested_fixture(6)?;
    let mut frame = mkr(9, 5)?;
    for raw in [0, 6, 7] {
        frame.mark_succeeded(StepIdx::new(raw))?;
    }
    frame.mark_failed(BODY)?;
    frame.set_pc(BODY)?;
    let before = frame.clone();
    assert_eq!(
        loop_body_rearm(&plan, &mut frame, StepIdx::new(7), BODY),
        Err(RuntimeEngineError::Core(
            CoreError::InvalidCompiledWorkflow {
                reason: "loop body step not in rearmable state",
            }
        ))
    );
    assert_eq!(frame, before);
    Ok(())
}
