# RS-213-core-shard-public-runs-invariant-bypass: Public `Shard::runs` allows external lifecycle corruption

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/config.rs:213`
- **Confidence**: confirmed

## Description
`Shard::runs` is public while the companion lifecycle maps are `pub(crate)`. External callers can mutate active run state directly without updating `runtime_states`, timers, journal sequences, terminal outcomes, or admission bookkeeping.

## Evidence
```rust
213: pub struct Shard {
214:     pub(crate) command_queue: ShardCommandQueue,
215:     pub runs: IndexMap<RunId, RunState>,
216:     /// Per-run lifecycle state tracking for resume eligibility.
217:     pub(crate) runtime_states: IndexMap<RunId, RuntimeState>,
...
225:     pub(crate) terminal_runs: LruRing<RunId>,
226:     /// Recorded terminal outcome per run id, populated when a run is moved
227:     /// into `terminal_runs` via cancel/kill/finish/fail.
228:     pub(crate) terminal_outcomes: IndexMap<RunId, TerminalOutcome>,
229:     /// Next durable journal sequence by run, owned by this shard.
230:     pub(crate) journal_sequences: IndexMap<RunId, EventSeq>,
231:     pub(crate) pending_timers: IndexMap<RunId, PendingTimer>,
```

The shard lifecycle depends on multiple maps staying in lockstep. A public `runs` field lets code remove a run from the active map while leaving it resumable, timed, or journal-sequenced elsewhere, or insert a run that lacks the required side-table state.

## Adversarial Check
This is not a mere encapsulation preference. The neighboring fields prove that run membership is an invariant spanning several private structures. Making only `runs` public exposes exactly the mutable state that those private structures are supposed to coordinate.

## Suggested Fix
Change `runs` to `pub(crate)` or private. Expose read-only inspection methods and mutation methods that update all lifecycle side tables atomically.
