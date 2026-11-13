use vb_core::{SlotValue, Taint};
use vb_storage::JournalEvent;

use crate::flow::run_cell_source;
use crate::support::{RUN, TestResult, nested_source, prepare};

#[test]
fn legacy_action_scheduled_selects_the_generic_replay_fallback() -> TestResult {
    let source = nested_source("for_each", "choose")?;
    let prepared = prepare(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(5)],
        &[Taint::Secret, Taint::DerivedFromSecret],
    )?;
    let legacy = JournalEvent::ActionScheduled {
        run: RUN,
        seq: vb_storage::EventSeq::new(0),
        step: prepared.workflow.entry(),
        action: vb_core::ActionId::new(1),
        attempt: 1,
    };
    let replay = vb_storage::recovery::replay_owned_for_each(
        &[legacy],
        &prepared.workflow,
        &prepared.roots,
        Some(&prepared.input_store),
    )?;
    if replay.is_some() {
        return Err("legacy action event did not select generic recovery".into());
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
