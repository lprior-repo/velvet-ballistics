# Proof Evidence: vb-core-ipc-sync-evidence

updated_at: 2026-05-17T00:00:00Z
state: 8
attempt: loom-fix-1
proof-writer: v1.0.1

## Workspace Guard

- Command: `pwd -P && rtk git status --short`
- Exit: non-zero because isolated workspace is not a Git repository.
- Output: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`; `fatal: not a git repository`.
- Classification: PASS for path isolation, NOT_APPLICABLE for Git status in jj isolated workspace.

## Artifacts Written Or Repaired

- `.beads/vb-core-ipc-sync-evidence/proof-writer-report.md`: refreshed State 5 attempt 3 repair delta, command evidence, classifications, and upstream/downstream route.
- `.beads/vb-core-ipc-sync-evidence/proof-evidence.md`: refreshed State 5 attempt 3 raw evidence summary.
- `.beads/vb-core-ipc-sync-evidence/STATE.md`: appended State 5 attempt 3 transition and completion evidence.
- No production source, tests, dependencies, CI, State 3/4 contract artifacts, TLA+ source, or Verus source were edited.

## Guard And JSONL Validation

- Command: `mkdir -p target/tmp && date -u +%Y-%m-%dT%H:%M:%SZ && TMPDIR=target/tmp test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence" && TMPDIR=target/tmp jq -c . .beads/vb-core-ipc-sync-evidence/proof-obligations.jsonl >/dev/null && TMPDIR=target/tmp jq -c . .beads/vb-core-ipc-sync-evidence/proof-obligations.planned.jsonl >/dev/null && TMPDIR=target/tmp jq -c . .beads/vb-core-ipc-sync-evidence/traceability-matrix.jsonl >/dev/null`
- Exit: 0
- Output: `2026-05-15T22:39:09Z`

## Canonical Status Discovery

- Command: `TMPDIR=target/tmp jq -r '.status' .beads/vb-core-ipc-sync-evidence/proof-obligations.jsonl | sort | uniq -c && TMPDIR=target/tmp jq -r 'select(.status != "planned") | .id + " " + .status + " owner_state=" + (.owner_state|tostring)' .beads/vb-core-ipc-sync-evidence/proof-obligations.jsonl`
- Exit: 0
- Output: `14 blocked`, `15 planned`, `1 waived`.
- Non-planned rows: `REFINE-IPC-001`, `REFINE-IPC-002`, `LOOM-IPC-002`, `REFINE-IPC-003`, `LOOM-IPC-003`, `REFINE-IPC-004`, `LOOM-IPC-004`, `REFINE-IPC-005`, `LOOM-IPC-005`, `PROP-IPC-006`, `SCAN-IPC-007`, `SCAN-IPC-008`, `WAIVE-VERUS-008`, `BLOCK-TLA-LIVENESS`, `GATE-IPC-009`.
- Classification: BLOCK_LOCAL upstream route to State 3/4 because `contract-verification-review.md` requires canonical rows to be `planned` only or blocker metadata moved out of canonical obligations.

## TLA+ Capacity 2

- Initial command: `TMPDIR=target/tmp tlc -metadir target/tmp/tlc-cap2-2239 -config verification/tla/IpcSyncEvidence.cfg verification/tla/IpcSyncEvidence.tla`
- Initial exit: non-zero.
- Initial result: BLOCK_LOCAL_ENV. Plain TLC attempted to copy standard modules under `/tmp` and failed with `java.io.IOException: Disk quota exceeded`.
- Repair command: `TMPDIR=target/tmp JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence/target/tmp" tlc -metadir target/tmp/tlc-cap2-2240 -config verification/tla/IpcSyncEvidence.cfg verification/tla/IpcSyncEvidence.tla`
- Repair exit: 0
- Result: PASS for bounded safety/enabledness only.
- Key output: `Model checking completed. No error has been found.`; `28060 states generated, 5136 distinct states found, 0 states left on queue.`; depth `17`.

