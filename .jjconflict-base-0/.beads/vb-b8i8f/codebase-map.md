# Codebase Map: vb-b8i8f State 2 Explore

## Scope
- Bead: `vb-b8i8f` — fresh recovery for prior capped `vb-9l7l` cancel/kill lattice, State 9 test ledger, and storage encoding gap.
- Isolated worktree: `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-b8i8f`.
- Source checkout: `/home/lewis/src/velvet-ballistics` is control-plane only; not edited.
- Fresh base from `STATE.md`: branch `fresh/vb-b8i8f`, main base `46cf61591`.
- Explore-only output: no production Rust, tests, proofs, manifests, or configs intentionally edited.

## Input Artifacts Read
- `.beads/vb-b8i8f/STATE.md`: current state 1, fresh replacement routed to State 2 explore, prior blocked bead `vb-9l7l`.
- `.beads/vb-b8i8f/baseline-report.md`: isolated workspace and prior evidence path `/home/lewis/isolated/velvet-ballistics-main-review/vb-9l7l`.
- `.beads/vb-b8i8f/global-readiness-report.md`: known blocker is clean cancel/kill lattice plus `StorageJournalAppend RecordKindFamilyMismatch` and corrupt ledgers.
- `.beads/vb-b8i8f/delivery-scope.jsonl`: seed row replaced by scoped State 2 map.
- Prior context read only from `/home/lewis/isolated/velvet-ballistics-main-review/vb-9l7l/.beads/vb-9l7l/`: `codebase-map.md`, `delivery-scope.jsonl`, `implementation.md`, `test-writer-report.md`, and prior test file `crates/workspace_tests/tests/restate_cancel_kill_lattice_tests.rs`.

## Fresh Workspace Files and Symbols Mapped

### Runtime public API
- `crates/vb_runtime/src/runtime.rs`
  - `Runtime::cancel_run(&self, run: RunId) -> RuntimeResult<()>` enqueues `ShardCommand::Cancel { run, reason: None }` without a preflight live/terminal existence check.
  - No `Runtime::kill_run` public facade is present on fresh `main` by scoped search; prior implementation report says that was added only in capped work.
  - `Runtime::snapshot_run`, action/timer/ask public facades remain relevant for stale authority assertions after terminalization.

### Shard cancel/kill lifecycle
- `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`
  - `Shard::handle_cancel(run, reason)` removes `pending_timers`, appends `RuntimeJournalEvent::RunCancelled` only if `runs.contains_key(&run)`, removes live run state, releases frame, inserts `terminal_runs`, increments failed counter, emits `TraceEvent::RunCancelled`, discards journal sequence, and returns `Ok(())` for missing/already-terminal runs.
  - `Shard::handle_kill(run, _reason)` removes `pending_timers`, removes live run state, releases frame, inserts `terminal_runs`, increments failed counter, emits `TraceEvent::RunKilled`, appends `RuntimeJournalEvent::RunKilled`, discards journal sequence, and returns `Ok(())` for missing/already-terminal runs.
  - `Shard::take_run_state` returns `RuntimeError::RunNotFound` for absent live runs, but cancel/kill handlers do not use it on fresh `main`.
- `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`
  - `Shard::tick` routes `ShardCommand::Cancel` and `ShardCommand::Kill` to those handlers. Prior capped implementation changed tick draining semantics; fresh main should be treated as still one-command-per-tick until re-read by implementation.
- `crates/vb_runtime/src/shard/types.rs`
  - Relevant private state: `runs`, `runtime_states`, `terminal_runs`, `journal_sequences`, `pending_timers`; relevant command variants: `ShardCommand::Cancel`, `ShardCommand::Kill`; relevant timer type: `PendingTimer`.

### Runtime journal to storage mapping
- `crates/vb_runtime/src/journal/chunk_001.rs`
  - `RuntimeJournalEvent::RunCancelled { run, reason }` and `RuntimeJournalEvent::RunKilled { run }` are the runtime terminal events.
- `crates/vb_runtime/src/journal/chunk_002.rs`
  - `StorageRuntimeJournal::run_storage_event` maps `RunCancelled` to `vb_storage::JournalEvent::RunCancelled { run, seq, attempt: 1, reason }`.
  - `StorageRuntimeJournal::run_storage_event` maps `RunKilled` to `vb_storage::JournalEvent::RunKilled { run, seq, attempt: 1 }`.
  - `StorageRuntimeJournal::append_storage_event` forwards to `FjallJournal::append_strict` or `append_journaled`, so storage codec acceptance controls durable kill success.

### Storage encoding gap (`RecordKindFamilyMismatch { kind: 28 }`)
- `crates/vb_storage/src/records.rs`
  - `RecordKind::RunKilled = 28`; `RecordKind::RunCancelled = 21`; `RecordKind::RunFinished = 22`; `RecordKind::RunFailed = 23`; `RunAdmission..RunAnswered = 24..=27`.
  - `RecordKind::id()` returns `28` for `RunKilled`.
- `crates/vb_storage/src/events.rs`
  - `JournalEvent::RunKilled { run, seq, attempt }` exists.
  - `JournalEvent::record_kind()` returns `RecordKind::RunKilled` for `RunKilled`.
  - `JournalEvent::attempt()` includes `RunKilled`; `JournalEvent::is_valid()` rejects attempt zero.
