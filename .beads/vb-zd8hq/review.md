# Independent test review: vb-aaqp7 shutdown candidate

- **Verdict:** REJECT pending one P2 assertion repair
- **Reviewed candidate:** `6a5a4d4e9861e6dc989362b496de1b6d47b4b939`
- **Remote bookmark:** `codex/vb-aaqp7-shutdown-candidate` (verified at the reviewed SHA)
- **Review workspace:** `/home/lewis/src/isoloated/velvet-ballistics-vb-aaqp7-shutdown-test-review-root-20261021`
- **Source changes:** none.

## Finding

**P2 — Assert the exact error in the actual Runtime + Fjall timer-preservation failure.** In `crates/vb_runtime/src/runtime/shutdown_tests.rs:20-41`, the test deliberately creates a durable event conflict to make the real queued Fjall drain fail, then only asserts `std::error::Error::source(&failure).is_some()`. A wrong underlying `RuntimeError` or storage error would satisfy that assertion while the test still passes. The neighboring failure-once port test checks its exact injected `StrictDurabilityFailed`, but that does not establish the exact source propagated from this actual Fjall `DuplicateEvent` path.

Repair by asserting `failure.cause()` equals the expected storage-backed `RuntimeError` for the conflicting `run` and sequence, while retaining the existing phase, completed-shard, timer, and queued-work assertions. No implementation change is requested by this test-review finding.

## Independent focused reruns

All commands ran from this isolated workspace against the exact candidate, with `CARGO_BUILD_JOBS=2` and test threads capped at 2:

1. `cargo test -p vb_runtime --lib shutdown -- --test-threads=2` — **57 passed, 0 failed, 0 ignored**.
2. `cargo test -p vb_storage --lib queue:: -- --test-threads=2` — **54 passed, 0 failed, 0 ignored**. Covers full capacity, multi-batch drain, final partial batch, retry ownership, and idempotent already-visible entry barrier failure.
3. `cargo test -p velvet-ballistics --bin velvet-ballistics ipc_serve::tests -- --test-threads=2` — **4 passed, 0 failed, 0 ignored**. Includes shared 65-record queue before correlated shutdown acknowledgement and duplicate-event failure returning wire RuntimeError plus typed service Storage error without a ShuttingDown response.
4. `cargo test -p vb_ipc --lib -- --test-threads=2` — **537 passed, 0 failed, 0 ignored**.
5. Migrated workspace shutdown filters across four integration targets — **11 passed, 0 failed, 0 ignored**.

Raw stdout is preserved under `evidence/`.

## Review scope

The timer ownership, durable queue retry, idempotent barrier, full/partial batch, shared CLI queue, and IPC suites all passed. The one finding concerns assertion strength for the original error on the actual storage failure path. No physical disk failure or broad CI run is claimed. Parent bead `vb-aaqp7` remains open.
