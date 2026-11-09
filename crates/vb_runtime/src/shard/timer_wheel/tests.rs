use super::*;
use vb_core::ids::RunId;

fn run(id: u64) -> RunId {
    RunId::new(id)
}

#[test]
fn insert_and_cancel() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    assert_eq!(wheel.insert(run(1), now, PendingTimerKind::Wait), Ok(()));
    assert!(!wheel.is_empty());
    assert!(wheel.cancel(run(1)));
    assert!(wheel.is_empty());
}

#[test]
fn cancel_nonexistent_returns_false() {
    let mut wheel = TimerWheel::new();
    assert!(!wheel.cancel(run(99)));
}

#[test]
fn fire_expired_returns_only_past_deadlines() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let past = now - std::time::Duration::from_millis(100);
    let future = now + std::time::Duration::from_secs(60);

    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), future, PendingTimerKind::Ask), Ok(()));

    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(1));
    assert!(!wheel.is_empty());
    assert_eq!(wheel.len(), 1);
}

#[test]
fn fire_expired_drains_all_expired() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let d1 = now - std::time::Duration::from_millis(200);
    let d2 = now - std::time::Duration::from_millis(100);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Ask), Ok(()));

    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 2);
    assert!(wheel.is_empty());
}

#[test]
fn next_deadline_returns_earliest() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let early = now + std::time::Duration::from_millis(10);
    let late = now + std::time::Duration::from_millis(100);

    assert_eq!(wheel.insert(run(1), late, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), early, PendingTimerKind::Ask), Ok(()));

    assert_eq!(wheel.next_deadline(), Some(early));
}

#[test]
fn next_deadline_none_when_empty() {
    let wheel = TimerWheel::new();
    assert!(wheel.next_deadline().is_none());
}

#[test]
fn replace_existing_timer() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let d1 = now + std::time::Duration::from_millis(10);
    let d2 = now + std::time::Duration::from_millis(20);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Ask), Ok(()));

    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
    assert_eq!(wheel.next_deadline(), Some(d2));
}

#[test]
fn multiple_runs_at_same_deadline() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let deadline = now + std::time::Duration::from_millis(50);

    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    assert_eq!(
        wheel.insert(run(2), deadline, PendingTimerKind::Ask),
        Ok(())
    );
    assert_eq!(
        wheel.insert(run(3), deadline, PendingTimerKind::Wait),
        Ok(())
    );

    assert_eq!(wheel.len(), 3);
    let fired = wheel.fire_expired(deadline);
    assert_eq!(fired.len(), 3);
    assert!(wheel.is_empty());
}

#[test]
fn len_tracks_active_timers() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    assert_eq!(wheel.len(), 0);

    assert_eq!(wheel.insert(run(1), now, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.len(), 1);

    assert_eq!(wheel.insert(run(2), now, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.len(), 2);

    wheel.cancel(run(1));
    assert_eq!(wheel.len(), 1);
}

#[test]
fn get_kind_returns_correct_kind() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();

    assert_eq!(wheel.insert(run(1), now, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
    assert_eq!(wheel.get_kind(run(2)), None);
}

#[test]
fn fire_expired_at_exact_deadline_fires() {
    let mut wheel = TimerWheel::new();
    let deadline = Instant::now();

    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    let fired = wheel.fire_expired(deadline);
    assert_eq!(fired.len(), 1);
}

#[test]
fn replacement_generation_overflow_fails_closed() {
    let mut wheel = TimerWheel::new();
    let deadline = Instant::now();
    let entry = TimerEntry {
        run: run(1),
        generation: u64::MAX,
        deadline,
        kind: PendingTimerKind::Wait,
    };
    wheel.by_deadline.entry(deadline).or_default().push(entry);
    wheel.by_run.insert(run(1), entry);

    let replacement = deadline + std::time::Duration::from_secs(1);
    assert_eq!(
        wheel.insert(run(1), replacement, PendingTimerKind::Ask),
        Err(TimerWheelError::GenerationExhausted)
    );
    assert_eq!(wheel.get_entry(run(1)), Some(entry));
}

#[test]
fn default_is_empty() {
    let wheel = TimerWheel::default();
    assert!(wheel.is_empty());
}
