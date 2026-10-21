**Bead**: vb-9pycz (discovered-from vb-aaqp7)  
**State**: independent CODE review  
**Reviewer**: black-hat-reviewer with async-rust-reviewer / Holzmann lenses  
**Source checkout**: /home/lewis/src/isoloated/velvet-ballistics-vb-aaqp7-code-review-sol-20261021  
**Attempt**: 1

## Gate Result

**STATUS: APPROVED** — scoped shutdown delta only.

Exact source `6a5a4d4e9861e6dc989362b496de1b6d47b4b939`, bookmark `codex/vb-aaqp7-shutdown-candidate`, parent `862543ed44c354846235f6676675ed86b3f19add`. Review excludes evolving children, broad CI and performance acceptance. Inherited IPC function-size findings from vb-t3cp3 remain open in vb-2a9i3 and are explicitly outside this assigned shutdown delta. This is not approval of the entire ancestry or full v1.

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|---|---|---|
| Actual production shutdown, synchronous mio/core contract | Pass | Runtime delegates to runtime/shutdown.rs; no async executor/task-region retrofit. Master36 requires graceful drain or reported remaining; master50 preserves pending runs. |
| Typed success/failure with meaningful state | Pass | Opaque ShutdownReport getters expose commands drained, exact remaining totals with bounded details, pre-release timer registrations and journal report. ShutdownFailure exposes phase, original RuntimeError via Error::source, completed-shard prefix and actual current boundary snapshot. |
| Quiesce before durable drain, release only after success | Pass | shutdown.rs:98-102 invokes quiesce, then fallible drain_journal, then release_timers. Timer identities/deadlines survive actual DuplicateEvent journal failure and one-shot port failure. |
| Bounded command work and actual remaining commands | Pass | quiesce_for_shutdown processes at most queue length on entry, marks stopped after success, preserves commands after a prequeued Shutdown sentinel and reports their depth. Success means quiescence/durable drain, not all runs completed. |
| Suspended frame/action/ask/timer accounting | Pass in scoped source/tests | Existing production pending_boundary_snapshot supplies actual totals and at most64 details per category/shard with explicit truncation; new tests inspect real waiting run/timer and actual action ticket/frame. Ask snapshot uses existing implementation; no new ask semantics claimed. |
| Atomic batch durability and ownership | Pass | shutdown_durable closes admission; each flush_batch_durable selects SyncAll, including explicit empty/idempotent barrier, before removing deque entries. Failed batch remains owned; successful prefix may retire durably. |
| Retry after partial durable failure | Pass | Independent later-batch barrier probe described below; production runtime journal-port retry also passes. |
| Success Ack only after checked shutdown | Pass | Both active and compatibility IPC handler paths map only Ok(report) to ShuttingDown. Actual shared65-record CLI tests assert complete queue drain before correlated Ack and RuntimeError/typed Storage error on conflicting65th event. |
| No forced cancellation | Pass | Suspended runtime/tickets retained; production tests exclude synthetic RunCancelled/ActionAbandoned/RunFinished for suspended work. |

### Independent partial-progress counterexample probe

`partial_durable_shutdown_probe.rs` executes the actual JournalWriterQueue and FjallJournal with capacity3/batch1. Event1 is already visible from an unsynced append; queued events0/1/2 are admitted. The test-only persist hook fails the later all-idempotent barrier, after event0 has already committed through SyncAll. Shutdown returns exact StrictDurabilityFailed; two entries remain queued, new admission returns QueueShutdown, event2 is absent. Retry drains exactly two, the queue empties, repeated shutdown drains0, and all three exact events survive drop/reopen. This exercises success-prefix/failing-later-batch retention rather than only failure of the first batch.

The hook is a deterministic production-bound fault seam, not physical disk-failure or power-loss evidence. Runtime one-shot journal-port failure is also labeled as port injection; source inspection plus actual queue hooks establish the composed ordering without claiming an external disk experiment.

---

## PHASE 2: Farley Engineering Rigor

| New production function | Physical lines inclusive | Limit | Status |
|---|---:|---:|---|
| shutdown.rs:83 ShutdownFailure::capture |13|25|Pass|
| shutdown.rs:98 shutdown |5|25|Pass|
| shutdown.rs:104 quiesce |7|25|Pass|
| shutdown.rs:112 quiesce_shard |15|25|Pass|
| shutdown.rs:128 drain_journal |10|25|Pass|
| shutdown.rs:139 release_timers |17|25|Pass|
| queue/writer/shutdown.rs:7 shutdown_durable |7|25|Pass|
| queue/writer/shutdown.rs:15 close_admission |8|25|Pass|
| queue/writer/shutdown.rs:26 drain_durable |18|25|Pass|

