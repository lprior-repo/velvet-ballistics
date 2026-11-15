use super::*;

#[test]
fn preparation_keeps_staging_invisible_and_capacity_refusal_atomic() -> Result<(), String> {
    let now = Instant::now();
    let later = now
        .checked_add(std::time::Duration::from_secs(60))
        .ok_or("later deadline")?;
    let second_stage = now
        .checked_add(std::time::Duration::from_secs(1))
        .ok_or("staged deadline")?;
    let live = TimerEntry {
        run: RunId::new(86_001),
        generation: 17,
        deadline: later,
        kind: PendingTimerKind::Wait,
    };
    let staged = RunId::new(86_002);
    let mut wheel = TimerWheel::new();
    wheel
        .insert_exact_generation(live, 2)
        .map_err(|error| error.to_string())?;
    wheel
        .prepare_entry(staged, now, 2)
        .map_err(|error| error.to_string())?;
    assert_eq!(wheel.get_entry(staged), None);
    assert_eq!(wheel.get_entry(live.run), Some(live));
    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.next_deadline(), Some(later));
    assert_eq!(wheel.due_peek(now), None);
    assert_eq!(wheel.due_peek(later), Some(live));
    assert_eq!(wheel.prepared_bucket, Some(now));
    assert_eq!(wheel.by_deadline.len(), 2);
    wheel
        .prepare_entry(staged, second_stage, 2)
        .map_err(|error| error.to_string())?;
    assert_eq!(wheel.by_deadline.get(&now), None);
    assert_eq!(wheel.by_deadline.len(), 2);
    assert_eq!(
        wheel
            .by_deadline
            .values()
            .filter(|entries| entries.is_empty())
            .count(),
        1
    );
    assert_eq!(wheel.prepared_bucket, Some(second_stage));
    assert_eq!(wheel.get_entry(staged), None);
    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.next_deadline(), Some(later));
    assert_eq!(wheel.due_peek(second_stage), None);
    let before_runs = wheel.by_run.clone();
    let before_deadlines = wheel.by_deadline.clone();
    let before_prepared = wheel.prepared_bucket;
    assert_eq!(
        wheel.prepare_entry(RunId::new(86_003), now, 1),
        Err(crate::RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
    assert_eq!(wheel.by_run, before_runs);
    assert_eq!(wheel.by_deadline, before_deadlines);
    assert_eq!(wheel.prepared_bucket, before_prepared);
    Ok(())
}

#[test]
fn exact_commit_reuses_prepared_storage_and_preserves_supplied_generation() -> Result<(), String> {
    let entry = TimerEntry {
        run: RunId::new(86_010),
        generation: 913,
        deadline: Instant::now(),
        kind: PendingTimerKind::Ask,
    };
    let mut wheel = TimerWheel::new();
    wheel
        .prepare_entry(entry.run, entry.deadline, 1)
        .map_err(|error| error.to_string())?;
    let prepared = wheel
        .by_deadline
        .get(&entry.deadline)
        .ok_or("prepared bucket")?;
    assert!(prepared.is_empty());
    let bucket_pointer = prepared.as_ptr();
    let bucket_capacity = prepared.capacity();
    let run_capacity = wheel.by_run.capacity();
    wheel
        .insert_exact_generation(entry, 1)
        .map_err(|error| error.to_string())?;
    let committed = wheel
        .by_deadline
        .get(&entry.deadline)
        .ok_or("committed bucket")?;
    assert_eq!(committed.as_ptr(), bucket_pointer);
    assert_eq!(committed.capacity(), bucket_capacity);
    assert_eq!(wheel.by_run.capacity(), run_capacity);
    assert_eq!(committed.as_slice(), std::slice::from_ref(&entry));
    assert_eq!(wheel.by_run.len(), 1);
    assert_eq!(wheel.by_deadline.len(), 1);
    assert_eq!(wheel.get_entry(entry.run), Some(entry));
    assert_eq!(wheel.next_deadline(), Some(entry.deadline));
    assert_eq!(wheel.due_peek(entry.deadline), Some(entry));
    Ok(())
}

#[test]
fn same_deadline_replacement_reuses_bucket_and_removes_old_authority_once() -> Result<(), String> {
    let old = TimerEntry {
        run: RunId::new(86_020),
        generation: 71,
        deadline: Instant::now(),
        kind: PendingTimerKind::Wait,
    };
    let replacement = TimerEntry {
        generation: 812,
        kind: PendingTimerKind::Ask,
        ..old
    };
    let mut wheel = TimerWheel::new();
    wheel
        .insert_exact_generation(old, 1)
        .map_err(|error| error.to_string())?;
    wheel
        .prepare_entry(old.run, old.deadline, 1)
        .map_err(|error| error.to_string())?;
    let prepared = wheel.by_deadline.get(&old.deadline).ok_or("owned bucket")?;
    let bucket_pointer = prepared.as_ptr();
    let bucket_capacity = prepared.capacity();
    let run_capacity = wheel.by_run.capacity();
    wheel
        .insert_exact_generation(replacement, 1)
        .map_err(|error| error.to_string())?;
    let committed = wheel
        .by_deadline
        .get(&old.deadline)
        .ok_or("replacement bucket")?;
    assert_eq!(committed.as_ptr(), bucket_pointer);
    assert_eq!(committed.capacity(), bucket_capacity);
    assert_eq!(wheel.by_run.capacity(), run_capacity);
    assert_eq!(committed.as_slice(), std::slice::from_ref(&replacement));
    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.by_deadline.len(), 1);
    assert_eq!(wheel.get_entry(old.run), Some(replacement));
    assert_eq!(wheel.due_peek(old.deadline), Some(replacement));
    assert_eq!(wheel.fire_expired(old.deadline), vec![replacement]);
    assert_eq!(wheel.fire_expired(old.deadline), Vec::<TimerEntry>::new());
    assert_eq!(wheel.get_entry(old.run), None);
    assert_eq!(wheel.next_deadline(), None);
    assert!(wheel.by_run.is_empty());
    assert!(wheel.by_deadline.is_empty());
    Ok(())
}
