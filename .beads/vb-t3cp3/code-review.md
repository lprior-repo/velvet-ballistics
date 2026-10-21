**Bead**: vb-t3cp3 (discovered-from vb-cyz6d)  
**State**: independent CODE review  
**Reviewer**: black-hat-reviewer with async-rust-reviewer and Holzmann lenses  
**Source checkout**: /home/lewis/src/isoloated/velvet-ballistics-vb-cyz6d-code-review-sol-20261021  
**Attempt**: 1

## Gate Result

**STATUS: REJECTED** — P2 function-size compliance only. No additional functional defect established in the assigned repair scope.

Exact source: `862543ed44c354846235f6676675ed86b3f19add`, bookmark `codex/vb-cyz6d-async-audit-candidate`, compared against `856603b8`. The evolving vb-aaqp7 shutdown child is excluded. Review source is immutable; temporary probes were removed from crate files before exact-source final checks.

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|---|---|---|
| Synchronous core and mio shell | Pass | Master62 forbids async fn/executors here. No Tokio retrofit is appropriate. Ownership/fairness/cancellation/durability lenses applied to the actual reactor. |
| Bounded framing and readiness retention | Pass in scoped inspection/tests | connection_io.rs:96-140 consumes only magic/header/declared payload, at most64 operations/turn, clears readiness at WouldBlock, retains it at budget exhaustion; reactor.rs:30-57 uses zero timeout for retained work. Large-frame and fragmented-input tests execute actual source. |
| Admission and hostile peer isolation | Pass | reactor.rs:70-95 accepts up to256 per turn, drops/counts client257; service_clients removes peer failures while preserving infrastructure failures. Actual saturation, invalid magic, disconnect, healthy-peer tests pass. |
| Single-flight output ownership | Pass | Pending buffer guard, partial write drain and read suppression are explicit. Independent real-WouldBlock probe preserves correlation101 until drained, then delivers101/102 in order as Healthy. |
| Atomic durable queue service | Pass in bounded source/tests | flush.rs:116-134 selects SyncAll, explicitly persists an empty all-idempotent batch, and commits before deque removal. Real production queue tests cover FIFO, bounded batch, injected commit/barrier failure, retention and retry. Hooks are not physical disk-failure experiments. |
| Service flush during operation | Pass | IpcJournalService owns shared journal/queue and calls flush_batch_durable before poll and after tick. Staging errors preserve typed causes and pending item. |
| Shutdown Ack delivery | Pass in scoped socket tests | Reactor stops dispatch/admission after ShuttingDown and polls until buffered output completes; actual full outgoing-socket test asserts correlation88 and ShuttingDown. No bounded wall-clock drain guarantee claimed. |
| Actual shared queue drained before successful shutdown Ack | Pass; reported P1 not demonstrated in production wiring | Independent65-record probe uses same Arc journal/queue in service and RuntimeJournalConfig::Journaled.shared_journal. Shutdown drains all65 before correlation777ShuttingDown, queue0, servicefalse; all65 survive reopening. See applicability discussion below. |
| Full runtime shutdown closure | Excluded | vb-aaqp7 separately owns production drain outcomes/model binding. Queue tests do not close that bead. |
| Performance acceptance | Excluded | No speed/p99/durable-throughput claim; vb-n9wil remains separate. |

### Shutdown wiring counterexample applicability

A disconnected `new_for_tests_and_benchmarks_only` Runtime reproduces the weaker helper composition:65 queued service records, pre-poll flush64, immediate Shutdown Ack, returnfalse with1 remaining. This does **not** establish cmd_ipc_serve production data loss. Production ipc_serve.rs:26-29 builds Runtime with clones of the exact same journal and queue. Successful Shutdown dispatch calls Runtime::shutdown_graceful (runtime.rs:1135), then QueuedStorageRuntimeJournal::drain_for_shutdown (journal/chunk_003.rs:50-62), which drains the remaining queue and calls persist_strict before ShuttingDown is returned. The independent shared-wiring control verifies this path, including reopen. The service helper's ownership coupling is implicit and can be strengthened, but a disconnected-fixture failure must not be presented as a proven production bug.

---

## PHASE 2: Farley Engineering Rigor

| Function | Physical lines inclusive | Nonblank/noncomment excluding brace-only lines | Limit | Status |
|---|---:|---:|---:|---|
| connection_io.rs:22 handle_readable |47|42|25|Fail|
| connection_io.rs:96 read_request |29|24|25|Physical-line rule fail; no logical-line failure claimed|
| reactor.rs:132 service_clients |30|25|25|Physical-line rule fail; no logical-line failure claimed|
| queue/writer/flush.rs:66 flush_batch_with_durability |87|47|25|Fail|
| connection_io.rs:70 handle_writable |24|20|25|Pass|
| connection_io.rs:142 queue_response |20|18|25|Pass|
| reactor.rs:109 register_client |22|21|25|Pass|
| ipc_serve.rs:94 IpcJournalService::turn |21|19|25|Pass|