New getters are3lines; changed shard quiesce helper remains below25. New functions have at most4parameters. Snapshot/data logic and persistence shell are separated through explicit capabilities; no new hidden asynchronous work.

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|---|---|
| Zero unsafe and forbidden panic/unwrap/expect/todo/dbg in new production | Pass scoped source inspection and strict source clippy |
| Checked counts/arithmetic | Pass; saturating totals and capacity-bounded drains |
| Explicit resource ownership | Pass; failed durable batch remains queued; timer release follows barrier |
| Fixed work bounds | Pass; shard count, entry queue length, capacity+1 batch iterations,64 snapshot details per category/shard |
| Pinned toolchain/features preserved | Pass; no toolchain/feature/dependency changes |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|---|---|
| Checked result is distinct from failure | Pass: Result<ShutdownReport,ShutdownFailure> and ShutdownPhase |
| Illegal success construction hidden | Pass: report fields private, construction only after drain success |
| Remaining work is explicit | Pass: pending boundaries and command depth reported; no claim every run finished |
| No forced cancellation or fabricated task model | Pass |
| Error provenance retained | Pass: original RuntimeError is boxed Error::source; phase and completed prefix retained |

Success remaining() intentionally captures boundaries before timer-registration cleanup; docs state that timing. Repeated shutdown reports current retained runs, zero released timers and zero new drain. Existing tick errors can consume a failing command before reporting error; this delta reports actual remaining state, and does not claim rollback of existing tick semantics. Lower-level legacy Shard shutdown helpers retain their existing compatibility behavior; canonical checked Runtime shutdown uses quiesce directly.

---

## PHASE 5: The Bitter Truth

Fresh executions passed57runtime shutdown-filter tests,54queue tests,537IPC tests,4CLI service tests and11migrated integration shutdown cases:663candidate tests, zero failed/ignored. The57 runtime subset includes9 new production-bound tests and legacy smoke/model tests; passing copied Loom models is not presented as production verification. One additional independent partial-progress/retry/reopen probe passed. Scoped strict source clippy over four affected crates and formatting over all20changed Rust files passed.

The actual inherited IPC production wiring already drained its shared queue before a successful Ack; the earlier disconnected helper observation was not a production-loss demonstration. This shutdown change instead adds real timer-authority failure retention, typed reports/provenance and per-batch durable ownership closure. No false red/green data-loss claim is repeated.

References read and applied: jj/SKILL.md; beads/SKILL.md; black-hat-reviewer/SKILL.md and response-template.md; async-rust-reviewer/SKILL.md and references/structural-correctness.md; holzman-rust/SKILL.md and references/nasa-jpl-standards.md and references/runtime-performance-architecture.md; authoritative master clauses and live vb-aaqp7 contract/candidate source. Previous IPC review evidence informs the explicitly excluded baseline findings; fresh results are identified separately.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---|---|---|---|
| No new actionable blocker found in assigned shutdown delta | — | — | Scoped approval |
| Inherited IPC function-size findings | P2 | Parent862543ed; vb-t3cp3 report | Remain open separately in vb-2a9i3; not waived |

---

## Quality Gates

| Gate | Result | Evidence |
|---|---|---|
| Runtime shutdown filter |57pass,0ignored;1866filtered|raw/runtime-shutdown.*|
| Storage queue filter |54pass,0ignored;1547filtered|raw/storage-queue.*|
| IPC library |537pass,0ignored,0filtered|raw/ipc-library.*|
| Actual CLI service tests |4pass,0ignored;352filtered|raw/cli-shared-service.*|
| Four changed integration suites, shutdown filter |11pass,0ignored|raw/shutdown-migrated-workspace-tests.*|
| Independent partial-progress/retry/reopen probe |1pass|raw/partial-probe.*; partial_durable_shutdown_probe.rs|
| Source-only strict clippy,4crates,lib/bins/allfeatures |exit0|raw/shutdown-source-lint.*; full argv in commands.jsonl|
| Exact20changed Rust files formatting |exit0|raw/shutdown-format-check.*|
| Production source/test restoration after temporary probe |empty gitdiff|production-source-diff.txt|
| Broad moon CI/formal proof/performance |Not run/not approved|Parent integration and separate gates|

Full argv, cwd, source pin, true exit and elapsed time are recorded in commands.jsonl; complete raw logs are bound by raw-sha256.jsonl. Temporary probe is archived only under this review bead, with crate files restored exactly to the candidate.

---

## Verdict

**STATUS: APPROVED**

### Summary

Exact6a5a4d4e passes the independent scoped shutdown CODE review. Actual failures retain timer authority and queue ownership; success reports remaining work and follows durable drain, and the later-batch retry probe passes. This does not close independent TEST review, inherited IPC repair, canonical CI, physical crash testing or full v1 acceptance.

---

## Required Repair Actions (if REJECTED)

None for the reviewed shutdown delta. Parent retains existing separately tracked IPC structural and integration gates.
