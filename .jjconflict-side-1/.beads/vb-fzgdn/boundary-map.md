# Boundary Map: vb-fzgdn

## Pure core
- Smart constructors for timer value objects.
- Checked deadline arithmetic.
- Full authority equality predicate.
- Duplicate-key classifier.
- Deterministic deadline ordering comparator.
- Pure admission planning returning plan or typed error.

## Imperative shard shell
- Owns pending timers, delayed-action key index, current logical tick, queues, and run frames.
- Applies admission plans only after pure validation.
- Removes pending entry only after full fire authority validation and downstream capacity checks.
- Journals numeric schedule/fire facts.

## Public Rust API boundary
- Exposes deterministic numeric schedule/clock seam.
- Converts primitives into value objects at API edge.
- Keeps run-only timer fire fail-closed.
- Any legacy `Instant` facade must be compatibility-only and fenced from replay-critical path.

## Workflow slot boundary
- `WaitUntil { deadline_slot }` reads absolute numeric deadline.
- `WaitEvent { timeout_slot }` and `Ask { timeout_slot }` read relative duration unless compiler normalizes.
- Invalid slot values reject before timer registration mutation.

## Time authority boundary
Host wall clock, OS timers, async sleep, and `Instant::now()` are outside deterministic core. Adapters may translate host wakeups to `AdvanceClockTo(TimerTick)`, but replay uses recorded ticks.

## Storage/journal boundary
Journals/snapshots store numeric tick/deadline/generation/kind/key facts. Recovery rebuilds timers without host time.

## Async/concurrency boundary
Shard remains single-owner. External drivers enqueue commands only. Multiple producers are serialized by bounded command queues and monotonic tick guard.

## FFI/unsafe/parser boundary
No first-party unsafe. IPC/public inputs validate envelope/length before value-object construction. Fuzz/proptest surfaces are constructors and classifiers.
