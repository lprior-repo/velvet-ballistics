use vb_core::action::{ActionOutputReady, ActionTicket, compute_action_idempotency_key};
use vb_core::{CompiledNodeKind as K, RunId, SlotIdx, SlotValue, Taint};
use vb_storage::JournalEvent;

use super::super::source::{compound, indent};
use super::super::support::{Case, TestResult};
use crate::source;

pub(super) const RUN: RunId = RunId::new(4_723);

pub(super) fn source_text() -> TestResult<String> {
    if !source::FAMILIES.contains(&"choose") || !source::FAMILIES.contains(&"together") {
        return Err("nested primitive source fixture is incomplete".into());
    }
    let leaf = "- id: first\n  do: {action: '0', input: '$seed'}\n- id: second\n  do: {action: '0', input: '$seed'}\n";
    let choose = compound("choose", "middle", leaf)?;
    // Choose does not expose its branch-local Do value to the outer Together
    // branch, so the outer branch supplies the explicit 88 result tail.
    let choose_and_result =
        format!("{choose}- id: selected\n  set: {{output: selected, value: '88'}}\n");
    let outer = compound("together", "outer", &choose_and_result)?;
    Ok(format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_effects_together_choose_do\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: number, max: 1}}}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '7'}}\n{}  - id: check\n    set: {{output: check, value: '$step.outer.0 + $step.outer.1'}}\n  - id: done\n    finish: {{result: '$check'}}\n",
        indent(&outer, 2)
    ))
}

pub(super) fn do_steps(
    case: &Case,
) -> TestResult<Vec<(vb_core::StepIdx, vb_core::ActionId, SlotIdx)>> {
    Ok(case
        .workflow
        .to_parts()
        .nodes
        .iter()
        .filter_map(|node| match node.kind {
            K::Do { action, .. } => node.output.slot().map(|output| (node.id, action, output)),
            _ => None,
        })
        .collect())
}

pub(super) fn completion(
    output: SlotIdx,
    value: SlotValue,
    taint: Taint,
) -> TestResult<ActionOutputReady> {
    let encoded = postcard::to_allocvec(&value)?;
    Ok(ActionOutputReady {
        output_slot: output,
        value,
        taint,
        encoded_len: u32::try_from(encoded.len())?,
    })
}

pub(super) fn assert_action_completion(
    events: &[JournalEvent],
    expected_ticket: &ActionTicket,
    expected_step: vb_core::StepIdx,
    expected_action: vb_core::ActionId,
    output: SlotIdx,
    value: SlotValue,
    taint: Taint,
) -> TestResult {
    let expected_value = postcard::to_allocvec(&value)?;
    let mut matching = events.iter().filter_map(|event| match event {
        JournalEvent::ActionCompletedEnvelope { ticket, .. } if ticket.step == expected_step => {
            Some(event)
        }
        _ => None,
    });
    let event = matching.next().ok_or("missing completion for Do step")?;
    if matching.next().is_some() {
        return Err("multiple completions for Do step".into());
    }
    match event {
        JournalEvent::ActionCompletedEnvelope {
            run,
            ticket,
            output: actual_output,
            outcome: vb_storage::DurableActionOutcome::Ready,
            value: actual_value,
            encoded_len,
            taint: actual_taint,
            ..
        } if *run == RUN
            && ticket == expected_ticket
            && ticket.run == RUN
            && ticket.step == expected_step
            && ticket.action == expected_action
            && ticket.run_attempt == 1
            && ticket.attempt == 1
            && ticket.idempotency_key
                == compute_action_idempotency_key(
                    RUN,
                    ticket.run_attempt,
                    ticket.seq,
                    expected_action,
                )
            && ticket.capacity == 1
            && ticket.fencing_token > 0
            && *actual_output == output
            && *actual_value == expected_value
            && *encoded_len == u32::try_from(expected_value.len())?
            && *actual_taint == taint =>
        {
            Ok(())
        }
        _ => Err("Do completion ticket, value, or taint differed".into()),
    }
}
