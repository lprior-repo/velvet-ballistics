# Round 4 Agent A12 — Final Integration Assessment

**Reviewer:** black-hat-reviewer · **Date:** 2026-06-07 · **Scope:** 4 P0 blockers + runtime risk walkthrough · **STATUS: HOLD**

The system **cannot** be declared "Backend / IR Interpreter Complete" as defined in master Section 44. Three independent critical runtime defects make the durable execution engine unmerchantable as advertised.

## Bead Reconciliation

| Bead | Type | Status | Actual State |
|------|------|--------|--------------|
| **vb-1ev82** | P0 bug | `BLOCKED` | Runtime facade **restored**. `cargo check -p vb_runtime` PASSES. State 6 proof-reviewer REJECTED with `E_STATUS_NOT_APPROVED`. |
| **vb-yesh4** | P0 bug | `BLOCKED` | Discovers from vb-1ev82. **Round 2: does not reproduce in main.** Stale claim. |
| **vb-8o7p5** | P0 bug | `BLOCKED` | Kani dep graph repaired. **NEW BLOCKER:** 3 harnesses timeout at 120s in `crossbeam_queue::ArrayQueue::new` unwinding. |
| **vb-o5zb** | P0 bug | `BLOCKED` | All 5 children closed. Parent should close. |

## Top-10 Prioritized Issues

| # | Severity | Issue |
|---|----------|-------|
| 1 | **CRITICAL** | **`Wait`/`Ask` timer deadline is silently ignored.** `await_timer` in `crates/vb_runtime/src/shard/transitions.rs:171` always uses `deadline: Instant::now()`. The `deadline_slot` value is never read. |
| 2 | **CRITICAL** | **No recovery of pending wait timers after process restart.** Timer wheel + `pending_timers: IndexMap<RunId, PendingTimer>` are in-memory only. |
| 3 | **CRITICAL** | **Run header after `Cancel` shows `NotFound`, not `Cancelled`.** `snapshot_run` (`chunk_001.rs:191-199`) reads from `runs`, not `terminal_runs`. |
| 4 | **HIGH** | **`discard_journal_sequence` is called inside cancel/kill paths before the RunCancelled/RunKilled event is replayed in a consistent recovery walk.** |
| 5 | **HIGH** | **Flux annotations are universally `#[flux_rs::trusted]`.** `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` declares 12 `#[flux_rs::trusted]` model functions. |
| 6 | **HIGH** | **Kani harnesses timeout at 120s in `crossbeam_queue::ArrayQueue::new` unwinding.** |
| 7 | **HIGH** | **`compile_ir_1000_steps` benchmark is 4% over the 200,000 µs budget (208,520 µs).** |
| 8 | **MEDIUM** | **7 new files over the 300-line limit, no exceptions filed.** |
| 9 | **MEDIUM** | **Stale exception entries.** `source-length-exceptions.txt:436, :493` reference files that are now under the 300-line limit. |
| 10 | **LOW** | **`postcard_content_type` references re-introduced in test comment block.** |

## End-to-End Failure Scenarios

