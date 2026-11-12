# Workflow Model: vb-fzgdn

## Lifecycle
```text
NoTimer --validated schedule--> Pending
Pending --identical duplicate key--> Pending (same authority, no semantic mutation)
Pending --replacement/cancel policy--> Superseded
Pending --valid authority fire at/after deadline--> Fired
Pending --invalid/stale authority fire--> Pending unchanged
Pending --run terminal lifecycle--> Cancelled
Superseded --prior authority fire--> current state unchanged
Fired/Cancelled --duplicate fire--> terminal no-op or typed stale error, no resurrection
```

## Admission workflow
1. Receive schedule command with run, step, kind, numeric delay/deadline, optional delayed-action key.
2. Parse all numeric inputs into value objects.
3. Validate run exists and step is a timer-registering IR node (`WaitUntil`, `WaitEvent`, `Ask`, retry/delayed-action extension).
4. Decode slot-derived time value before mutation.
5. Compute/validate deadline with checked arithmetic.
6. Classify duplicate key: new, identical, or conflict.
7. Check capacity/reservation and generation successor.
8. Produce pure `TimerAdmissionPlan`.
9. Commit registry mutation and numeric schedule journal atomically with shard state.
10. Return authority derived from committed pending entry.

## Clock advance workflow
1. Receive `AdvanceClockTo { tick }`.
2. Reject `tick < current_tick` before mutation.
3. Set current tick.
4. Select entries with `deadline <= current_tick` in deterministic order.
5. Enqueue/apply fire commands carrying full authorities.

## Fire workflow
1. Receive `FireTimer { authority }`.
2. Lookup pending entry.
3. Compare full authority: run, step, generation, deadline, kind.
4. On mismatch/missing/not-yet-due, return typed error with no removal, journal success, enqueue, or frame advancement.
5. On valid authority, ensure downstream delayed-action/action queue capacity where applicable.
6. Remove pending entry and advance the run according to timer kind.
7. Journal numeric fire/resolution evidence.

## Outcomes
- `Scheduled(authority)`
- `AlreadyScheduled(authority)`
- `Rejected(error)`
- `Fired(outcome)`
- `StaleRejected(error)`
- `Cancelled`

## Guards
Validation-before-mutation, checked addition, monotonic clock, deterministic tie-break, bounded indexes, and replay-sufficient numeric journal facts are mandatory.
