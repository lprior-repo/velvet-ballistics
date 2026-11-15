use super::*;

#[test]
fn replacement_into_full_peer_bucket_reserves_before_commit_and_preserves_due_order()
-> Result<(), String> {
    let now = Instant::now();
    let first_deadline = now
        .checked_add(std::time::Duration::from_secs(1))
        .ok_or("old deadline")?;
    let shared_deadline = now
        .checked_add(std::time::Duration::from_secs(2))
        .ok_or("shared deadline")?;
    let old = TimerEntry {
        run: RunId::new(86_201),
        generation: 14,
        deadline: first_deadline,
        kind: PendingTimerKind::Wait,
    };
    let peer_low = TimerEntry {
        run: RunId::new(86_200),
        generation: 8,
        deadline: shared_deadline,
        kind: PendingTimerKind::Wait,
    };
    let peer_high = TimerEntry {
        run: RunId::new(86_202),
        generation: 19,
        deadline: shared_deadline,
        kind: PendingTimerKind::Ask,
    };
    let replacement = TimerEntry {
        generation: 15,
        deadline: shared_deadline,
        ..old
    };
    let mut wheel = TimerWheel::new();
    for entry in [old, peer_high, peer_low] {
        wheel
            .insert_exact_generation(entry, 3)
            .map_err(|error| error.to_string())?;
    }
    let full_bucket = wheel
        .by_deadline
        .get_mut(&shared_deadline)
        .ok_or("peer bucket")?;
    full_bucket.shrink_to_fit();
    assert_eq!(full_bucket.len(), 2);
    assert_eq!(full_bucket.capacity(), full_bucket.len());
    wheel
        .prepare_entry(old.run, shared_deadline, 3)
        .map_err(|error| error.to_string())?;
    assert_eq!(wheel.get_entry(old.run), Some(old));
    assert_eq!(wheel.get_entry(peer_low.run), Some(peer_low));
    assert_eq!(wheel.get_entry(peer_high.run), Some(peer_high));
    assert_eq!(wheel.due_peek(first_deadline), Some(old));
    let prepared = wheel
        .by_deadline
        .get(&shared_deadline)
        .ok_or("reserved peer bucket")?;
    assert_eq!(prepared.as_slice(), &[peer_high, peer_low]);
    assert!(prepared.capacity() > prepared.len());
    let pointer = prepared.as_ptr();
    let capacity = prepared.capacity();
    let run_capacity = wheel.by_run.capacity();
    wheel
        .insert_exact_generation(replacement, 3)
        .map_err(|error| error.to_string())?;
    let committed = wheel
        .by_deadline
        .get(&shared_deadline)
        .ok_or("committed peer bucket")?;
    assert_eq!(committed.as_ptr(), pointer);
    assert_eq!(committed.capacity(), capacity);
    assert_eq!(wheel.by_run.capacity(), run_capacity);
    assert_eq!(committed.as_slice(), &[peer_high, peer_low, replacement]);
    assert_eq!(wheel.by_deadline.get(&first_deadline), None);
    assert_eq!(wheel.get_entry(old.run), Some(replacement));
    assert_eq!(wheel.len(), 3);
    assert_eq!(wheel.next_deadline(), Some(shared_deadline));
    assert_eq!(wheel.due_peek(first_deadline), None);
    assert_eq!(wheel.due_peek(shared_deadline), Some(peer_low));
    assert!(wheel.cancel(peer_low.run));
    assert_eq!(wheel.due_peek(shared_deadline), Some(replacement));
    assert!(wheel.cancel(replacement.run));
    assert_eq!(wheel.due_peek(shared_deadline), Some(peer_high));
    assert_eq!(wheel.fire_expired(shared_deadline), vec![peer_high]);
    assert!(wheel.is_empty());
    Ok(())
}
