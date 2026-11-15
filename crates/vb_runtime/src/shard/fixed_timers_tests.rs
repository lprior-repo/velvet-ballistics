use super::*;
use crate::shard::types::PendingTimerKind;

fn timer(deadline: Instant, generation: u64, step: u16) -> PendingTimer {
    PendingTimer {
        step: StepIdx::new(step),
        kind: PendingTimerKind::Wait,
        generation,
        deadline,
    }
}

#[test]
fn admitted_timer_storage_reuses_fixed_layout_across_full_replacement_remove_and_clear()
-> Result<(), String> {
    let mut timers = AdmittedTimers::try_new(2).map_err(|error| error.to_string())?;
    let layout = (
        timers.entries.as_ptr(),
        timers.entries.len(),
        timers.steps.as_ptr(),
        timers.steps.len(),
    );
    let first = RunId::new(87_001);
    let second = RunId::new(87_002);
    let deadline = Instant::now();
    for generation in 1..=64 {
        let original = timer(deadline, generation, 1);
        assert_eq!(timers.insert(second, original, 2), Ok(None));
        assert_eq!(timers.insert(first, original, 2), Ok(None));
        assert_eq!(
            timers.due_peek(deadline).map(|entry| entry.run),
            Some(first)
        );
        let replacement = timer(deadline, generation.checked_add(1).ok_or("generation")?, 3);
        assert_eq!(timers.prepare(first, replacement, 2), Ok(()));
        assert_eq!(timers.get(first), Some(original));
        assert_eq!(timers.insert(first, replacement, 2), Ok(Some(original)));
        assert_eq!(
            timers.iter().map(|(run, _)| run).collect::<Vec<_>>(),
            vec![second, first]
        );
        assert_eq!(timers.remove(second), Some(original));
        assert_eq!(
            timers.iter().collect::<Vec<_>>(),
            vec![(first, replacement)]
        );
        timers.clear();
        assert_eq!(timers.len(), 0);
        assert_eq!(timers.due_peek(deadline), None);
        assert!(timers.entries.iter().all(Option::is_none));
        assert!(timers.steps.iter().all(Option::is_none));
        assert_eq!(
            (
                timers.entries.as_ptr(),
                timers.entries.len(),
                timers.steps.as_ptr(),
                timers.steps.len()
            ),
            layout
        );
    }
    Ok(())
}

#[test]
fn admitted_timer_refusal_is_atomic_and_replacement_reorders_without_extra_slot()
-> Result<(), String> {
    let mut timers = AdmittedTimers::try_new(2).map_err(|error| error.to_string())?;
    let now = Instant::now();
    let future = now
        .checked_add(std::time::Duration::from_secs(1))
        .ok_or("deadline")?;
    let first = RunId::new(87_010);
    let second = RunId::new(87_011);
    let original = timer(now, u64::MAX, 1);
    let later = timer(future, 3, 2);
    assert_eq!(timers.insert(first, original, 2), Ok(None));
    assert_eq!(timers.insert(second, later, 2), Ok(None));
    let before = (timers.entries.clone(), timers.steps.clone(), timers.len);
    assert_eq!(
        timers.insert(RunId::new(87_012), original, 2),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 })
    );
    assert_eq!(
        (&timers.entries, &timers.steps, timers.len),
        (&before.0, &before.1, before.2)
    );
    let after_both = future
        .checked_add(std::time::Duration::from_secs(1))
        .ok_or("later deadline")?;
    let replacement = timer(after_both, 0, 4);
    assert_eq!(timers.insert(first, replacement, 2), Ok(Some(original)));
    assert_eq!(timers.due_peek(now), None);
    assert_eq!(timers.due_peek(future).map(|entry| entry.run), Some(second));
    assert_eq!(timers.remove(second), Some(later));
    assert_eq!(
        timers.due_peek(after_both).map(|entry| entry.generation),
        Some(0)
    );
    assert_eq!(
        timers.iter().collect::<Vec<_>>(),
        vec![(first, replacement)]
    );
    Ok(())
}

#[test]
fn admitted_timer_constructor_capacity_overflow_returns_existing_typed_error() {
    assert!(matches!(AdmittedTimers::try_new(usize::MAX),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity }) if capacity == usize::MAX));
}

#[test]
fn admitted_timer_zero_capacity_preserves_direct_shard_constructor_behavior() -> Result<(), String>
{
    let mut timers = AdmittedTimers::try_new(0).map_err(|error| error.to_string())?;
    assert_eq!(timers.entries.len(), 0);
    assert_eq!(timers.steps.len(), 0);
    assert_eq!(
        timers.insert(RunId::new(87_020), timer(Instant::now(), 1, 0), 0),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 0 })
    );
    timers.clear();
    assert_eq!(timers.len(), 0);
    assert_eq!(timers.due_peek(Instant::now()), None);
    let shard = crate::shard::Shard::new(crate::shard::ShardConfig {
        max_active_runs: 0,
        ..crate::shard::ShardConfig::default()
    })
    .map_err(|error| error.to_string())?;
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}