## TLA+ Capacity 1

- Initial command: `TMPDIR=target/tmp tlc -config verification/tla/IpcSyncEvidenceCap1.cfg verification/tla/IpcSyncEvidence.tla`
- Initial exit: non-zero.
- Initial result: BLOCK_LOCAL_ENV. Parallel TLC invocation collided with generated `states/26-05-15-17-39-26`; local metadir rerun used below.
- Repair command: `TMPDIR=target/tmp JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence/target/tmp" tlc -metadir target/tmp/tlc-cap1-2240 -config verification/tla/IpcSyncEvidenceCap1.cfg verification/tla/IpcSyncEvidence.tla`
- Repair exit: 0
- Result: PASS for bounded safety/enabledness only.
- Key output: `Model checking completed. No error has been found.`; `15781 states generated, 2997 distinct states found, 0 states left on queue.`; depth `15`.

## Verus Strict Admission

- Command: `TMPDIR=target/tmp verus verification/verus/ipc_strict_admission.rs`
- Exit: 0
- Result: PASS_PURE_ONLY.
- Output: `verification results:: 5 verified, 0 errors`.

## Verus Capacity Bounds

- Command: `TMPDIR=target/tmp verus verification/verus/ipc_capacity_bounds.rs`
- Exit: 0
- Result: PASS_PURE_ONLY.
- Output: `verification results:: 6 verified, 0 errors`.

## Verus Runtime Transitions

- Command: `TMPDIR=target/tmp verus verification/verus/ipc_runtime_transitions.rs`
- Exit: 0
- Result: PASS_PURE_ONLY.
- Output: `verification results:: 7 verified, 0 errors`.

## Loom Bounded Queue

- Command: `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue`
- Exit: 0
- Result: PASS
- Output: `cargo test: 2 passed, 1467 filtered out (9 suites, 0.01s)`

## Loom Action Completion Cancel

- Command: `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime action_completion_cancel`
- Exit: 0
- Result: PASS
- Output: `cargo test: 2 passed, 1467 filtered out (9 suites, 0.01s)`

## Loom Timer Fired Cancel

- Command: `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime timer_fired_cancel`
- Exit: 0
- Result: PASS
- Output: `cargo test: 1 passed, 1468 filtered out (9 suites, 0.00s)`

## Loom Shutdown Drain

- Command: `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime shutdown_drain`
- Exit: 0
- Result: PASS
- Output: `cargo test: 3 passed, 1466 filtered out (9 suites, 0.01s)`

## Slow Client Filter

- Command: `TMPDIR=target/tmp rtk cargo test -p vb_ipc slow_client`
- Exit: 0
- Result: BLOCK_LOCAL, vacuous pass.
- Output: `cargo test: 0 passed, 407 filtered out (1 suite, 0.00s)`.

## Fanout And Buffer Static Scan Count

- Command: `TMPDIR=target/tmp rtk rg -n "tokio::spawn|std::thread::spawn|spawn\(|unbounded|Vec::new|Vec<|channel::unbounded" crates/vb_ipc/src crates/vb_runtime/src crates/vb_core/src crates/vb_storage/src | wc -l`
- Exit: 0
- Result: BLOCK_LOCAL until State 10 classification.
- Output: `465`.

## YAML/JSON/HTTP Static Scan Count

- Command: `TMPDIR=target/tmp rtk rg -n "serde_json|serde_yaml|yaml|json|http|hyper|reqwest|axum|tonic" Cargo.toml crates/vb_ipc crates/vb_runtime crates/vb_core crates/vb_storage | wc -l`
- Exit: 0
- Result: BLOCK_LOCAL until State 10 classification.
- Output: `46`.

## Flux Discovery

- Command: `TMPDIR=target/tmp cargo flux --version`
- Exit: 101
- Result: BLOCK_LOCAL_TOOLING for non-required `BLOCK-FLUX-001` row.
- Output: `error: no such command: flux`.

## Assumptions And Bounds