The first and fourth rows exceed25 even after blanks/comments/brace-only lines are excluded. This review does not claim the repository hot-file glob itself covers IPC/storage, nor that a canonical source-length command was executed. Pre-existing large helpers such as send_response and cmd_ipc_serve are not treated as newly introduced whole-repository findings. Extraction must preserve readable execution transitions and commit-before-drain ordering; no lint suppressions or contract weakening.

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|---|---|
| Zero unsafe in modified production | Pass scoped source inspection and strict source clippy |
| Zero unwrap/expect/panic/todo/dbg in modified production | Pass strict source clippy |
| Checked arithmetic/bounded I/O | Pass scoped inspection and clippy |
| Fallible allocation on newly grown request/output buffers | Pass; existing postcard allocator behavior remains |
| Explicit ownership and typed failure | Pass in scoped source; ResponsePending prevents replacement; peer failures distinguish reactor failures |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|---|---|
| Explicit read outcome and close policy | Pass: ReadProgress and ClosePolicy enums |
| Output ownership | Pass: nonempty response buffer is never overwritten |
| Queue state at failed commit/barrier | Pass: retained until retry; no drain before success |
| No async dependency/runtime mismatch | Pass: master-authorized synchronous mio implementation |
| Focused decomposition | Fail: two substantial production functions still combine several transitions |

The async skill's Option state-machine prohibition is applied to protocol states, not to legitimate optional resolver/diagnostic capabilities. No future cancellation/task supervision evidence is invented for synchronous code.

---

## PHASE 5: The Bitter Truth

Fresh exact-source executions passed537IPC library tests,52storage queue tests,3CLI service tests and7IPC acceptance tests:599 distinct candidate tests, zero failed/ignored. Three additional reviewer probes passed: shared-wiring65records/reopen, disconnected-wiring control, and two correlations under real socket backpressure. The preliminary service run included the two reviewer wiring probes (5tests); the later exact-source3test service run is the clean candidate result. Repeated subsets are not added to the599 total.

No full moon CI, verifier closure, physical storage failure, full production shutdown, or performance acceptance is inferred from these results. The independent TEST verdict belongs to its reviewer.

References actually read: jj/SKILL.md; beads/SKILL.md; black-hat-reviewer/SKILL.md and response-template.md; async-rust-reviewer/SKILL.md and references/structural-correctness.md; holzman-rust/SKILL.md and references/nasa-jpl-standards.md and references/runtime-performance-architecture.md. Master and candidate contract/audit were read; author evidence was used for navigation, not substituted for fresh results.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---|---|---|---|
| IPC-REVIEW-001: production functions exceed the explicit25line review limit | P2 / MEDIUM | connection_io.rs:22; queue/writer/flush.rs:66 | Open |

### IPC-REVIEW-001: decompose touched protocol and durable-commit functions

**Location**: `crates/vb_ipc/src/server/connection_io.rs:22`, `crates/vb_storage/src/queue/writer/flush.rs:66`.

**Problem**: handle_readable contains registry acquisition, client/output eligibility, frame/result classification, payload extraction/dispatch, response queueing and shutdown state mutation. flush_batch_with_durability combines queue locking/selection, batch staging, profile selection, injected-failure handling, all-idempotent persistence, commit and ownership release. They are47/87physical lines and42/47under the disclosed reduced count. This violates the explicitly requested black-hat25line rule. Adjacent read_request/service_clients also exceed25physical lines as disclosed, without a logical-length failure claim.

**Evidence**: pinned source locations and counts above; candidate formatting/clippy pass does not enforce this review rule.

**Required Fix**: extract named bounded transition/staging/commit/release helpers with at most25lines, preserving exact errors, readiness retention, single-flight output, SyncAll before queue ownership release and all tests. Publish a separate source candidate for independent re-review. No unrelated architecture rewrite.

---

## Quality Gates

| Gate | Result | Evidence |
|---|---|---|
| Exact candidate cargo test -p vb_ipc --lib |537pass,0fail,0ignored|raw/exact-ipc-lib.*|
| Exact candidate storage queue filter |52pass,0fail,0ignored;1547filtered|raw/storage-queue.*|
| Exact candidate CLI service filter |3pass,0fail,0ignored;352filtered|raw/exact-cli-service.*|
| Exact candidate binary IPC acceptance target |7pass,0fail,0ignored|raw/ipc-acceptance.*|
| Reviewer wiring probes |2pass|raw/wiring-probe.* and shutdown_wiring_probe.rs|
| Reviewer correlation/backpressure probe |1pass|raw/correlation-probe.* and reactor_correlation_probe.rs|
| Exact changed-file rustfmt check |exit0|raw/exact-file-format.*|
| Exact source clippy,3touched crates,lib/bins,allfeatures,strict warning/panic/index/cast/arithmetic/result/print denies |exit0|raw/exact-source-lint.*; full argv commands.jsonl|
| Production crate files restored to exact862543ed |empty gitdiff|production-source-diff.txt|
| Full moon CI / proof / performance / vb-aaqp7 |Not run/not approved|Outside assigned bounded review|

`commands.jsonl` preserves actual argv/cwd/returncodes; `raw-sha256.jsonl` binds all raw logs. Temporary test additions are archived only in this bead directory, not left in production/test crate files.

---

## Verdict

**STATUS: REJECTED**

### Summary

The functional repairs survived the assigned independent checks, including actual shared shutdown wiring and output backpressure. Approval is blocked by the explicit function-size rule in touched production functions. The65-record disconnected-runtime observation is not a demonstrated production data-loss finding.

---

## Required Repair Actions (if REJECTED)

1. P2: mechanically decompose the named functions without altering errors, ordering, bounds or assertions; publish a new pinned candidate and rerun scoped tests/lint/format for re-review.
2. Keep parent vb-cyz6d and repair beads open until their independent gates complete; vb-aaqp7 shutdown and vb-n9wil performance closure remain separate.
