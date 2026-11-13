//! Cell C: Together > Choose > Do on owned strict Fjall admission.
//!
//! Bead: vb-eirmk.4.7.3. The chosen branch schedules two real actions. The
//! first completion is live, the handles drop, and the second completion is
//! driven from the recovered pending ticket without reissuing the first.

use vb_core::action::ActionOutputReady;
use vb_core::action::ActionTicket;
use vb_core::{CompiledNodeKind as K, RunId, SlotIdx, SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_storage::JournalEvent;

use super::source::{compound, indent};
use super::support::{Case, TestResult, assert_completed, contract, count, run_finished};
use crate::source;

const RUN: RunId = RunId::new(4_723);

fn source_text() -> TestResult<String> {
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

fn do_steps(case: &Case) -> TestResult<Vec<(vb_core::StepIdx, SlotIdx)>> {
    Ok(case
        .workflow
        .to_parts()
        .nodes
        .iter()
        .filter_map(|node| match node.kind {
            K::Do { .. } => node.output.slot().map(|output| (node.id, output)),
            _ => None,
        })
        .collect())
}

fn completion(output: SlotIdx, value: SlotValue, taint: Taint) -> TestResult<ActionOutputReady> {
    let encoded = postcard::to_allocvec(&value)?;
    Ok(ActionOutputReady {
        output_slot: output,
        value,
        taint,
        encoded_len: u32::try_from(encoded.len())?,
    })
}

fn assert_action_completion(
    events: &[JournalEvent],
    expected_ticket: &ActionTicket,
    output: SlotIdx,
    value: SlotValue,
    taint: Taint,
) -> TestResult {
    let expected_value = postcard::to_allocvec(&value)?;
    let mut matching = events.iter().filter_map(|event| match event {
        JournalEvent::ActionCompletedEnvelope { ticket, .. }
            if ticket.step == expected_ticket.step =>
        {
            Some(event)
        }
        _ => None,
    });
    let event = matching.next().ok_or("missing action completion")?;
    if matching.next().is_some() {
        return Err("multiple completions for Do step".into());
    }
    match event {
        JournalEvent::ActionCompletedEnvelope {
            ticket,
            output: actual_output,
            outcome: vb_storage::DurableActionOutcome::Ready,
            value: actual_value,
            taint: actual_taint,
            ..
        } if ticket == expected_ticket
            && *actual_output == output
            && *actual_value == expected_value
            && *actual_taint == taint =>
        {
            Ok(())
        }
        _ => Err("Do completion ticket, value, or taint differed".into()),
    }
}

#[test]
fn owned_together_choose_do_reopens_between_completions_without_reissue() -> TestResult {
    let text = source_text()?;
    let case = Case::new(
        &text,
        RUN,
        &[SlotValue::I64(1)],
        &[Taint::Clean],
        &[contract()?],
    )?;
    let steps = do_steps(&case)?;
    assert_eq!(steps.len(), 2, "two Do activations in the chosen branch");
    let (first_step, first_output) = *steps.first().ok_or("first Do")?;
    let (second_step, second_output) = *steps.get(1).ok_or("second Do")?;
    let bound = case.workflow.node_count().checked_mul(64).ok_or("bound")?;
    let values = [55_i64, 88_i64];

    let (journal, mut runtime) = case.activate_live()?;
    let mut completed = 0_usize;
    let mut first_ticket = None;
    for _ in 0..bound {
        runtime.tick_all()?;
        assert_eq!(
            runtime.counters_snapshot().runs_failed,
            0,
            "recovered execution failed: events={:?}, trace={:?}",
            case.events(&journal)?,
            runtime.list_events(RUN)?
        );
        if completed == 1 {
            break;
        }
        if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(first_step.get()))
        {
            first_ticket = Some(ticket.clone());
            runtime.complete_action_with_output(
                ticket,
                completion(
                    first_output,
                    SlotValue::I64(values[0]),
                    Taint::DerivedFromSecret,
                )?,
            )?;
            completed = completed.checked_add(1).ok_or("completion overflow")?;
        }
    }
    assert_eq!(completed, 1, "one live action activation expected");
    let before_duplicate = case.events(&journal)?.len();
    let stale_first = first_ticket
        .as_ref()
        .ok_or("completed first action ticket")?
        .clone();
    let refused = match runtime.complete_action_with_output(
        stale_first,
        completion(first_output, SlotValue::I64(99), Taint::Clean)?,
    ) {
        Ok(()) => return Err("duplicate action completion was accepted".into()),
        Err(error) => error,
    };
    assert!(
        matches!(refused, RuntimeError::InvalidActionCompletion),
        "unexpected duplicate refusal: {refused:?}"
    );
    assert_eq!(
        case.events(&journal)?.len(),
        before_duplicate,
        "refusal appended evidence"
    );
    // Applying the first completion also schedules the branch's second action,
    // so the live journal holds two scheduled tickets and one completion.
    assert_eq!(
        count(&case.events(&journal)?, |event| matches!(
            event,
            JournalEvent::ActionScheduledTicket { .. }
        )),
        2
    );
    assert_eq!(
        count(&case.events(&journal)?, |event| matches!(
            event,
            JournalEvent::ActionCompletedEnvelope { .. }
        )),
        1
    );
    drop(runtime);
    drop(journal);

    let (journal, mut runtime) = case.open_with_quantum(1)?;
    assert!(matches!(
        runtime.recover_product(RUN)?,
        vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    assert_eq!(
        count(&case.events(&journal)?, |event| matches!(
            event,
            JournalEvent::ActionScheduledTicket { .. }
        )),
        2,
        "recovery must not reschedule a scheduled action"
    );
    let mut second_ticket = None;
    for _ in 0..bound {
        runtime.tick_all()?;
        assert_eq!(
            runtime.counters_snapshot().runs_failed,
            0,
            "recovered execution failed: events={:?}, trace={:?}",
            case.events(&journal)?,
            runtime.list_events(RUN)?
        );
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        if let Some(ticket) =
            runtime.lookup_pending_action_ticket(RUN, u64::from(second_step.get()))
        {
            second_ticket = Some(ticket.clone());
            runtime.complete_action_with_output(
                ticket,
                completion(second_output, SlotValue::I64(values[1]), Taint::Clean)?,
            )?;
            completed = completed.checked_add(1).ok_or("completion overflow")?;
        }
    }

    let events = case.events(&journal)?;
    assert_completed(&runtime, &events);
    assert_eq!(completed, 2);
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::ActionScheduledTicket { .. }
        )),
        2
    );
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::ActionCompletedEnvelope { .. }
        )),
        2
    );
    let first_ticket = first_ticket.ok_or("first action ticket")?;
    let second_ticket = second_ticket.ok_or("second action ticket")?;
    let scheduled_tickets: Vec<ActionTicket> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionScheduledTicket { ticket, .. } => Some(ticket.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        scheduled_tickets,
        [first_ticket.clone(), second_ticket.clone()],
        "Do tickets must remain ordered across cold recovery"
    );
    assert_eq!(first_ticket.step, first_step);
    assert_eq!(second_ticket.step, second_step);
    assert_eq!((first_ticket.attempt, second_ticket.attempt), (1, 1));
    assert_action_completion(
        &events,
        &first_ticket,
        first_output,
        SlotValue::I64(values[0]),
        Taint::DerivedFromSecret,
    )?;
    assert_action_completion(
        &events,
        &second_ticket,
        second_output,
        SlotValue::I64(values[1]),
        Taint::Clean,
    )?;
    // together.[0] = explicit 88 outer tail after outputless Choose;
    // together.[1] = 17. Do completions are asserted separately above.
    assert_eq!(run_finished(&events)?, (SlotValue::I64(105), Taint::Clean));

    Ok(())
}
