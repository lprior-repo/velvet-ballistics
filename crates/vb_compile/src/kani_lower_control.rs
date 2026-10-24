#![forbid(unsafe_code)]
//! Kani harnesses for `lower_ask` resume IDs and bounded Choose lowering.
//! Bead: vb-onsk; scope: current public `vb_compile` APIs.

use crate::{CompileError, SlotCompiler, lower_ask, lower_choose};
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

// ───────────────────────────────────────────────────────────────────────────
// vb-awhr harnesses: fanout limit and otherwise label resolution
// ───────────────────────────────────────────────────────────────────────────

use vb_core::workflow::SlotBranch;

/// PO-001 H1: lower_choose correctly enforces the 64-branch fanout limit.
#[kani::proof]
#[kani::unwind(128)]
fn lower_choose_fanout_bound() {
    let test_rejection: bool = kani::any();

    let branches: Vec<SlotBranch> = if test_rejection {
        (0..65)
            .map(|_| SlotBranch {
                condition: SlotIdx::new(kani::any()),
                target: StepIdx::new(kani::any()),
            })
            .collect()
    } else {
        (0..64)
            .map(|_| SlotBranch {
                condition: SlotIdx::new(kani::any()),
                target: StepIdx::new(kani::any()),
            })
            .collect()
    };

    let mut builder = SlotCompiler::new();
    let result = lower_choose(
        StepIdx::new(0),
        branches,
        Some(StepIdx::new(1)),
        &mut builder,
    );

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
        kani::assert(result.is_ok(), "≤64 branches must be accepted");
    }
    std::mem::forget(builder);
}

/// PO-001 H2: Public lower_choose API enforces the fanout limit.
#[kani::proof]
#[kani::unwind(128)]
fn lower_choose_live_api_has_fanout_check() {
    let branches: Vec<SlotBranch> = (0..65)
        .map(|_| SlotBranch {
            condition: SlotIdx::new(kani::any()),
            target: StepIdx::new(kani::any()),
        })
        .collect();

    let mut builder = SlotCompiler::new();
    let result = lower_choose(
        StepIdx::new(0),
        branches,
        Some(StepIdx::new(1)),
        &mut builder,
    );

    match result {
        Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => {}
        Ok(_) => {
            kani::assert(false, "public lower_choose must reject 65 branches");
        }
        Err(_) => {
            kani::assert(
                false,
                "public lower_choose must reject with PrimitiveLoweringLimitExceeded",
            );
        }
    }
    std::mem::forget(builder);
}

// otherwise label resolution harness is in mod_compile_lowering/kani_vb_awhr.rs
