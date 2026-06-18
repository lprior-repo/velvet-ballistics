#![forbid(unsafe_code)]
//! Kani harnesses for `lower_repeat`, `lower_ask`, and `lower_choose` control lowering.
//! Bead: vb-onsk/vb-awhr; scope: current public `vb_compile` APIs.

use crate::{CompileError, SlotCompiler, lower_ask, lower_choose, lower_repeat};
use crate::mod_compile_lowering::validate_choose_fanout;
use vb_core::workflow::SlotBranch;
use vb_core::{CompiledNode, CompiledNodeKind, SlotIdx, StepIdx};

const MAX_NON_OVERFLOWING_STEP_RAW: u16 = 65_534;

fn symbolic_non_max_step_raw() -> u16 {
    let raw: u16 = kani::any();
    kani::assume(raw < u16::MAX);
    kani::cover!(raw == 0, "non-max step domain includes zero");
    kani::cover!(
        raw == MAX_NON_OVERFLOWING_STEP_RAW,
        "non-max step domain includes max-minus-one"
    );
    raw
}

fn expected_successor(raw: u16) -> u16 {
    match raw.checked_add(1) {
        Some(value) => value,
        None => {
            kani::assert(false, "non-max step id must have an id + 1 successor");
            0
        }
    }
}

fn max_step_plus_one() -> usize {
    match usize::from(u16::MAX).checked_add(1) {
        Some(value) => value,
        None => {
            kani::assert(false, "usize must represent u16::MAX + 1 on this target");
            0
        }
    }
}

fn symbolic_step() -> StepIdx {
    StepIdx::new(kani::any::<u16>())
}

fn symbolic_slot() -> SlotIdx {
    SlotIdx::new(kani::any::<u16>())
}

fn symbolic_timeout() -> Option<SlotIdx> {
    if kani::any::<bool>() {
        Some(symbolic_slot())
    } else {
        None
    }
}

fn assert_repeat_nodes(
    nodes: &[CompiledNode],
    id: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    expected_slot: SlotIdx,
) {
    match nodes {
        [start, attempt, finish] => {
            assert_repeat_start(start, id, max_attempts, body, done);
            assert_repeat_attempt(attempt, body, done, expected_slot);
            assert_repeat_finish(finish, done, expected_slot);
        }
        _ => kani::assert(false, "lower_repeat must emit exactly three nodes"),
    }
}

fn assert_repeat_start(
    node: &CompiledNode,
    id: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
) {
    kani::assert(node.id == id, "RepeatStart id must equal input id");
    match node.kind {
        CompiledNodeKind::RepeatStart {
            max_attempts: actual,
            body: actual_body,
            done: actual_done,
        } => {
            kani::assert(actual == max_attempts, "RepeatStart max_attempts preserved");
            kani::assert(actual_body == body, "RepeatStart body preserved");
            kani::assert(actual_done == done, "RepeatStart done preserved");
        }
        _ => kani::assert(false, "first repeat node must be RepeatStart"),
    }
}

fn assert_repeat_attempt(
    node: &CompiledNode,
    body: StepIdx,
    done: StepIdx,
    expected_slot: SlotIdx,
) {
    kani::assert(node.id == body, "RepeatAttempt id must equal body step");
    kani::assert(
        node.output == Some(expected_slot),
        "RepeatAttempt output id + 1",
    );
    match node.kind {
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body: actual_body,
            done: actual_done,
        } => {
            kani::assert(attempt_slot == expected_slot, "RepeatAttempt slot id + 1");
            kani::assert(actual_body == body, "RepeatAttempt body preserved");
            kani::assert(actual_done == done, "RepeatAttempt done preserved");
        }
        _ => kani::assert(false, "second repeat node must be RepeatAttempt"),
    }
}

fn assert_repeat_finish(node: &CompiledNode, done: StepIdx, expected_slot: SlotIdx) {
    kani::assert(node.id == done, "RepeatFinish id must equal done step");
    match node.kind {
        CompiledNodeKind::RepeatFinish { result } => {
            kani::assert(result == expected_slot, "RepeatFinish result id + 1");
        }
        _ => kani::assert(false, "third repeat node must be RepeatFinish"),
    }
}

fn assert_ask_nodes(
    nodes: &[CompiledNode],
    id: StepIdx,
    expected_resume: StepIdx,
    prompt: SlotIdx,
    answer: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) {
    match nodes {
        [ask, resume] => {
            assert_ask_start(ask, id, prompt, timeout_slot);
            assert_ask_resume(resume, expected_resume, answer);
        }
        _ => kani::assert(false, "lower_ask must emit exactly two nodes"),
    }
}

fn assert_ask_start(
    node: &CompiledNode,
    id: StepIdx,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) {
    kani::assert(node.id == id, "Ask id must equal input id");
    match node.kind {
        CompiledNodeKind::Ask {
            prompt: actual_prompt,
            timeout_slot: actual_timeout,
        } => {
            kani::assert(actual_prompt == prompt, "Ask prompt preserved");
            kani::assert(actual_timeout == timeout_slot, "Ask timeout preserved");
        }
        _ => kani::assert(false, "first ask node must be Ask"),
    }
}

fn assert_ask_resume(node: &CompiledNode, expected_resume: StepIdx, answer: SlotIdx) {
    kani::assert(node.id == expected_resume, "AskResume id must be id + 1");
    kani::assert(node.output == Some(answer), "AskResume output answer slot");
    match node.kind {
        CompiledNodeKind::AskResume {
            answer: actual_answer,
        } => kani::assert(actual_answer == answer, "AskResume answer preserved"),
        _ => kani::assert(false, "second ask node must be AskResume"),
    }
}

