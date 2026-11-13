//! Cell A: ForEach > Repeat > Ask on owned strict Fjall admission.
//!
//! Bead: vb-eirmk.4.7.3. Two outer items x two bounded Repeat attempts give
//! four real Ask boundaries. Two boundaries complete live, the runtime and
//! journal handles drop, the same database reopens, and the remaining two
//! answers complete through freshly issued authorities without rescheduling a
//! resolved boundary or reissuing a resolved effect.

use vb_core::{CompiledNodeKind as K, RunId, SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_runtime::shard::AskAnswer;
use vb_storage::JournalEvent;

use super::source::{compound, indent};
use super::support::{Case, TestResult, assert_completed, count, journal_len, run_finished};

const RUN: RunId = RunId::new(4_703);
const FOREIGN_RUN: RunId = RunId::new(4_704);

fn source_text() -> TestResult<String> {
    let ask = "- id: question\n  ask: {prompt: '$vars.prompt'}\n";
    let repeat = compound("repeat", "middle", ask)?;
    let outer = compound("for_each", "outer", &repeat)?;
    Ok(format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_effects_foreach_repeat_ask\nwhen: {{manual: {{}}}}\nvars: {{prompt: 'row'}}\ninputs: {{items: {{is: list, of: number, max: 2}}}}\nsteps:\n{}  - id: check\n    set: {{output: check, value: '$step.outer.0 + $step.outer.1'}}\n  - id: done\n    finish: {{result: '$check'}}\n",
        indent(&outer, 2)
    ))
}

fn answer_sequence() -> [(SlotValue, Taint); 4] {
    [
        (SlotValue::I64(101), Taint::DerivedFromSecret),
        (SlotValue::I64(102), Taint::Clean),
        (SlotValue::I64(103), Taint::DerivedFromSecret),
        (SlotValue::I64(104), Taint::Clean),
    ]
}

#[test]
fn owned_foreach_repeat_ask_reopens_between_answers_without_effect_reissue() -> TestResult {
    let text = source_text()?;
    let case = Case::new(
        &text,
        RUN,
        &[SlotValue::I64(10), SlotValue::I64(20)],
        &[Taint::Secret, Taint::Clean],
        &[],
    )?;
    let ask = case.step(|kind| matches!(kind, K::Ask { .. }))?;
    let bound = case.workflow.node_count().checked_mul(64).ok_or("bound")?;
    let answers = answer_sequence();

    let (journal, mut runtime) = case.activate_live()?;
    let mut tickets = Vec::new();
    let mut answered = 0_usize;
    for _ in 0..bound {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if answered == 2 {
            break;
        }
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            let (value, taint) = answers[answered];
            assert!(!tickets.contains(&ticket), "duplicate live Ask authority");
            runtime.answer_ask(AskAnswer::new(ticket, ticket.answer_slot(), value, taint))?;
            tickets.push(ticket);
            answered = answered.checked_add(1).ok_or("answer count overflow")?;
        }
    }
    assert_eq!(answered, 2, "two live Ask activations expected");
    for _ in 0..bound {
        runtime.tick_all()?;
        if runtime.lookup_pending_ask_ticket(RUN, ask).is_some() {
            break;
        }
    }
    assert!(
        runtime.lookup_pending_ask_ticket(RUN, ask).is_some(),
        "third boundary must be suspended before the cold cut"
    );
    assert_eq!(
        count(&case.events(&journal)?, |event| matches!(
            event,
            JournalEvent::AskScheduledEvent { .. }
        )),
        3
    );
    drop(runtime);
    drop(journal);

    let (journal, mut runtime) = case.recover()?;
    for _ in 0..bound {
        if runtime.lookup_pending_ask_ticket(RUN, ask).is_some() {
            break;
        }
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    }
    let reopened = runtime
        .lookup_pending_ask_ticket(RUN, ask)
        .ok_or("recovered third Ask boundary")?;
    assert!(
        !tickets.contains(&reopened),
        "recovered authority must be fresh"
    );
    assert_eq!(
        count(&case.events(&journal)?, |event| matches!(
            event,
            JournalEvent::AskScheduledEvent { .. }
        )),
        3,
        "recovery must not reschedule resolved boundaries"
    );

    let mut remaining = answers.iter().skip(2);
    for _ in 0..bound {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            let (value, taint) = remaining.next().ok_or("unexpected extra Ask boundary")?;
            assert!(
                !tickets.contains(&ticket),
                "duplicate recovered Ask authority"
            );
            runtime.answer_ask(AskAnswer::new(ticket, ticket.answer_slot(), *value, *taint))?;
            tickets.push(ticket);
        }
    }
    assert!(
        remaining.next().is_none(),
        "recovered answers not all consumed"
    );

    let events = case.events(&journal)?;
    assert_completed(&runtime, &events);
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::AskScheduledEvent { .. }
        )),
        4
    );
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::AskAnsweredEvent { .. }
        )),
        4
    );
    assert_eq!(tickets.len(), 4);

    // The two attempts for item one are 101/102; item two's are 103/104.
    // Repeat retains each item's final answer, and ForEach joins the source
    // list's Secret taint into its result.
    assert_eq!(
        run_finished(&events)?,
        (SlotValue::I64(206), Taint::Secret),
        "final journal {events:?}"
    );

    // Stale authority after terminal completion: typed refusal, no journal append.
    let before = journal_len(&case, &journal)?;
    let stale = *tickets.first().ok_or("captured ticket")?;
    let refused = match runtime.answer_ask(AskAnswer::new(
        stale,
        stale.answer_slot(),
        SlotValue::I64(9),
        Taint::Clean,
    )) {
        Ok(()) => return Err("stale Ask answer was accepted after completion".into()),
        Err(error) => error,
    };
    assert_eq!(refused, RuntimeError::InvalidActionCompletion);
    assert_eq!(
        journal_len(&case, &journal)?,
        before,
        "refusal appended evidence"
    );

    // Foreign authority: the same ticket against a runtime that never owned RUN.
    let foreign = Case::new(
        &text,
        FOREIGN_RUN,
        &[SlotValue::I64(10)],
        &[Taint::Clean],
        &[],
    )?;
    let (foreign_journal, foreign_runtime) = foreign.activate_live()?;
    let foreign_before = journal_len(&foreign, &foreign_journal)?;
    let refused = match foreign_runtime.answer_ask(AskAnswer::new(
        stale,
        stale.answer_slot(),
        SlotValue::I64(9),
        Taint::Clean,
    )) {
        Ok(()) => return Err("foreign Ask answer was accepted".into()),
        Err(error) => error,
    };
    assert_eq!(refused, RuntimeError::RunNotFound);
    assert_eq!(
        journal_len(&foreign, &foreign_journal)?,
        foreign_before,
        "foreign refusal appended evidence"
    );
    Ok(())
}
