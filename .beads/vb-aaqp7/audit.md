# vb-aaqp7 production shutdown candidate

Base: immutable IPC candidate 862543ed44c354846235f6676675ed86b3f19add. Workspace: /home/lewis/src/isoloated/velvet-ballistics-vb-aaqp7-shutdown-sol; Git root and JJ root match, backing is this worktree Git admin. No main-checkout implementation or broad moon ci was run. Parent owns canonical moon ci and independent code/test reviews.

## Actual defect and repair

`production-timer-authority-red-confirmed` executed one actual Runtime + queued journal + Fjall regression against the inherited shutdown implementation. A conflicting stored event makes the journal drain fail. Runtime remained active_run_count=1 but pending_timer_count became0 (expected1). The earlier red-labelled attempts were fixture/compile failures, not behavioral evidence. `production-timer-authority-green` executes the same ownership assertion after repair.

Canonical Runtime::shutdown_graceful now returns Result<ShutdownReport,ShutdownFailure>. New runtime/shutdown.rs separates bounded shard quiescence, journal durability and timer cleanup. Failure keeps timer registrations, original RuntimeError as Error::source, actual current runtime boundary snapshot, failing phase/shard index and completed-shard prefix. Success reports processed command count, preserved run/timer/action/ask snapshot, timer registrations released and journal flush report. Success does not certify active runs finished. Snapshot includes exact totals and at most64 details per category/shard, with explicit truncation. Suspended frames/action tickets remain owned, no cancellation is synthesized. Commands after an explicit queued Shutdown sentinel remain visible in the report.

QueuedStorageRuntimeJournal now uses JournalWriterQueue::shutdown_durable: closes admission after shard quiescence, commits each configured batch with SyncAll before removing it, and retains a failed batch for retry. A final persist_strict also covers prior profile-driven flushes. Already successful batches may have retired durably before a later error. Existing generic profile-driven drain APIs retain compatibility. Narrow legacy caller assertions explicitly project checked success to unit; dedicated new tests inspect typed report fields. New production shutdown functions are all <=25 physical lines after rustfmt.

## Executed evidence

Complete argv/cwd/exit/duration/stdout/stderr/hashes are in commands.jsonl and raw/. CARGO_BUILD_JOBS=2 throughout.

- runtime-shutdown-compatibility:57 passed,0ignored,1866filtered; includes9 new actual runtime/storage tests: failure retains timer/deadline, success reports suspended run, actual action ticket/frame, completed runs/full command queue, full128-event journal queue, explicit Shutdown sentinel remaining command,65-run bounded details, journal-port retry, multishard error provenance.
- durable-shutdown-writer-suite:5 passed,0ignored; two new shutdown tests exercise actual Fjall queue commit/barrier hooks, closed admission, retention and idempotent retry.
- shutdown-storage-queue-suite: full scoped queue result recorded in ledger.
- shutdown-ipc-suite:537 passed,0ignored,0filtered.
- production-cli-owned-queue-suite:4 passed,0ignored,352filtered. The real RuntimeJournalConfig::Journaled shares the service queue: all65 events are stored before correlation501 ShuttingDown Ack. Divergence at65th event returns correlation502 RuntimeError, retains1 pending event, and returns typed Storage(DuplicateEvent) service error, never ShuttingDown.
- shutdown-migrated-workspace-tests:11 passed across4 touched suites,0ignored; filtered only shutdown behaviors. Other tests in those suites were not run here.
- shutdown-source-lint: canonical source-only --lib --bins --all-features clippy bans on vb_runtime/vb_storage/vb_ipc/velvet-ballistics passed.
- shutdown-format-check: all20 changed Rust files passed. git diff --check reports raw Cargo output terminal blank lines; those immutable raw bytes are intentionally retained; source-only diff whitespace check is separate.

## Evidence limits and corrected claims

The journal-port failure-once wrapper delegates real append/storage behavior but injects StrictDurabilityFailed at the runtime port; it is not a physical disk-failure experiment. Actual storage commit and idempotent-barrier hooks separately establish queue ownership ordering. DuplicateEvent tests exercise real Fjall staging errors. No disconnected Loom/Kani model, list/version probe, or fake proof closure is claimed. No speed claim or reactor/durable throughput baseline is claimed (vb-n9wil remains open).

A disconnected VolatileRuntimeJournal CLI helper composition with65 service-only queued events does not establish production data loss. Actual cmd_ipc_serve wires Runtime and service to the same queued journal; inherited successful Runtime shutdown already drained all batches before Ack. Independent reviewers confirmed this exact862 production wiring. No after-Ack drain workaround was added. The new service failure test initially expected continued service, but production returns a typed Storage error after the wire RuntimeError; that invalid test assumption is retained in production-service-failed-shutdown-red and corrected by production-cli-owned-queue-suite. It is not a production red/green repair claim.

The inherited IPC audit cited installed Fjall3.1.10 incorrectly. Actual dependency is pinned3.1.4 under /cache/cargo-shared/registry/src/index.crates.io-1949cf8c6b5b557f/fjall-3.1.4. Its batch/mod.rs:102-106 has the same empty-batch early return; the prior repair/regressions remain valid. The inherited audit and vb-fnugl tracker comment now state the correct attribution.

Existing shard tick error semantics can consume the failing command before returning its error; shutdown reports that original error and actual remaining queues rather than claiming the command remains queued. This lane does not rewrite admission/recovery/tick ownership. Independent review is still required; no bead is self-closed. Separate IPC structural review fixes against862 remain coordinated by the parent.
