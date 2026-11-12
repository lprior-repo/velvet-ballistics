# Codebase Map: vb-qi37.12

bead_id: vb-qi37.12
title: runtime/storage: Eliminate silent discard paths
state: 2
updated_at: 2026-05-15T19:48:05Z
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12
source_checkout_db_only: /home/lewis/src/velvet-ballistics/.beads/dolt

## Bead Reality

Command used exactly as requested from the isolated workspace:

```text
bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.12 --json
```

Observed bead status is `in_progress`. Acceptance requires no first-party runtime, storage, or compiler path to silently drop fallible outcomes. Ignored results must be forbidden or moved behind explicit typed discard APIs. Tests must inject journal, storage, action, and recovery failures and prove typed errors reach callers and the evidence chain.

## Relevant Architecture And Policy

- `docs/rust-governance.md:50-57` requires typed railway errors for parse, validation, compile, runtime, IPC, storage, and tooling failures, and explicitly forbids ignored `Result` or fallible return values.
- `docs/storage-journal.md:19-28` defines explicit durability names: `append_journaled` has no caller-visible fsync barrier; `append_strict` persists before returning.
- `docs/runtime-architecture.md:34-40` keeps runtime/core separate from storage and assigns the Fjall durability boundary to `vb_storage`.
- `docs/language-spec.md:1646-1916` states the event journal is the runtime source of truth and forbids performance shortcuts that bypass it.

## Primary Production Surfaces

- `crates/vb_storage/src/journal/core.rs`: owns `FjallJournal::open` and `Drop`. `Drop` currently calls `database.persist(SyncAll)` and discards any error (`let _ = e`), a high-risk silent durability failure surface because drop cannot return a typed error.
- `crates/vb_storage/src/journal/append.rs`: explicit append APIs. `append_strict` and `append_strict_batch` already propagate persist errors with `?`; this is the model expected elsewhere.
- `crates/vb_storage/src/journal/internal.rs`: low-level append/get helpers. Needed to verify every Fjall operation maps to `JournalError` without lossy conversion.
- `crates/vb_storage/src/batch.rs`: atomic cross-keyspace `JournalWriteBatch`; digest, key, encode, and commit paths must keep abort/error state typed. Grep showed explicit error propagation in put methods plus `let _ = item.key()` in tests/examples around iteration helpers, which must be classified as non-production or repaired.
- `crates/vb_storage/src/recovery/recover.rs`: recovery entry points propagate `events_for_run` errors and return `NoRecoveryData` on empty journals. `verify_digests` still has an explicit deferred gap for action ABI and policy digest verification.
- `crates/vb_storage/src/recovery/replay/summary.rs`: contains `.ok()` conversions around optional taint decoding and digest mismatch rejection in test/support areas. Needs classification because lossy decode-to-None can hide corruption if production-facing.
- `crates/vb_storage/src/events.rs`: contains `postcard::from_bytes(bytes).ok()` for event payload accessors. Needs classification as optional typed access or conversion to a typed diagnostic if corruption can reach recovery/runtime callers.
- `crates/vb_storage/src/process_lock.rs`: uses `let _ = file.set_len(0)`, `let _ = write!(...)`, `let _ = rewind/read_to_string`, and `.ok()` while acquiring/reading lock metadata. This is a production storage boundary and must be reviewed for typed error propagation or an explicit best-effort discard API.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`: submit, resume, action completion, and action failure paths. Journal append failures are propagated; resume rollback maps append failure into `ResumeError::journal_append_failed_with_source`.
- `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`: ask answer, timer, cancel, drive, and terminal routing. Journal append failures propagate with `?`; however `apply_drive_result` maps any engine error to terminal failure via `Err(_)`, dropping the concrete runtime/core cause from caller-visible diagnostics.
- `crates/vb_runtime/src/shard/transitions.rs`: finish, await action/timer, and fail paths append journal events before final cleanup. This is part of strict persistence-before-visible-state evidence.
- `crates/vb_runtime/src/error/conversions.rs`: maps `JournalError` into `RuntimeError::StorageJournalAppend` and maps `ResumeError` back into runtime errors. This is the cross-boundary typed diagnostic preservation surface.
- `crates/vb_compile/src/type_taint.rs`, `schema.rs`, `references.rs`, `strict_yaml.rs`: compiler validation accumulates and propagates `CompileError`/`CompileErrors`. Grep showed `if let Err(e)` accumulation, which appears intentional rather than silent discard, but this must remain in scope because the bead includes compiler paths.

## Existing Gate And Evidence Surfaces

- `crates/workspace_tests/src/quality/test_loop_inventory/*`: existing quality inventory machinery, but it targets test loop patterns, not ignored fallible results.
- `crates/workspace_tests/src/boundary_inventory/*`: validates boundary inventories and may be reusable as a pattern for a fallible-result inventory, but it is not a silent-discard gate today.
- `crates/vb_runtime/src/shard/lifecycle_tests/*`: action failure, ask answer, retry, cancel, finish, and evidence-chain tests are natural homes for failure injection at runtime boundaries.
- `crates/vb_storage/src/journal/tests.rs`, `queue/tests.rs`, `recovery/tests.rs`, `tests.rs`: natural homes for journal/storage failure injection and recovery corruption tests.

## Initial Silent-Discard Inventory Seeds

- `SD-001`: `crates/vb_storage/src/journal/core.rs:108-113`, `FjallJournal::drop`, discard of `database.persist(SyncAll)` error. Requires design decision: remove fallible work from `Drop`, add explicit `close/persist` API, or route through typed best-effort discard evidence.
- `SD-002`: `crates/vb_storage/src/process_lock.rs:57-103`, process lock metadata writes/reads silently ignored or converted with `.ok()`. Requires typed propagation unless deliberately best-effort and isolated behind a named discard API.
- `SD-003`: `crates/vb_storage/src/events.rs:299`, event payload decode converts failure to `None`. Requires classification as optional accessor or typed corruption diagnostic.
- `SD-004`: `crates/vb_storage/src/journal/replay.rs:17`, optional event decode path converts a fallible decode to `None`. Requires classification against recovery fail-closed expectations.
- `SD-005`: `crates/vb_storage/src/recovery/replay/summary.rs:431`, taint decode converts corruption to `None`. Requires fail-closed recovery review.
- `SD-006`: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:170`, engine errors are collapsed by `Err(_) => apply_terminal_failed`, dropping cause. Requires typed diagnostic preservation or explicit evidence that the cause is persisted elsewhere.

## Delivery Boundaries

- In scope: production runtime/storage/compiler fallible result handling, typed error propagation, mechanical gate design, and targeted failure-injection tests in later states.
- Out of scope for State 2: production edits, test edits, proof edits, benchmark claims, UI, generated Rust/codegen parity, and source checkout writes.
- Cross-bead dependency: `vb-qi37.12.2` owns journal/storage propagation but is blocked by source-checkout artifact contamination; this bead must not rely on that blocked child being complete.
- Cross-bead dependency: `vb-qi37.12.4` owns the mechanical ignored-result gate and remains in progress; this bead should coordinate scope without duplicating incompatible gate machinery.

## Suggested Next State Inputs

- Contract clauses should distinguish hard errors, optional accessors, destructor limitations, and deliberately best-effort cleanup.
- Proof/test planning should require failure injection for strict append, batch commit, process lock metadata, recovery decode corruption, action failure, resume rollback, and engine-error-to-terminal conversion.
- Implementation planning should prefer small local changes and typed APIs over logging-only fixes.
