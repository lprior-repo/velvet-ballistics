use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::{ActionId, CompiledNodeKind, RuntimePolicy, SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

use crate::flow::run_cell_source;
use crate::support::{RUN, TestResult, nested_source, prepare_with_contracts};

use super::support::runtime;

fn action_contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("legacy-replay-fallback")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

fn next_sequence(events: &[JournalEvent]) -> TestResult<EventSeq> {
    let last = events
        .last()
        .ok_or("accepted artifact has no durable admission events")?;
    Ok(EventSeq::new(
        last.seq()
            .get()
            .checked_add(1)
            .ok_or("legacy event sequence overflow")?,
    ))
}

#[test]
fn legacy_action_scheduled_selects_the_generic_replay_fallback() -> TestResult {
    let source = "version: velvet-ballistics/v1\nname: legacy_loop_action\nwhen: {manual: {}}\ninputs: {items: {is: list, of: number, max: 2}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '$input.items.0'}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: action\n          do: {action: '0', input: '$step.seed'}\n        - id: tail\n          set: {output: tail, value: '$step.seed'}\n  - id: done\n    finish: {result: '$step.seed'}\n";
    let prepared = prepare_with_contracts(
        source,
        &[SlotValue::I64(3)],
        &[Taint::Clean],
        &[action_contract()?],
    )?;
    let (step, action) = prepared
        .workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::Do { action, .. } => Some((node.id, action)),
            _ => None,
        })
        .ok_or("compiled workflow has no Do")?;
    let finish_result = prepared
        .workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
        .ok_or("compiled workflow has no terminal result")?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
    let live_runtime = runtime(journal.clone());
    live_runtime.activate_owned_submission(RUN, prepared.source_digest)?;

    let admitted = journal.events_for_run_full(RUN)?;
    assert!(
        admitted
            .iter()
            .any(|event| matches!(event, JournalEvent::RunAccepted { .. })),
        "legacy events must follow persisted accepted source/artifact admission"
    );
    let schedule_seq = next_sequence(&admitted)?;
    journal.append_strict(&JournalEvent::ActionScheduled {
        run: RUN,
        seq: schedule_seq,
        step,
        action,
        attempt: 1,
    })?;

    let pending_events = journal.events_for_run_full(RUN)?;
    assert!(matches!(
        pending_events.last(),
        Some(JournalEvent::ActionScheduled { run, seq, step: event_step, action: event_action, attempt: 1 })
            if *run == RUN && *seq == schedule_seq && *event_step == step && *event_action == action
    ));
    drop(live_runtime);
    let pending_runtime = runtime(journal.clone());
    assert_eq!(
        pending_runtime.recover_product(RUN),
        Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("pending_actions")
        })
    );
    assert_eq!(
        pending_runtime.recover_and_resume(RUN),
        Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("pending_actions")
        })
    );
    assert_eq!(
        journal.events_for_run_full(RUN)?,
        pending_events,
        "typed legacy refusal must not mutate the full durable journal"
    );
    assert!(
        pending_runtime
            .lookup_pending_action_ticket(RUN, u64::from(step.get()))
            .is_none(),
        "legacy schedule must not fabricate a live ticket"
    );
    assert!(!pending_events.iter().any(|event| matches!(
        event,
        JournalEvent::ActionScheduledTicket { .. } | JournalEvent::ActionCompletedEnvelope { .. }
    )));
    let completion_seq = next_sequence(&pending_events)?;
    let terminal_seq = EventSeq::new(
        completion_seq
            .get()
            .checked_add(1)
            .ok_or("legacy terminal sequence overflow")?,
    );
    journal.append_strict_batch(&[
        JournalEvent::ActionCompletedEvent {
            run: RUN,
            seq: completion_seq,
            step,
            action,
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run: RUN,
            seq: terminal_seq,
            result: finish_result,
            attempt: 1,
        },
    ])?;
    let terminal_events = journal.events_for_run_full(RUN)?;
    assert!(terminal_events.iter().any(|event| matches!(
        event,
        JournalEvent::ActionCompletedEvent { run, seq, step: event_step, action: event_action, attempt: 1 }
            if *run == RUN && *seq == completion_seq && *event_step == step && *event_action == action
    )));
    assert!(terminal_events.iter().any(|event| matches!(
        event,
        JournalEvent::RunFinished { run, seq, result, attempt: 1 }
            if *run == RUN && *seq == terminal_seq && *result == finish_result
    )));
    drop(pending_runtime);
    let terminal_runtime = runtime(journal.clone());
    let terminal = terminal_runtime.recover_product(RUN)?;
    assert!(matches!(
        &terminal,
        vb_runtime::recovery::RuntimeRecoveryProduct::CannotResume(_)
    ));
    assert_eq!(terminal.cannot_resume_reason(), Some("terminal_state"));
    assert_eq!(
        terminal_runtime.recover_and_resume(RUN),
        Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("terminal_state")
        })
    );
    assert_eq!(
        journal.events_for_run_full(RUN)?,
        terminal_events,
        "terminal legacy handling must preserve the full durable journal"
    );
    Ok(())
}

#[test]
fn retained_owned_source_choose_only_workflow_recovers_and_replays() -> TestResult {
    let source = nested_source("choose", "choose")?;
    let source = source.replace(
        "finish: {result: '$steps.outer'}",
        "finish: {result: '$vars.sentinel'}",
    );
    let observed = run_cell_source("choose", "choose", &source)?;
    if observed.terminal_value != SlotValue::I64(42)
        || observed.terminal_taint != Taint::Clean
        || !observed.cut_replay_selected
        || !observed.full_replay_selected
        || observed.cut_completed != 0
        || observed.leaf_successes != 1
    {
        return Err("retained-source choose-only cold replay evidence diverged".into());
    }
    Ok(())
}

#[test]
fn collect_outer_tail_preserves_distinct_input_order_after_cold_replay() -> TestResult {
    let source = nested_source("for_each", "choose")?;
    let changed = source.replace("$input.items.0", "$item_outer");
    if changed == source {
        return Err("outer tail witness rewrite did not apply".into());
    }
    let observed = run_cell_source("for_each", "choose", &changed)?;
    if observed.terminal_items.as_deref() != Some(&[SlotValue::I64(3), SlotValue::I64(5)]) {
        return Err(format!("cold replay order was {:?}", observed.terminal_items).into());
    }
    if observed.cut_completed != 0
        || !observed.cut_replay_selected
        || !observed.full_replay_selected
    {
        return Err("ordered witness did not cross nonterminal cold replay".into());
    }
    Ok(())
}
