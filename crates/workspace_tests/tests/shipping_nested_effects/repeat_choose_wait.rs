//! Cell B: Repeat > Choose > Wait on owned strict Fjall admission.
//!
//! Bead: vb-eirmk.4.7.3. Two bounded Repeat attempts each suspend on a real
//! Wait timer. The first timer fires live, the handles drop, the same database
//! reopens, and the second timer fires from a freshly captured authority.

use std::time::{Duration, Instant};

use vb_core::{RunId, SlotValue, Taint};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::PendingTimerKind;
use vb_runtime::shard::timer_wheel::TimerEntry;
use vb_storage::JournalEvent;

use super::source::{compound, indent};
use super::support::{Case, TestResult, assert_completed, count, run_finished};

const RUN: RunId = RunId::new(4_713);

fn source_text() -> TestResult<String> {
    let leaf = "- id: paused\n  wait: {timeout: '$delay'}\n- id: observed\n  set: {output: observed, value: '41'}\n";
    let choose = compound("choose", "middle", leaf)?;
    let outer = compound(
        "repeat",
        "outer",
        &format!("{choose}- id: tail\n  set: {{output: tail, value: '41'}}\n"),
    )?;
    Ok(format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_effects_repeat_choose_wait\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: number, max: 1}}}}\nsteps:\n  - id: delay\n    set: {{output: delay, value: '1'}}\n{}  - id: check\n    set: {{output: check, value: '$step.outer + $step.outer'}}\n  - id: done\n    finish: {{result: '$check'}}\n",
        indent(&outer, 2)
    ))
}

fn fire_bound(timer: &TimerEntry) -> Duration {
    timer.deadline.saturating_duration_since(Instant::now())
}

fn wait_for_deadline(timer: &TimerEntry) {
    let remaining = fire_bound(timer);
    if remaining > Duration::ZERO {
        std::thread::sleep(remaining);
    }
}

fn fire_pending(runtime: &Runtime, run: RunId) -> TestResult<bool> {
    let Ok(entry) = runtime.capture_timer_entry(run) else {
        return Ok(false);
    };
    assert_eq!(entry.kind, PendingTimerKind::Wait);
    wait_for_deadline(&entry);
    runtime.timer_entry_fired(entry)?;
    Ok(true)
}

#[test]
fn owned_repeat_choose_wait_reopens_between_timers_without_reissue() -> TestResult {
    let text = source_text()?;
    let case = Case::new(&text, RUN, &[SlotValue::I64(1)], &[Taint::Clean], &[])?;
    let bound = case.workflow.node_count().checked_mul(64).ok_or("bound")?;
    let (journal, mut runtime) = case.activate_live()?;

    let mut fired = 0_usize;
    for _ in 0..bound {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if fired == 1 {
            break;
        }
        if fire_pending(&runtime, RUN)? {
            fired = fired.checked_add(1).ok_or("fire overflow")?;
        }
    }
    assert_eq!(fired, 1, "one live Wait activation expected");
    let scheduled = count(&case.events(&journal)?, |event| {
        matches!(event, JournalEvent::WaitScheduledEvent { .. })
    });
    assert_eq!(scheduled, 1);
    drop(runtime);
    drop(journal);

    let (journal, mut runtime) = case.recover()?;
    assert_eq!(
        count(&case.events(&journal)?, |event| matches!(
            event,
            JournalEvent::WaitScheduledEvent { .. }
        )),
        1,
        "recovery must not reschedule a resolved Wait boundary"
    );
    for _ in 0..bound {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        if fire_pending(&runtime, RUN)? {
            fired = fired.checked_add(1).ok_or("fire overflow")?;
        }
    }

    let events = case.events(&journal)?;
    assert_completed(&runtime, &events);
    assert_eq!(fired, 2, "exactly two Wait activations");
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::WaitScheduledEvent { .. }
        )),
        2
    );
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::WaitResolvedEvent { .. }
        )),
        2
    );
    assert_eq!(run_finished(&events)?, (SlotValue::I64(82), Taint::Clean));
    Ok(())
}

/// Stale timer authority: a generation already resolved cannot fire again.
#[test]
fn stale_wait_timer_after_resolution_is_typed_refusal_without_journal_append() -> TestResult {
    // Given: the first wait has been durably scheduled and its timer expires.
    let text = source_text()?;
    let case = Case::new(&text, RUN, &[SlotValue::I64(1)], &[Taint::Clean], &[])?;
    let bound = case.workflow.node_count().checked_mul(64).ok_or("bound")?;
    let (journal, mut runtime) = case.activate_live()?;
    let mut first = None;
    for _ in 0..bound {
        runtime.tick_all()?;
        if let Ok(entry) = runtime.capture_timer_entry(RUN) {
            first = Some(entry);
            break;
        }
    }
    let stale = first.ok_or("live timer authority")?;
    // When: the first timer resolves and the repeated wait becomes current.
    wait_for_deadline(&stale);
    runtime.timer_entry_fired(stale)?;
    runtime.tick_all()?;

    // Then: a same-deadline stale generation is refused without evidence.
    let mut current = None;
    for _ in 0..bound {
        runtime.tick_all()?;
        match runtime.capture_timer_entry(RUN) {
            Ok(entry) => {
                current = Some(entry);
                break;
            }
            Err(vb_runtime::RuntimeError::InvalidTimerFire) => {}
            Err(error) => return Err(error.into()),
        }
    }
    let current = current.ok_or("repeated wait timer authority")?;
    assert_eq!(current.run, stale.run);
    assert_eq!(current.kind, stale.kind);
    assert_ne!(current.generation, stale.generation);
    let stale_same_deadline = TimerEntry {
        deadline: current.deadline,
        ..stale
    };
    let before = case.events(&journal)?.len();
    let refused = match runtime.timer_entry_fired(stale_same_deadline) {
        Ok(()) => return Err("resolved timer authority was accepted again".into()),
        Err(error) => error,
    };
    assert!(
        matches!(refused, vb_runtime::RuntimeError::InvalidTimerFire),
        "unexpected timer refusal: {refused:?}"
    );
    assert_eq!(
        case.events(&journal)?.len(),
        before,
        "timer refusal appended evidence"
    );
    assert_eq!(
        runtime.capture_timer_entry(RUN)?,
        current,
        "timer refusal must preserve the captured authority"
    );
    Ok(())
}
