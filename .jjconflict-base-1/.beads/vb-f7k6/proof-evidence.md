# Proof Evidence — vb-f7k6 — State 5 Repair Attempt 3

## Obligation Mapping

- `PO-001`: TLA bounded overflow/no wrap — PASS via main TLC.
- `PO-002`: TLA replacement bi-index/generation — PASS via main TLC.
- `PO-003`: TLA cancel index removal — PASS via main TLC.
- `PO-004`: TLA due-only fire/removal/deadlock/progress — PASS via main TLC.
- `PO-005`: TLA stale `TimerFired` rejection non-vacuity — PASS for safety via main TLC; mechanical reachability probes cover `ValidDelivery`, `StaleAfterCancel`, `StaleAfterReplace`, `WrongGeneration`, `WrongDeadline`, `WrongKind`.
- `PO-006`: TLA terminal/cancelled/shutdown no mutation/no resurrection — PASS for safety via main TLC; mechanical reachability probe covers `TerminalRejected`.
- `PO-007`: Loom captured fire vs cancel/replace/terminal outcome lattice — PASS, target-design pre-implementation.
- `PO-008`: runtime parity scoped timer tests — PASS and persisted to `.beads/vb-f7k6/test-report.md`; current regression baseline only.
- `PO-011`: production freshness authority binding — NOT RUN / NOT IMPLEMENTED in State 5; required State 10 production obligation.

## Authority Mismatch Disposition

State 4 chose target-design freshness metadata/token authority for State 10. Therefore:

- TLA and Loom stale-after-replace evidence is **target-design pre-implementation**.
- This packet does **not** claim current production `RunId`-only timer delivery is bound to `(generation, deadline, kind)`.
- Runtime tests are retained as a current regression baseline only.
- State 10 must implement and prove `PO-011`: carry or derive freshness metadata/token equivalent to `(generation, deadline, kind)` and validate it before mutation.

## Command Evidence

### `PO-001`..`PO-006` — Main TLC

Command:

```bash
tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla
```

Status: PASS, exit code `0`.

Key output:

```text
Checking temporal properties for the complete state space with 315211 total distinct states at (2026-05-18 13:54:36)
Model checking completed. No error has been found.
4209522 states generated, 315211 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 16.
Finished in 12s at (2026-05-18 13:54:37)
```

Checked invariants/properties: `TypeOK`, `NoDeadlineWrap`, `OneActiveTimerPerRun`, `BiIndexConsistent`, `CancelRemovesAllIndexes`, `ReplaceRemovesOldGeneration`, `DueOnlyFires`, `FireRemovesReturned`, `StaleFireNoMutation`, `TerminalNoTimerMutation`, `NoResurrectionAlways`, `OverflowEventuallySuspended`, `DueTimerEventuallyFireable`, deadlock check.

### `PO-005`/`PO-006` — TLC Coverage Probes

Command pattern:

```bash
tlc -config verification/tla/TimerWheelCoverage<CoverageItem>.cfg verification/tla/TimerWheel.tla
```

Status: PASS as reachability evidence. Each probe intentionally configures a `Missing*` invariant and expects TLC exit code `12` with an invariant violation showing the item reached.

```text
TimerWheelCoverageValidDelivery.cfg: Error: Invariant MissingValidDelivery is violated; coverage = {"ValidDelivery"}; exit=12.
TimerWheelCoverageStaleAfterCancel.cfg: Error: Invariant MissingStaleAfterCancel is violated; coverage = {"StaleAfterCancel"}; exit=12.
TimerWheelCoverageStaleAfterReplace.cfg: Error: Invariant MissingStaleAfterReplace is violated; coverage = {"StaleAfterReplace"}; exit=12.
TimerWheelCoverageWrongGeneration.cfg: Error: Invariant MissingWrongGeneration is violated; coverage = {"WrongGeneration"}; exit=12.
TimerWheelCoverageWrongDeadline.cfg: Error: Invariant MissingWrongDeadline is violated; coverage = {"WrongDeadline"}; exit=12.
TimerWheelCoverageWrongKind.cfg: Error: Invariant MissingWrongKind is violated; coverage = {"WrongKind"}; exit=12.
TimerWheelCoverageTerminalRejected.cfg: Error: Invariant MissingTerminalRejected is violated; coverage = {"TerminalRejected"}; exit=12.
```

### `PO-007` — Loom

Command:

```bash
cargo xtask loom --model timer_fired_cancel
```

Status: PASS, exit code `0`.

Key output:

```text
Running loom model: timer_fired_cancel
Command: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel
running 3 tests
test models::loom::timer_fired_cancel::timer_fired_replace_ordering ... ok
test models::loom::timer_fired_cancel::timer_fired_cancel_ordering ... ok
test models::loom::timer_fired_cancel::timer_fired_terminal_ordering ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1443 filtered out; finished in 0.00s
PASS: Loom model 'timer_fired_cancel' completed successfully
```

### `PO-008` — Runtime parity

Command:

```bash
/usr/bin/env cargo test -p vb_runtime timer
```

Status: PASS, exit code `0`. Durable evidence path: `.beads/vb-f7k6/test-report.md`.

Key output:

```text
running 66 tests
test result: ok. 66 passed; 0 failed; 0 ignored; 0 measured; 1369 filtered out; finished in 0.00s

running 1 test
test timer_fired_persists_before_ack ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s
```

## Assumptions / Bounds / Simplifications

- TLA time domain is finite: `TIMES = 0..1`.
- TLA duration domain is finite and includes overflow witness: `DURATIONS = 0..2`.
- TLA generation domain is finite: `GENERATIONS = 0..2`.
- TLA kinds are finite and include wrong-kind witness: `{Wait, Ask}`.
- TLA run domain is one run (`{r1}`); obligations are per-run and bi-index invariants still bind run/deadline projections.
- TLA event sets are bounded to at most one pending, delivered, and rejected event.
- TLA wrong-generation event injection uses older generation (`g < e.gen`).
- Coverage probes use expected invariant violations as existential reachability evidence; they are separate from the main all-behaviors safety/liveness pass.
- Loom model abstracts the production shard to the target authority tuple `(generation, deadline, kind)` plus terminal flag.
- Loom model covers one captured event and a single run; the outcome lattice is per captured event.
- No production code or normal tests were edited.

## Waivers / Blockers

- No State 5 tooling blocker remains.
- Production authority binding remains an explicit downstream blocker for implementation acceptance until `PO-011` is completed in State 10.
