# Domain Model: vb-b8i8f Cancel/Kill Lattice Recovery

## Scope

- Bead: `vb-b8i8f`
- State: 3 / `rust-contract`
- Feature slice: public cancel/kill lifecycle, terminal journal uniqueness, stale action/timer cleanup, and storage admission for `RunKilled` record kind family.
- Out of scope for this state: production Rust implementation, tests, verifier artifacts, proof obligations, and proof review approval.

## Ubiquitous Language

| Term | Meaning | Contract relevance |
|---|---|---|
| Run | A shard-owned execution instance identified by `RunId`. | All lifecycle commands target exactly one run on its owning shard. |
| Live run | A run present in shard live state (`runs`) and not terminal. | Only live runs may accept cancel/kill and stale external authority events. |
| Terminal run | A run that has reached exactly one terminal outcome: finished, failed, cancelled, or killed. | Terminal state is absorbing; no later command may append another terminal journal event. |
| Cancel | Caller-requested graceful terminalization of a live run. | Public API operation; may preserve an optional reason. |
| Kill | Caller/runtime-requested hard terminalization of a live run. | Must have a public `Runtime::kill_run` facade for acceptance tests and users. |
| Terminal journal event | Durable event representing the single terminal winner for a run. | Exactly one of `RunFinished`, `RunFailed`, `RunCancelled`, `RunKilled`. |
| Stale authority | Timer fire, ask answer, action completion/failure, or resume command that refers to a run after terminalization. | Must be rejected with typed errors and must not mutate state or append journal events. |
| Pending timer | A scheduled wait/ask authority keyed by run and generation/deadline/kind. | Cancel/kill must remove it atomically with terminalization. |
| Action ticket | Authority for external action completion/failure. | Cancel/kill invalidates all live action authority for the run. |
| Record kind family | Storage envelope validation tying a `record_kind_u16` to a magic family. | `RunKilled = 28` must be known and admitted as a journal event. |

## Aggregates and Entities

### Runtime Aggregate

- Routes public operations by `RunId` to exactly one shard.
- Owns public lifecycle API surface.
- Contract change: `Runtime::kill_run(&self, run: RunId) -> RuntimeResult<()>` is required beside `Runtime::cancel_run`.
- Public cancel/kill must not report success when the run is missing or already terminal.

### Shard Aggregate

- Owns mutable run state, terminal marker set, pending timers, journal sequence state, and trace ring.
- Serializes commands through bounded queue/tick processing.
- Must enforce live-only terminalization and absorbing terminal semantics.

### Run Lifecycle Entity

- Identity: `RunId`.
- State partition: `Live` or one terminal variant; never both.
- Terminal variants: `Finished`, `Failed`, `Cancelled`, `Killed`.
- The actual implementation may store only a terminal marker set today, but the domain contract requires preserving enough information to prove terminal uniqueness and stale authority rejection.

### Journal Event Entity

- Runtime event: `RuntimeJournalEvent::RunCancelled { run, reason }` or `RuntimeJournalEvent::RunKilled { run }`.
- Storage event: `vb_storage::JournalEvent::RunCancelled { run, seq, attempt, reason }` or `JournalEvent::RunKilled { run, seq, attempt }`.
- Sequence: monotonic per run and contiguous across replay-visible events.

## Value Objects

| Value object | Invariant |
|---|---|
| `RunId` | Numeric identity; no string parsing in runtime core. |
| `LiveRunId` | Internal proof concept: a `RunId` known to be present in live shard state and absent from terminal set. |
| `TerminalRunId` | Internal proof concept: a `RunId` known to have a terminal marker/event. |
| `TerminalKind` | Closed domain set: `Finished`, `Failed`, `Cancelled`, `Killed`. |
| `LifecycleCommandKind` | `Cancel` or `Kill`; must not be a boolean behavior flag. |
| `TerminalizationOutcome` | `Terminalized { kind }` or typed rejection; no silent no-op success. |
| `RecordKindId` | `u16` storage kind; `28` is the `RunKilled` kind. |
| `JournalFamily` | Magic `MAGIC_JOURNAL_EVENT` admits all runtime journal record kinds including `RunKilled=28`. |

## Policies

1. **Live-only command policy:** cancel/kill may only transition a live run. Missing and already-terminal runs return typed rejection and append no events.
2. **Single terminal winner policy:** exactly one terminal journal event may be appended for any run. A second terminal command is a typed rejection, not idempotent success.
3. **Cleanup-before-observability policy:** cancel/kill must remove pending timer/action authority and release the live frame before exposing terminal state.
4. **No stale mutation policy:** stale action/timer/ask/resume commands after cancel/kill must not mutate frames, slots, counters, timers, trace, or journal.
5. **Durable admission policy:** runtime `RunKilled` maps to storage `RunKilled` and storage codec validation must accept kind `28` as a known journal-family record on encode and decode.

## Domain Decisions

- Public kill is required: shard-only kill is insufficient for the requested public lattice recovery.
- Missing and already-terminal cancel/kill share the same externally safe semantic: typed not-found/terminal rejection. Existing `RuntimeError::RunNotFound` may serve this surface if no more precise error is added; a distinct `RunAlreadyTerminal { run }` would be more explicit but is not required by this contract.
- Cancel and kill are both terminal failure-like outcomes for counters unless a later product decision introduces separate cancellation/kill metrics. Counter behavior must not affect terminal uniqueness.
- `RunKilled=28` is normative for this bead even though the master storage table currently omits ID 28; downstream implementation should repair validation and a later documentation alignment bead may update the table if required.