- TLA+ uses finite `RUNS = {r1, r2}` and `CLIENTS = {c1, c2}`.
- TLA+ checked queue and client buffer capacities `1` and `2`.
- TLA+ configs use `INVARIANT` declarations and `CHECK_DEADLOCK FALSE`; no temporal `PROPERTY`, fairness, or deadlock-freedom pass is claimed.
- Verus proofs are pure witnesses only. They do not import or verify production admission, queue, timer, runtime, storage, journal, channel, socket, or wall-clock APIs.
- Loom blockers require source/model repair outside State 5.
- Static-scan commands found matches but did not perform exhaustive per-match classification in this State 5 proof-writer pass.

## Remaining Blockers And Routes

- `REFINE-IPC-001..005`: BLOCK_LOCAL. Route to State 3/5 for reviewed refinement maps/adapters and State 8 for production-linked source/adapter work.
- `LOOM-IPC-002..005`: PASS. Arc import fixed in loom model files; all 4 lanes pass.
- `PROP-IPC-006`: BLOCK_LOCAL. Route to State 8 to add or identify executable slow-client oracle.
- `SCAN-IPC-007` and `SCAN-IPC-008`: BLOCK_LOCAL. Route to State 10 for exhaustive classification and State 8 for defects.
- `BLOCK-TLA-LIVENESS`: BLOCK_LOCAL. Route to State 3/5 to either add real TLA+ temporal/fairness/deadlock proof or keep liveness outside required approval scope.
- Canonical `proof-obligations.jsonl` status shape: BLOCK_LOCAL. Route to State 3/4; do not make State 5 rewrite canonical contract/planning artifacts.
- `GATE-IPC-009`: DEFERRED_GLOBAL to State 11; no `moon ci` pass claimed.
- `BLOCK-FLUX-001`: BLOCK_LOCAL_TOOLING but non-required.

## State 5 Attempt 4 Restructuring Evidence (2026-05-16)

### Contract-Verification Reviewer Rejection Repair

- State 6 contract-verification-reviewer rejected with LETHAL finding: canonical `proof-obligations.jsonl` must only have `planned` status rows.
- Original status discovery: 15 planned, 14 blocked, 1 waived.
- Repair action: restructured canonical obligations into two files:
  1. `proof-obligations.jsonl` with only `planned` rows (15 rows)
  2. `proof-obligations.blocked.jsonl` as separate blocker register (14 blocked + 1 waived)

### Restructuring Commands

- Extract planned rows: `jq -c 'select(.status == "planned")' .beads/vb-core-ipc-sync-evidence/proof-obligations.jsonl`
- Create blocker register by ID extraction from `proof-obligations.planned.jsonl`
- Normalize blocker register statuses: `blocked_tooling` -> `blocked`, preserve `waived`

### Restructuring Validation

- `jq -c . .beads/vb-core-ipc-sync-evidence/proof-obligations.jsonl >/dev/null`; exit 0
- `jq -c . .beads/vb-core-ipc-sync-evidence/proof-obligations.blocked.jsonl >/dev/null`; exit 0
- `proof-obligations.jsonl`: 15 planned rows
- `proof-obligations.blocked.jsonl`: 14 blocked + 1 waived rows

### Downstream Blockers Preserved

| Obligation | Status | Owner State | Blocker Reason |
|------------|--------|-------------|---------------|
| REFINE-IPC-001..005 | blocked | State 5/8 | Pure Verus detached from production APIs |
| LOOM-IPC-002..005 | PASS | State 8 | Arc import fixed; all 4 lanes pass |
| PROP-IPC-006 | blocked | State 8 | No slow-client test oracle (production source write required) |
| SCAN-IPC-007/008 | blocked | State 10 | Exhaustive classification pending |
| BLOCK-TLA-LIVENESS | blocked | State 5 | No executable temporal liveness |
| GATE-IPC-009 | blocked | State 11 | moon ci gate downstream |
| WAIVE-VERUS-008 | waived | State 3 | Verus inapplicable to source scan |