### Scenario A: "Wait 1 hour" — process restart mid-wait
1. `Runtime::submit_direct(run_42, workflow)` → reaches `WaitUntil { deadline_slot: 1 }` → engine returns `AwaitingWait`
2. `await_timer` uses `Instant::now()` instead of `deadline_slot` value (Bug #1)
3. Operator at T+5 min: `kill -9 $(pidof velvet-ballistics)`
4. Restart: `Runtime::new_with_journal` creates empty shards. `pending_timers` empty.
5. Operator: `velvet-ballistics resume 42` → `ResumeError::RunIdNotFound`
6. The deploy action WAS dispatched (side effect already happened) but the cleanup step is orphaned forever.

### Scenario B: `ActionScheduled` → `Kill`
- Operator calls `kill_run(77)` → `handle_kill` removes run from `runs`, adds to `terminal_runs`, increments counter, journal `RunKilled`
- Operator: `inspect 77` → returns `NotFound` (not `Cancelled`/`Killed`)
- The user has no way to know whether the side effect happened

### Scenario C: Cancel-Then-Cancel idempotency
- First `cancel_run(99)` succeeds
- Second `cancel_run(99)` returns `RunNotFound`
- `terminal_runs_contains(99)` is true, but the API surface doesn't check it

## Recovery Test (Question 5)

**Answer: NO.** The runtime cannot resume a `Wait`-suspended run after restart. Three independent defects:
1. **Deadline is `Instant::now()`** (`transitions.rs:171`)
2. **`pending_timers` is in-memory only** (`shard/timer.rs:22-27`)
3. **No `Runtime::recover()` integration** — `recovery.rs` defines `RuntimeRecoveryBoundary` but no `Runtime` method calls it

## Composite Verdict — "Backend / IR Interpreter Complete"?

Master Section 44 (`velvet-ballistics-MASTER.md:2023-2050`) defines 24 acceptance points.

### Three most damning reasons it CANNOT be called Complete

1. **Acceptance Point 3** ("Every primitive validates, compiles, runs, persists, recovers, and replays") is FALSE for `wait` and `ask`. The `wait` primitive does not persist or recover.
2. **Acceptance Point 11** ("Queues, stacks, buffers, retries, fanout, timers, traces, batches, IPC frames, and resource contracts are bounded") is FALSE for the timer wheel's interaction with restart.
3. **Acceptance Point 23** ("Full current-scope gates pass: ... benchmark build ...") is FALSE. `moon ci` exits 1. `compile_ir_1000_steps` is 4% over budget. 7 files violate the 300-line source-length rule.

### Three most damning reasons it CAN be called Complete

1. **Acceptance Point 1** (canonical spelling) is true.
2. **Acceptance Point 4-17** (most of the function-surface list) are demonstrably true.
3. **Acceptance Point 24** (closed beads) is partially true.

The 3 "cannot" reasons out-weigh the 3 "can" reasons because the failing points are **hard correctness** (timer non-durability, restart loss, gate failure), while the "can" reasons are **soft surface** (spelling, API presence, bead hygiene). A user cannot spell-correct their way out of a lost wait timer.

## Readiness Score

| Dimension | Weight | Score (0-10) | Weighted |
|-----------|--------|--------------|----------|
| Build / Clippy / Fmt | 10% | 9 | 0.9 |
| Tests run | 15% | 9 | 1.35 |
| Source length policy | 5% | 2 | 0.10 |
| Performance budget | 10% | 6 | 0.60 |
| `moon ci` canonical | 15% | 2 | 0.30 |
| Recovery / replay correctness | 20% | 0 | 0.00 |
| Kani / Verus / Flux proof closure | 10% | 3 | 0.30 |
| Bead hygiene | 5% | 7 | 0.35 |
| Documentation / spec parity | 5% | 6 | 0.30 |
| API surface completeness | 5% | 9 | 0.45 |

**Weighted Total: 4.65 / 10 = 46.5** → **47 / 100**

## Composite Final Verdict: HOLD

Do not declare "Backend / IR Interpreter Complete." Do not merge past `moon ci` RED. Do not waive the canonical gate.

The product identity (`MASTER.md:31`) is "an AI-safe, local-first, single-server durable execution engine that verifies AI-authored workflows before admission, persists an inspectable journal, protects side effects with idempotency evidence." The current state is "a runtime that loses suspended runs on restart and ignores the deadline it claims to honor." The two are not the same product.

## Required Actions Before SHIP Can Be Reconsidered

1. (CRITICAL) Fix the `deadline_slot` read in `await_timer` (`transitions.rs:171`) and persist the deadline to the journal event.
2. (CRITICAL) Add `Runtime::recover()` that re-inserts `pending_timers`, `runtime_states`, `journal_sequences`, and `terminal_runs` from Fjall.
3. (CRITICAL) Repair the 3 Kani harnesses timing out in `crossbeam_queue::ArrayQueue::new` unwinding.
4. (HIGH) Repair or retire the 7 source-length violations.
5. (HIGH) Repair the `compile_ir_1000_steps` benchmark budget breach.
6. (HIGH) Replace the vacuous `#[flux_rs::trusted]` Flux annotations with real refinements.
7. (MEDIUM) Add a `Terminal` variant to `InspectResponse`.
8. (MEDIUM) Make `Runtime::cancel_run` API-surface idempotent.
9. (LOW) Clean stale source-length exceptions.
10. (PROCESS) Reclassify vb-yesh4 as `DEFERRED` or close with `NOT-REPRODUCING-IN-MAIN` evidence; close vb-o5zb parent.
