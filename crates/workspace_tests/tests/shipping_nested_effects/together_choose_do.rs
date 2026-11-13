//! Cell C: Together > Choose > Do on owned strict Fjall admission.
//!
//! Bead: vb-eirmk.4.7.3. The chosen branch schedules two real actions. The
//! first completion is live, the handles drop, and the second completion is
//! driven from the recovered pending ticket without reissuing the first.

#[path = "together_choose_do/evidence.rs"]
mod evidence;

use self::evidence::{RUN, assert_action_completion, completion, do_steps, source_text};
use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::{SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_storage::JournalEvent;

use super::support::{Case, TestResult, assert_completed, contract, count, run_finished};

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
    let (first_step, first_action, first_output) = *steps.first().ok_or("first Do")?;
    let (second_step, second_action, second_output) = *steps.get(1).ok_or("second Do")?;
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
    let before_duplicate = case.events(&journal)?;
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
        case.events(&journal)?,
        before_duplicate,
        "refusal changed the durable journal"
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
    let scheduled_sequences: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionScheduledTicket { run, seq, .. } if *run == RUN => Some(*seq),
            _ => None,
        })
        .collect();
    assert_eq!(scheduled_sequences.len(), 2);
    let (first_schedule_seq, second_schedule_seq) = match scheduled_sequences.as_slice() {
        [first, second] => (*first, *second),
        _ => return Err("expected two scheduled Do events".into()),
    };
    assert!(first_schedule_seq < second_schedule_seq);
    let scheduled_tickets: Vec<ActionTicket> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionScheduledTicket { run, ticket, .. } if *run == RUN => Some(*ticket),
            _ => None,
        })
        .collect();
    assert_eq!(
        scheduled_tickets,
        [first_ticket, second_ticket],
        "Do tickets must remain ordered across cold recovery"
    );
    assert_ticket_identity(&first_ticket, first_step, first_action)?;
    assert_ticket_identity(&second_ticket, second_step, second_action)?;
    let completions: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionCompletedEnvelope {
                run, seq, ticket, ..
            } if *run == RUN => Some((*seq, *ticket)),
            _ => None,
        })
        .collect();
    let completed_tickets: Vec<ActionTicket> =
        completions.iter().map(|(_, ticket)| *ticket).collect();
    assert_eq!(
        completed_tickets,
        [first_ticket, second_ticket],
        "completion order and multiplicity must match the two compiled Do nodes"
    );
    let completion_sequences: Vec<_> = completions.iter().map(|(seq, _)| *seq).collect();
    let (first_completion_seq, second_completion_seq) = match completion_sequences.as_slice() {
        [first, second] => (*first, *second),
        _ => return Err("expected two completion events".into()),
    };
    assert!(first_completion_seq < second_completion_seq);
    assert_action_completion(
        &events,
        &first_ticket,
        first_step,
        first_action,
        first_output,
        SlotValue::I64(values[0]),
        Taint::DerivedFromSecret,
    )?;
    assert_action_completion(
        &events,
        &second_ticket,
        second_step,
        second_action,
        second_output,
        SlotValue::I64(values[1]),
        Taint::Clean,
    )?;
    // together.[0] = explicit 88 outer tail after outputless Choose;
    // together.[1] = 17. Do completions are asserted separately above.
    assert_eq!(run_finished(&events)?, (SlotValue::I64(105), Taint::Clean));

    Ok(())
}
pub(super) fn assert_ticket_identity(
    ticket: &ActionTicket,
    expected_step: vb_core::StepIdx,
    expected_action: vb_core::ActionId,
) -> TestResult {
    assert_eq!(ticket.run, RUN);
    assert_eq!(ticket.step, expected_step);
    assert_eq!(ticket.action, expected_action);
    assert_eq!(ticket.run_attempt, 1);
    assert_eq!(ticket.attempt, 1);
    assert_eq!(
        ticket.idempotency_key,
        compute_action_idempotency_key(RUN, ticket.run_attempt, ticket.seq, expected_action)
    );
    assert_eq!(ticket.capacity, 1);
    assert!(ticket.fencing_token > 0);
    Ok(())
}
