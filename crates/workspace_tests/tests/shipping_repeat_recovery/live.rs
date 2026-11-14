//! Live Repeat transitions: bounded attempts, terminal state and result taint.

use vb_core::{SlotValue, StepState, Taint};

use crate::support::{
    SOURCE, TAINT_SOURCE, TestResult, counter_sequence, drive_live, persist_reopen, repeat_nodes,
    started,
};

const SINGLE_SOURCE: &str = "version: velvet-ballistics/v1\nname: shipping_repeat_single\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: base, value: '7'}\n  - id: r\n    repeat:\n      max_attempts: 1\n      steps:\n        - id: leaf\n          set: {output: leaf_out, value: '9'}\n  - id: done\n    finish: {result: '$step.r'}\n";

#[test]
fn repeat_live_drive_runs_body_max_times_then_finishes_once() -> TestResult {
    let (workflow, frame) = drive_live(SOURCE, None, 9)?;
    let nodes = repeat_nodes(&workflow, 3)?;
    assert_eq!(frame.step_state(nodes.start)?, StepState::Succeeded);
    assert_eq!(frame.step_state(nodes.control)?, StepState::Succeeded);
    assert_eq!(frame.step_state(nodes.finish)?, StepState::Succeeded);
    assert_eq!(frame.pc(), nodes.terminal);
    assert_eq!(
        frame.read_slot(nodes.attempt)?,
        &SlotValue::I64(0x0003_0000_0003)
    );
    assert_eq!(frame.read_slot(nodes.result)?, &SlotValue::I64(9));
    assert_eq!(frame.read_taint(nodes.result)?, Taint::Clean);
    Ok(())
}

#[test]
fn repeat_single_attempt_limit_runs_body_once() -> TestResult {
    let (workflow, frame) = drive_live(SINGLE_SOURCE, None, 9)?;
    let nodes = repeat_nodes(&workflow, 1)?;
    assert_eq!(
        frame.read_slot(nodes.attempt)?,
        &SlotValue::I64(0x0001_0000_0001)
    );
    assert_eq!(frame.read_slot(nodes.result)?, &SlotValue::I64(9));
    assert_eq!(frame.pc(), nodes.terminal);

    let (_, events) = persist_reopen(SINGLE_SOURCE)?;
    assert_eq!(started(&events, nodes.body), 1);
    assert_eq!(started(&events, nodes.control), 1);
    assert_eq!(started(&events, nodes.finish), 1);
    assert_eq!(
        counter_sequence(&events, nodes.attempt)?,
        vec![(1, 0), (1, 1)]
    );
    Ok(())
}

#[test]
fn repeat_finish_preserves_body_result_taint() -> TestResult {
    let workflow = vb_compile::compile_workflow(TAINT_SOURCE.as_bytes())?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or("secret input slot")?
        .slot;
    let (_, frame) = drive_live(
        TAINT_SOURCE,
        Some((input, SlotValue::I64(41), Taint::Secret)),
        41,
    )?;
    let nodes = repeat_nodes(&workflow, 2)?;
    assert_eq!(frame.read_slot(nodes.result)?, &SlotValue::I64(41));
    assert_eq!(frame.read_taint(nodes.result)?, Taint::Secret);
    Ok(())
}
