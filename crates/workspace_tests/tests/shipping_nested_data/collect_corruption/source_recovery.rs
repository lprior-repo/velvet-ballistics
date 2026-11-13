use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::{ActionId, CompiledNodeKind, RuntimePolicy, SlotValue, Taint};
use vb_storage::{FjallJournal, JournalEvent};

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
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
    let mut runtime = runtime(journal.clone());
    runtime.activate_owned_submission(RUN, prepared.source_digest)?;
    let mut events = None;
    for _ in 0..128 {
        runtime.tick_all()?;
        let current = journal.events_for_run_full(RUN)?;
        if current.iter().any(|event| {
            matches!(event, JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.step == step)
        }) {
            events = Some(current);
            break;
        }
    }
    let mut events = events.ok_or_else(|| {
        format!(
            "public loop runtime did not schedule the Do: {:?}",
            journal.events_for_run_full(RUN)
        )
    })?;
    let mut replaced = false;
    for event in &mut events {
        let legacy = match event {
            JournalEvent::ActionScheduledTicket {
                run, seq, ticket, ..
            } if ticket.step == step => Some(JournalEvent::ActionScheduled {
                run: *run,
                seq: *seq,
                step: ticket.step,
                action: ticket.action,
                attempt: ticket.attempt,
            }),
            _ => None,
        };
        if let Some(legacy) = legacy {
            *event = legacy;
            replaced = true;
        }
    }
    if !replaced {
        return Err("real Do schedule was not converted to legacy form".into());
    }
    if vb_storage::recovery::replay_owned_for_each(
        &events,
        &prepared.workflow,
        &prepared.roots,
        Some(&prepared.input_store),
    )?
    .is_some()
    {
        return Err(format!("legacy schedule selected owned replay for action {action:?}").into());
    }
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
