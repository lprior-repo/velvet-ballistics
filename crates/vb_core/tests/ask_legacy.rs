//! The host authorizes answers from the immediate AskResume, not Ask.output.
#[path = "slot_flow_support/mod.rs"]
mod support;
use support::*;
use vb_core::workflow::{CompiledNodeKind as K, CompiledWorkflow, SlotFlowFailure};

fn ask(output: Option<u16>) -> vb_core::CompiledNode {
    node(
        1,
        K::Ask {
            prompt: slot(0),
            timeout_slot: None.into(),
        },
        output,
        Some(2),
    )
}

fn legacy_parts() -> vb_core::WorkflowParts {
    parts(
        vec![
            set(0, 0, 0, 1),
            ask(None),
            node(2, K::AskResume { answer: slot(1) }, Some(2), Some(3)),
            finish(3, 2),
        ],
        3,
    )
}

fn rejected_at(p: vb_core::WorkflowParts, at: u16, read: u16) -> TestResult {
    let error = vb_core::workflow::validate_slot_flow(&p)
        .err()
        .ok_or("slot flow accepted absent answer")?;
    assert_eq!(error.step, step(at));
    assert_eq!(error.slot, Some(slot(read)));
    assert_eq!(error.reason, SlotFlowFailure::UninitializedRead);
    rejected(p, SlotFlowFailure::UninitializedRead)
}

#[test]
fn output_none_supplies_only_the_immediate_resume_answer() -> TestResult {
    CompiledWorkflow::try_from_parts(legacy_parts())?;
    Ok(())
}

#[test]
fn existing_unrelated_ask_output_does_not_replace_resume_authority() -> TestResult {
    let mut p = legacy_parts();
    p.nodes.get_mut(1).ok_or("missing Ask")?.output = Some(slot(0)).into();
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn branch_bypassing_output_none_ask_keeps_answer_absent() -> TestResult {
    let mut p = parts(
        vec![
            choose(0, 1, 2),
            ask(None),
            node(2, K::AskResume { answer: slot(1) }, Some(2), Some(3)),
            finish(3, 2),
        ],
        4,
    );
    p.nodes.get_mut(1).ok_or("missing Ask")?.kind = K::Ask {
        prompt: slot(3),
        timeout_slot: None.into(),
    };
    declare(&mut p, 0, vb_core::InputSlotKind::Bool);
    declare(&mut p, 3, vb_core::InputSlotKind::I64);
    rejected_at(p, 2, 1)
}

#[test]
fn error_edge_does_not_inherit_successful_host_answer() -> TestResult {
    let mut p = legacy_parts();
    p.nodes.get_mut(1).ok_or("missing Ask")?.routing.error =
        vb_core::workflow::ErrorRouting::Handler {
            handler: step(2),
            error_slot: None,
        };
    rejected_at(p, 2, 1)
}

#[test]
fn error_handler_can_read_the_pre_ask_prompt() -> TestResult {
    let mut p = legacy_parts();
    p.nodes.get_mut(1).ok_or("missing Ask")?.routing.error =
        vb_core::workflow::ErrorRouting::Handler {
            handler: step(4),
            error_slot: None,
        };
    let mut nodes = p.nodes.into_vec();
    nodes.push(finish(4, 0));
    p.nodes = nodes.into_boxed_slice();
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn ask_resume_without_ask_cannot_create_an_answer() -> TestResult {
    rejected_at(
        parts(
            vec![
                node(0, K::AskResume { answer: slot(0) }, Some(1), Some(1)),
                finish(1, 1),
            ],
            2,
        ),
        0,
        0,
    )
}

#[test]
fn non_resume_successor_does_not_authorize_ask_output() -> TestResult {
    rejected_at(
        parts(
            vec![
                set(0, 0, 0, 1),
                ask(Some(1)),
                node(2, K::Nop, None, Some(3)),
                finish(3, 1),
            ],
            2,
        ),
        1,
        1,
    )
}
