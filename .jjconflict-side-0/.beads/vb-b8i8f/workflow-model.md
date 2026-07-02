# Workflow Model: vb-b8i8f Cancel/Kill Lattice Recovery

## Lifecycle State Machine

```text
Missing
  └─ cancel/kill -> Err(MissingRun)

Live
  ├─ finish -> Terminal(Finished) + RunFinished
  ├─ fail -> Terminal(Failed) + RunFailed
  ├─ cancel -> Terminal(Cancelled) + RunCancelled
  └─ kill -> Terminal(Killed) + RunKilled

Terminal(Finished|Failed|Cancelled|Killed)
  ├─ cancel -> Err(AlreadyTerminal or RunNotFound)
  ├─ kill -> Err(AlreadyTerminal or RunNotFound)
  ├─ action/timer/ask/resume -> Err(stale authority)
  └─ inspect/snapshot -> read-only observation
```

## Cancel Success Workflow

1. Caller invokes `Runtime::cancel_run(run)`.
2. Runtime routes `run` to owning shard.
3. Shard validates that the run is live at processing time.
4. Shard builds terminal event `RuntimeJournalEvent::RunCancelled { run, reason }`.
5. Runtime journal adapter maps it to `JournalEvent::RunCancelled { run, seq, attempt: 1, reason }`.
6. Storage append succeeds under configured durability policy.
7. Shard removes pending timer/action authority, releases frame, marks run terminal, emits trace/counter evidence, and prevents future live mutation.
8. Any subsequent cancel/kill for the run returns typed rejection and appends no event.

## Kill Success Workflow

1. Caller invokes required `Runtime::kill_run(run)`.
2. Runtime routes `run` to owning shard and enqueues/processes `ShardCommand::Kill`.
3. Shard validates live state.
4. Shard builds terminal event `RuntimeJournalEvent::RunKilled { run }`.
5. Runtime journal adapter maps it to `JournalEvent::RunKilled { run, seq, attempt: 1 }`.
6. Storage codec accepts `RecordKind::RunKilled.id() == 28` as a known `MAGIC_JOURNAL_EVENT` kind.
7. Storage append/replay can encode/decode the killed record without `RecordKindFamilyMismatch`.
8. Shard performs the same cleanup and terminal marking guarantees as cancel.

## Missing / Already-Terminal Workflow

```text
cancel_or_kill(run):
  if run not live:
    return Err(RunNotFound or AlreadyTerminal)
    append no journal event
    emit no terminal trace
    increment no terminal counter
    do not discard sequence in a way that corrupts replay
  else:
    terminalize exactly once
```

Required behavior change from fresh main: silent `Ok(())` for absent/already-terminal cancel/kill is invalid.

## Stale Authority Workflow

After `Cancelled` or `Killed`:

- `TimerFired` for prior `PendingTimer` must fail as stale/invalid and not reinsert state.
- `ActionCompleted`, `ActionFailed`, `AskAnswered`, and `Resume` must fail as stale/invalid or not-found.
- No stale command may append `SlotWritten`, `ActionCompleted`, `ActionFailed`, `WaitResolved`, `AskAnswered`, `StepSucceeded`, or terminal events.
- Snapshot/inspect remains read-only and must not synthesize live state.

## Storage Admission Workflow for `RunKilled`

```text
RuntimeJournalEvent::RunKilled
  -> JournalEvent::RunKilled
  -> RecordKind::RunKilled.id() == 28
  -> validate_known_kind(28) == Ok
  -> validate_kind_family(MAGIC_JOURNAL_EVENT, 28) == Ok
  -> encode envelope
  -> decode envelope admits kind 28
  -> replay validates contiguous EventSeq
```

## Temporal Invariants

- Terminal transition is single-winner even if cancel, kill, finish, fail, timer, and action completion are queued near each other.
- Cleanup must happen in an order that cannot leave a terminal run with a valid pending timer/action authority.
- Storage append failure must not commit terminal in-memory state as if durable terminalization succeeded under durable profiles.