#[kani::proof]
#[kani::unwind(12)]
fn lower_repeat_accepts_non_max_id_and_uses_id_plus_one_slot() {
    let id_raw = symbolic_non_max_step_raw();
    let successor_raw = expected_successor(id_raw);
    let id = StepIdx::new(id_raw);
    let expected_slot = SlotIdx::new(successor_raw);
    let max_attempts: u16 = kani::any();
    let body = symbolic_step();
    let done = symbolic_step();

    kani::cover!(max_attempts == 0, "repeat max_attempts includes zero");
    kani::cover!(max_attempts == u16::MAX, "repeat max_attempts includes max");
    kani::cover!(body == id, "repeat body may alias start id");
    kani::cover!(done == body, "repeat done may alias body");

    let mut builder = SlotCompiler::new();
    match lower_repeat(id, max_attempts, body, done, &mut builder) {
        Ok(nodes) => {
            assert_repeat_nodes(&nodes, id, max_attempts, body, done, expected_slot);
            std::mem::forget(nodes);
        }
        Err(_) => kani::assert(false, "lower_repeat must accept non-max step ids"),
    }
    std::mem::forget(builder);
}

#[kani::proof]
#[kani::unwind(12)]
fn lower_repeat_rejects_max_id_without_overflow() {
    let max_attempts: u16 = kani::any();
    let body = symbolic_step();
    let done = symbolic_step();
    let mut builder = SlotCompiler::new();

    match lower_repeat(StepIdx::MAX, max_attempts, body, done, &mut builder) {
        Ok(nodes) => {
            std::mem::forget(nodes);
            kani::assert(false, "lower_repeat must reject max step id");
        }
        Err(CompileError::StepIndexOutOfRange { value }) => {
            kani::assert(value == max_step_plus_one(), "repeat reports id + 1");
        }
        Err(_) => kani::assert(false, "lower_repeat must reject with limit error"),
    }
    std::mem::forget(builder);
}

#[kani::proof]
#[kani::unwind(12)]
fn lower_ask_accepts_non_max_id_and_uses_id_plus_one_resume() {
    let id_raw = symbolic_non_max_step_raw();
    let successor_raw = expected_successor(id_raw);
    let id = StepIdx::new(id_raw);
    let expected_resume = StepIdx::new(successor_raw);
    let prompt = symbolic_slot();
    let answer = symbolic_slot();
    let timeout_slot = symbolic_timeout();

    kani::cover!(prompt == answer, "ask prompt and answer may alias");
    kani::cover!(timeout_slot.is_some(), "ask timeout includes Some");
    kani::cover!(timeout_slot.is_none(), "ask timeout includes None");

    let mut builder = SlotCompiler::new();
    match lower_ask(id, prompt, answer, timeout_slot, &mut builder) {
        Ok(nodes) => {
            assert_ask_nodes(&nodes, id, expected_resume, prompt, answer, timeout_slot);
            std::mem::forget(nodes);
        }
        Err(_) => kani::assert(false, "lower_ask must accept non-max step ids"),
    }
    std::mem::forget(builder);
}

#[kani::proof]
#[kani::unwind(12)]
fn lower_ask_rejects_max_id_without_overflow() {
    let prompt = symbolic_slot();
    let answer = symbolic_slot();
    let timeout_slot = symbolic_timeout();
    let mut builder = SlotCompiler::new();

    match lower_ask(StepIdx::MAX, prompt, answer, timeout_slot, &mut builder) {
        Ok(nodes) => {
            std::mem::forget(nodes);
            kani::assert(false, "lower_ask must reject max step id");
        }
        Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: _,
            field: _,
            value,
            limit,
        }) => {
            kani::assert(value == usize::from(u16::MAX), "ask reports max id");
            kani::assert(limit == usize::from(u16::MAX), "ask reports step limit");
        }
        Err(_) => kani::assert(false, "lower_ask must reject with limit error"),
    }
    std::mem::forget(builder);
}

/// PO-001 H1: lower_choose correctly enforces the 64-branch fanout limit.
#[kani::proof]
#[kani::unwind(128)]
fn lower_choose_fanout_bound() {
    let test_rejection: bool = kani::any();
    let branch_count = if test_rejection { 65 } else { 64 };
    let result = validate_choose_fanout(branch_count);

    if test_rejection {
        match result {
            Err(CompileError::PrimitiveLoweringLimitExceeded {
                primitive,
                field,
                value,
                limit,
            }) => {
                kani::assert(primitive == "choose", "error primitive is choose");
                kani::assert(field == "branches", "error field is branches");
                kani::assert(value == 65, "error value matches branch count");
                kani::assert(limit == 64, "error limit is 64");
            }
            _ => {
                kani::assert(
                    false,
                    ">64 branches must reject with PrimitiveLoweringLimitExceeded",
                );
            }
        }
    } else {
        kani::assert(result.is_ok(), "64 branches must pass fanout check");
    }
}

/// PO-001 H2: Public lower_choose API enforces the fanout limit.
#[kani::proof]
#[kani::unwind(128)]
fn lower_choose_live_api_has_fanout_check() {
    let branch = SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    };
    let branches: Vec<SlotBranch> = vec![branch; 65];
    let mut builder = SlotCompiler::new();
    let result = lower_choose(
        StepIdx::new(0),
        branches,
        Some(StepIdx::new(1)),
        &mut builder,
    );

    match result {
        Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => {}
        Ok(_) => kani::assert(false, "public lower_choose must reject 65 branches"),
        Err(_) => kani::assert(
            false,
            "public lower_choose must reject with PrimitiveLoweringLimitExceeded",
        ),
    }
    std::mem::forget(builder);
}
