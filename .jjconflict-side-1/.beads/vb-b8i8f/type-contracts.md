# Type Contracts: vb-b8i8f Cancel/Kill Lattice Recovery

## Desired Type Shape

These are contracts for downstream implementation/proof planning, not implementation code.

```text
RunLifecycle = Live(LiveRunState) | Terminal(TerminalState)
TerminalState = Finished | Failed | Cancelled | Killed
LifecycleCommand = Cancel { run, reason } | Kill { run, reason }
LifecycleError = MissingRun | AlreadyTerminal | QueueFull | StorageAppendFailed | InternalInvariantViolation
Terminalization = Terminalized { run, terminal_kind } | Rejected { run, error }
```

## Public API Contracts

| API | Precondition | Success postcondition | Rejection postcondition |
|---|---|---|---|
| `Runtime::cancel_run(run)` | `run` routes to existing shard; queue has capacity; run is live when processed. | Enqueues/executes a cancel command that reaches `Cancelled` exactly once. | Missing/already-terminal returns typed error; no terminal event appended. |
| `Runtime::kill_run(run)` | Same as cancel, but terminal kind is `Killed`. | Enqueues/executes a kill command that reaches `Killed` exactly once. | Missing/already-terminal returns typed error; no terminal event appended. |
| `Runtime::snapshot_run(run, correlation)` | Any run identity. | Must not resurrect terminal runs; terminal/missing snapshots remain observational only. | No state mutation. |

## Shard Internal Type Contracts

### Live/Terminal Partition

- A `RunId` must not be simultaneously present in live `runs` and terminal marker state.
- Terminal marker state must be sufficient to reject second terminalization.
- If the implementation keeps a set rather than typed `TerminalKind`, proof/test lanes must bridge from journal evidence to terminal kind.

### Terminalization Function Contract

Conceptual pure core:

```text
terminalize(lifecycle, command) -> Result<(terminal_kind, cleanup_plan, journal_event), LifecycleError>
```

Required invariants:

- `Live + Cancel -> Cancelled + RunCancelled`.
- `Live + Kill -> Killed + RunKilled`.
- `Terminal(_) + Cancel/Kill -> AlreadyTerminal`.
- `Missing + Cancel/Kill -> MissingRun`.
- Success emits exactly one terminal journal event.
- Rejection emits no journal event and performs no cleanup that would hide evidence.

### Pending Authority Cleanup

On successful cancel/kill:

- Remove `pending_timers[run]` if present.
- Invalidate action/ask/timer authority for the run by removing live state and terminal-marking before any subsequent authority can be accepted.
- Release frame exactly once.
- Discard journal sequence only after the terminal event append succeeds, or preserve sequence state on append failure so retry/recovery does not create a corrupt ledger.

## Storage Type Contracts

| Symbol | Contract |
|---|---|
| `RecordKind::RunKilled` | Stable ID `28`. |
| `JournalEvent::RunKilled` | Carries `run`, `seq`, `attempt`; `attempt > 0`. |
| `JournalEvent::record_kind()` | Returns `RecordKind::RunKilled` for killed events. |
| `is_known_record_kind(28)` | Must be true. |
| `validate_kind_family(MAGIC_JOURNAL_EVENT, 28)` | Must return `Ok(())`. |
| Decode validation | Must admit kind 28 for journal envelopes before postcard decode. |

## Error Surface Contract

| Domain error | Existing candidate | Notes |
|---|---|---|
| Missing live run | `RuntimeError::RunNotFound` | Applies to never-seen and already-terminal if API intentionally hides terminal existence. |
| Already terminal | `RuntimeError::RunNotFound` or new explicit variant | Must not return `Ok(())`. |
| Queue full | `RuntimeError::QueueFull` | Public API enqueue failure. |
| Storage append failed | `RuntimeError::StorageJournalAppend` | Must prevent terminal state commit unless implementation proves atomic rollback/ordering. |
| Record kind mismatch | `JournalError::RecordKindFamilyMismatch` | Must no longer occur for `RunKilled=28`. |

## Illegal States to Make Unrepresentable

- `RunId` both live and terminal.
- Terminal run accepting cancel/kill as successful no-op.
- Cancel/kill success without exactly one terminal journal event.
- `RunKilled` runtime event that cannot be encoded by storage.
- Stale timer/action/ask authority mutating a killed/cancelled run.
- Journal sequence discarded before a failed terminal event append is accounted for.
