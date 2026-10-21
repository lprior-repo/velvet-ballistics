# Independent IPC review

- Candidate commit: `862543ed44c354846235f6676675ed86b3f19add`
- Candidate bookmark: `codex/vb-cyz6d-async-audit-candidate`
- Remote verification: `git ls-remote origin refs/heads/codex/vb-cyz6d-async-audit-candidate` returned `862543ed44c354846235f6676675ed86b3f19add` during review.
- Reviewer checkout: `/home/lewis/src/isoloated/velvet-ballistics-vb-cyz6d-ipc-review-root-20261021`
- Review was read-only for source; evidence only was added.

## Verdicts

- Code: REJECTED pending the mechanical function-size repair described below.
- Tests: APPROVED for the reviewed behavior and bounded test scope.

## Correction: production shutdown drains the shared queue

The initial P1 shutdown-loss finding is withdrawn. The 65-event example used a test `Runtime` disconnected from `IpcJournalService.queue`; it does not reproduce production wiring. `cmd_ipc_serve` passes the same journal and queue to `RuntimeJournalConfig::shared_journal` and `IpcJournalService` (`crates/velvet_ballistics/src/ipc_serve.rs:18-30, 56`). The production Shutdown handler calls `Runtime::shutdown_graceful` before returning `ShuttingDown` (`crates/vb_ipc/src/server/handlers/session.rs:17-20`). That drains pending shard commands and calls the runtime journal's `drain_for_shutdown` (`crates/vb_runtime/src/runtime.rs:1134-1141`); the storage adapter drains all queue batches and persists `SyncAll` before returning (`crates/vb_runtime/src/journal/chunk_003.rs:50-62`).

I independently reran `CARGO_BUILD_JOBS=2 cargo test -p vb_runtime --lib runtime_shutdown_graceful_drains_owned_queued_journal`; 1 passed, 0 failed, 0 ignored. A separate production-wiring probe reported by the coordinator also queued 65 events, sent Shutdown through the shared queue/runtime, observed the acknowledgement, and verified the queue was empty and all 65 events were readable after reopen. No production shutdown data-loss counterexample remains from the initial finding.

## Code finding

**[P2] Four changed production functions exceed the repository review's 25-line limit.** The state transitions are bounded and the scoped behavior review found no functional blocker, but the changed code exceeds the mechanical function-size gate. Split these routines without changing their behavior or durability/readiness invariants: `handle_readable` (`crates/vb_ipc/src/server/connection_io.rs:22`), `read_request` (`crates/vb_ipc/src/server/connection_io.rs:96`), `flush_batch_with_durability` (`crates/vb_storage/src/queue/writer/flush.rs:66`), and `send_response` (`crates/vb_ipc/src/server/helpers.rs:142`).

## Independent focused test runs

All commands ran in this checkout with `CARGO_BUILD_JOBS=2`, exited 0, and reported zero ignored tests:

| Command | Result |
|---|---:|
| `cargo test -p vb_ipc --lib` | 537 passed, 0 failed, 0 ignored |
| `cargo test -p vb_storage --lib queue::` | 52 passed, 0 failed, 0 ignored |
| `cargo test -p velvet-ballistics --bin velvet-ballistics ipc_serve::tests` | 3 passed, 0 failed, 0 ignored |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_te1i_binary_ipc_acceptance` | 7 passed, 0 failed, 0 ignored |
| `cargo test -p vb_runtime --lib runtime_shutdown_graceful_drains_owned_queued_journal` | 1 passed, 0 failed, 0 ignored |

Raw output for each command is stored next to this report.