- `crates/vb_storage/src/codec/validation.rs`
  - `is_known_record_kind(kind)` currently matches `1 | 2 | 3 | 10..=27 | 30 | 40 | 50`; **kind 28 is not known**.
  - `validate_kind_family(MAGIC_JOURNAL_EVENT, kind)` currently accepts only `10..=27`; **kind 28 is not in the journal family**.
  - This is the direct fresh-main explanation for prior failing evidence: `StorageJournalAppend { source: RecordKindFamilyMismatch { magic: 1447184965, kind: 28 } }` when appending `RunKilled`.
- `crates/vb_storage/src/codec/mod.rs` and `crates/vb_storage/src/codec/header.rs`
  - Both `encode_record` and `encode_record_header` call `validate_kind_family` before payload/header construction, so `RunKilled` cannot be encoded as a journal record until kind validation admits 28.
- `crates/vb_storage/src/journal/internal.rs`
  - `FjallJournal::append_unpersisted` encodes each `JournalEvent` with `event.record_kind()`. Therefore `JournalEvent::RunKilled` reaches `encode_record(..., RecordKind::RunKilled, ...)` and fails before insert.
- `crates/vb_storage/src/journal/replay.rs`
  - `events_for_run` uses `decode_record::<JournalEvent>` and replay sequence validation; after append succeeds, decode-side validation must also recognize kind 28 or kill records will be unreadable.

### Workspace test target and prior State 9 ledger risk
- Fresh main registered target: `crates/workspace_tests/Cargo.toml` has `[[test]] name = "cancel_kill_lattice_tests" path = "tests/cancel_kill_lattice_tests.rs"`.
- Fresh main does **not** contain `crates/workspace_tests/tests/restate_cancel_kill_lattice_tests.rs`; only `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` is present.
- Fresh target file `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` currently has cancel-heavy tests and ignored action-suspended tests; no public kill lattice because `Runtime::kill_run` is absent.
- Prior capped State 9 created `crates/workspace_tests/tests/restate_cancel_kill_lattice_tests.rs` with 16 tests and fixed sequence fixtures. Its latest target run in prior `test-writer-report.md` showed 13 passed / 3 failed, all tied to `RecordKindFamilyMismatch { kind: 28 }` for `RunKilled` storage append.
- Prior capped implementation report showed additional production deltas that should not be blindly reused: public `Runtime::kill_run`, cancel/kill preflight, stale authority checks, shard fail-closed handlers, and bounded tick queue-depth draining.

## Existing Related Tests / Proof Surfaces Located
- `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs`: current registered cancel lattice test target.
- `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`: direct runtime cancel acceptance and journal/trace evidence.
- `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs`: cancel lifecycle/journal/counter coverage.
- `crates/workspace_tests/tests/vb_test_runtime_queue_timer_behavior.rs`: timer cancellation/removal behavior.
- `crates/workspace_tests/tests/postcard_envelope_wire_tests.rs`: record-kind wire tests; note existing proptest range cited by grep is `10u16..=27u16`, which excludes `RunKilled = 28`.
- `crates/workspace_tests/tests/restate_decode_error_taxonomy_tests.rs`: decode taxonomy family-mismatch cases.
- `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs`: Kani lifecycle harness surface named in prior map; downstream proof planner should re-read before proof changes.

## Risks for Downstream States
- **persistence / codec:** `RunKilled = 28` exists in the enum and event mapping, but codec known-kind and journal-family ranges still stop at `27`. This is the highest-confidence storage gap.
- **public-api:** fresh main has `Runtime::cancel_run` only; no `Runtime::kill_run` facade. Public lattice tests either need a public API addition or an approved shard-level scope.
- **temporal / command-queue:** fresh main cancel/kill handlers return `Ok(())` for absent/already-terminal runs; acceptance from prior work expected typed not-found and no duplicate terminal event.
- **persistence / corrupt-ledger:** prior sequence-gap failures were repaired in tests, but storage-side `RunKilled` encode/decode validation still prevents kill records from persisting and can leave target tests with append failures. Any implementation must preserve contiguous per-run `EventSeq` replay semantics from `events_for_run`.
- **test-target collision:** controller mentions prior State 9 ledger; fresh repo has registered `cancel_kill_lattice_tests`, while prior red suite file was named `restate_cancel_kill_lattice_tests` and is not registered in fresh main.
- **private-observability:** `pending_timers`, `terminal_runs`, and journal sequence internals are private; public tests may need journal/trace/snapshot evidence or approved diagnostic API.
- **semantic collision:** prior implementation tightened missing/already-terminal cancel/kill to `RunNotFound`; fresh shard tests may currently assert no-op semantics and will need scoped updates if the contract changes.

## State 2 Validator / Command Evidence
```text
$ pwd -P && test -d ".beads/vb-b8i8f" && test -s ".beads/vb-b8i8f/STATE.md" && test -s ".beads/vb-b8i8f/baseline-report.md" && test -s ".beads/vb-b8i8f/global-readiness-report.md"
/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-b8i8f

$ go-skill-v9-validate --state 2
zsh:1: command not found: go-skill-v9-validate
```

## Recommended Next Downstream Scope
1. Contract/proof planning should state whether kill must be public runtime API or shard-only; fresh main currently lacks `Runtime::kill_run`.
2. Storage repair lane should admit `RecordKind::RunKilled` kind 28 in codec known-kind and `MAGIC_JOURNAL_EVENT` family validation, with encode and decode regression coverage.
3. Test lane should decide whether to resurrect prior `restate_cancel_kill_lattice_tests.rs` and Cargo registration or extend the existing registered `cancel_kill_lattice_tests.rs`; do not keep both without an explicit manifest plan.
4. Implementation lane must not reuse capped ledgers as proof; cite prior evidence only as context and rerun target commands in the fresh worktree after production/test changes.
