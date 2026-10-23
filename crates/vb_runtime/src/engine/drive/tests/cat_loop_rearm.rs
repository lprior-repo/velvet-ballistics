#![forbid(unsafe_code)]

use super::common::{cn, fin, mkr, mkwfc, nop};
use crate::engine::drive::loop_rearm::loop_body_rearm;
use crate::engine::types::RuntimeEngineError;
use vb_core::engine::EngineError;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

fn fixture() -> Result<CompiledWorkflow, String> {
    let nodes = vec![
        cn(
            0,
            Some(1),
            None,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(2),
                limit: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        ),
        nop(1, 2),
        nop(2, 3),
        cn(
            3,
            Some(2),
            None,
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(1),
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        ),
        fin(4, 0),
    ];
    mkwfc(nodes, 4, vec![])
}

fn checked<T, E: std::fmt::Debug>(r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| format!("{e:?}"))
}

fn succeed(run: &mut RunFrame, id: u16) -> Result<(), String> {
    checked(run.mark_running(StepIdx::new(id)))?;
    checked(run.mark_succeeded(StepIdx::new(id)))
}

fn expect_core_reason(r: Result<(), RuntimeEngineError>, reason: &str) -> Result<(), String> {
    match r {
        Err(RuntimeEngineError::Core(EngineError::InvalidCompiledWorkflow { reason: actual }))
            if actual == reason =>
        {
            Ok(())
        }
        other => Err(format!("unexpected error: {other:?}")),
    }
}

#[test]
fn test_full_succeeded_loop_rearms() -> Result<(), String> {
    let wf = checked(fixture())?;
    let mut run = checked(mkr(5, 4))?;
    checked(run.set_pc(StepIdx::new(1)))?;
    checked(run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret))?;

    for id in [0u16, 1, 2, 3, 4] {
        succeed(&mut run, id)?;
    }

    checked(loop_body_rearm(
        &wf,
        &mut run,
        StepIdx::new(3),
        StepIdx::new(1),
    ))?;

    assert_eq!(
        checked(run.step_state(StepIdx::new(1)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(2)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(3)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(0)))?,
        StepState::Succeeded
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(4)))?,
        StepState::Succeeded
    );
    assert_eq!(
        checked(run.read_slot(SlotIdx::new(0)))?,
        &SlotValue::I64(99)
    );
    assert_eq!(checked(run.read_taint(SlotIdx::new(0)))?, Taint::Secret);
    assert_eq!(run.pc(), StepIdx::new(1));
    Ok(())
}

#[test]
fn test_bad_states_reject_rearm() -> Result<(), String> {
    let wf = checked(fixture())?;
    let bad_states = [
        StepState::Running,
        StepState::Waiting,
        StepState::Asking,
        StepState::Failed,
        StepState::Cancelled,
        StepState::Skipped,
    ];

    for state in bad_states {
        let mut run = checked(mkr(5, 4))?;
        checked(run.set_pc(StepIdx::new(1)))?;
        checked(run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret))?;
        succeed(&mut run, 1)?;

        match state {
            StepState::Running => {
                checked(run.mark_running(StepIdx::new(2)))?;
            }
            StepState::Waiting => {
                checked(run.mark_running(StepIdx::new(2)))?;
                checked(run.mark_waiting(StepIdx::new(2)))?;
            }
            StepState::Asking => {
                checked(run.mark_running(StepIdx::new(2)))?;
                checked(run.mark_asking(StepIdx::new(2)))?;
            }
            StepState::Failed => {
                checked(run.mark_running(StepIdx::new(2)))?;
                checked(run.mark_failed(StepIdx::new(2)))?;
            }
            StepState::Cancelled => {
                checked(run.mark_running(StepIdx::new(2)))?;
                checked(run.mark_cancelled(StepIdx::new(2)))?;
            }
            StepState::Skipped => {
                checked(run.mark_skipped(StepIdx::new(2)))?;
            }
            _ => {}
        }

        succeed(&mut run, 3)?;

        expect_core_reason(
            loop_body_rearm(&wf, &mut run, StepIdx::new(3), StepIdx::new(1)),
            "loop body step not in rearmable state",
        )?;

        assert_eq!(
            checked(run.step_state(StepIdx::new(1)))?,
            StepState::Succeeded
        );
        assert_eq!(checked(run.step_state(StepIdx::new(2)))?, state);
        assert_eq!(
            checked(run.step_state(StepIdx::new(3)))?,
            StepState::Succeeded
        );
        assert_eq!(
            checked(run.step_state(StepIdx::new(0)))?,
            StepState::Pending
        );
        assert_eq!(
            checked(run.step_state(StepIdx::new(4)))?,
            StepState::Pending
        );
        assert_eq!(
            checked(run.read_slot(SlotIdx::new(0)))?,
            &SlotValue::I64(99)
        );
        assert_eq!(checked(run.read_taint(SlotIdx::new(0)))?, Taint::Secret);
    }

    Ok(())
}

#[test]
fn test_pc_and_latch_requirements() -> Result<(), String> {
    let wf = checked(fixture())?;

    let mut run = checked(mkr(5, 4))?;
    checked(run.set_pc(StepIdx::new(4)))?;
    checked(run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret))?;
    succeed(&mut run, 3)?;
    expect_core_reason(
        loop_body_rearm(&wf, &mut run, StepIdx::new(3), StepIdx::new(1)),
        "loop rearm requires successful owned backedge",
    )?;
    assert_eq!(
        checked(run.step_state(StepIdx::new(1)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(3)))?,
        StepState::Succeeded
    );
    assert_eq!(
        checked(run.read_slot(SlotIdx::new(0)))?,
        &SlotValue::I64(99)
    );
    assert_eq!(checked(run.read_taint(SlotIdx::new(0)))?, Taint::Secret);

    let mut run2 = checked(mkr(5, 4))?;
    checked(run2.set_pc(StepIdx::new(1)))?;
    checked(run2.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret))?;
    succeed(&mut run2, 1)?;
    expect_core_reason(
        loop_body_rearm(&wf, &mut run2, StepIdx::new(3), StepIdx::new(1)),
        "loop rearm requires successful owned backedge",
    )?;
    assert_eq!(
        checked(run2.step_state(StepIdx::new(1)))?,
        StepState::Succeeded
    );
    assert_eq!(
        checked(run2.step_state(StepIdx::new(3)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run2.read_slot(SlotIdx::new(0)))?,
        &SlotValue::I64(99)
    );
    assert_eq!(checked(run2.read_taint(SlotIdx::new(0)))?, Taint::Secret);

    Ok(())
}

#[test]
fn test_pending_body_preserved() -> Result<(), String> {
    let wf = checked(fixture())?;
    let mut run = checked(mkr(5, 4))?;
    checked(run.set_pc(StepIdx::new(1)))?;
    checked(run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret))?;

    succeed(&mut run, 1)?;
    succeed(&mut run, 3)?;

    checked(loop_body_rearm(
        &wf,
        &mut run,
        StepIdx::new(3),
        StepIdx::new(1),
    ))?;

    assert_eq!(
        checked(run.step_state(StepIdx::new(1)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(2)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(3)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(0)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.step_state(StepIdx::new(4)))?,
        StepState::Pending
    );
    assert_eq!(
        checked(run.read_slot(SlotIdx::new(0)))?,
        &SlotValue::I64(99)
    );
    assert_eq!(checked(run.read_taint(SlotIdx::new(0)))?, Taint::Secret);
    assert_eq!(run.pc(), StepIdx::new(1));

    Ok(())
}
