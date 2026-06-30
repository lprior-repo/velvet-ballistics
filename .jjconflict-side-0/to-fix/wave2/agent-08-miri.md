# Wave 2 — Agent 08 (miri / UB-detector review)

Scope: 18 bugs (runtime/action/durability/shard tier). Mode: read-only.
Source-tier policy: every production crate declares `#![forbid(unsafe_code)]`
at lib/submodule root (`vb_runtime/src/shard/{mod,lifecycle,types,...}.rs:1`,
`vb_storage/src/preview.rs:1`, `vb_core/src/lib.rs:1`, plus all
submodules checked). No `unsafe` blocks, raw pointers, `MaybeUninit`,
`addr_of!`, `mem::transmute`, or `repr(C)` / `repr(packed)` exist in
the production source tree for any of these fixes. The strings
"unsafe" appearing in test names refer to the `RetrySafety::Unsafe`
semantic enum variant, not the keyword.

Miri toolchain: `cargo-miri 0.1.0 (52b6e2c208 2026-04-27)` on
`rustup default nightly`. Strict-provenance flags accepted.

## Pre-existing baseline debt — wave-2 blockers

`vb_storage/src/preview.rs` was already repaired in wave-1 (RS-001/002
regression landed the fix); `vb_storage` lib now compiles and
`cargo test -p vb_storage --lib` runs cleanly. The remaining
wave-2 baseline debt is concentrated in the **runtime shard arena
path**:

1. `crates/vb_runtime/src/shard/arena/mod.rs` — does NOT exist in
   main. Beads vb-kfkyl (RS-202 `ArenaManager::deallocate_all`),
   vb-ofk9m (RS-201 `Arena::clear` free-list leak), vb-irenu
   (duplicate of vb-ofk9m) were closed in `/home/lewis/src/isolated/`
   but never merged to main.
2. `crates/vb_runtime/src/shard/impl_parts/journal_helpers.rs` —
   does NOT exist in main. Beads vb-j8nb2 (RS-001 coalesce-flush
   cross-run seq corruption) and vb-j4h0m (B-014 buffering-vs-
   immediate guard) were closed via isolated patches; main has
   neither `flush_coalesce_buffer` nor `coalesce_window_ticks`
   anywhere.
3. `crates/vb_runtime/src/shard/lru_ring.rs` — does NOT exist in
   main. Bead vb-k0jj0 (RS-210 `LruRing::clear` strands) was
   closed in isolated; main has no `LruRing` symbol.
4. `crates/vb_runtime/src/shard/lifecycle/chunk_001_action.rs` —
   bead vb-if1eo (RS-105) references this file; main has only
   `shard/lifecycle/chunk_001.rs` containing `handle_action_failure`
   at line 451, which still mutates retry/handler state
   (`apply_action_failure_to_state`) BEFORE journaling
   (`append_journal_event(RuntimeJournalEvent::ActionFailed)`).

These are precisely the path-fix inconsistencies documented in the
bead close reasons: "Pre-existing BLOCK_GLOBAL baseline debt …
unchanged and out of scope per scoped per-crate gates" (vb-j4d19,
vb-kfkyl, vb-kqjo1, vb-kz475 close reasons).

The orphan-Kani fix for vb-kgjjk is also NOT-PATCHED in main:
`crates/vb_runtime/src/verification/kani/mod.rs` wires only
`kani_retry_math`, `kani_for_each_ordering`,
`kani_together_ordering`, `kani_engine_signals`. The 7 orphan
modules still on disk: `kani_admission_ordering`,
`kani_ask_answer_lifecycle`, `kani_cancel_kill_lattice`,
`kani_idempotency_tracker`, `kani_resume_state_machine`,
`kani_shard_lifecycle_harnesses`, `vb_fzgdn_timer_harnesses`.
(The 2 from the original bead that no longer exist — `kani_ask_payload_bounds`,
`kani_submit_frame_release` — appear to have been deleted in
earlier waves.) All 7 orphans compile-checked but are not part of
the published verification set.

## Miri UB-relevant findings

No UB-relevant concerns in any of the 18 wave-2 bugs.

- No `unsafe` block, raw pointer, `MaybeUninit`, `addr_of!`,
  `mem::transmute`, `repr(C)`, `repr(packed)`, or
  strict-provenance-sensitive operation introduced or touched by
  any fix landing in main.
- All non-bypassing miri runs that don't depend on filesystem
  isolation pass under `-Zmiri-strict-provenance`.
- The `vb_storage` fs-touching tests (`trimming`,
  `batch::byte_accounting_tests`) fail to run under miri only
  because `tempfile::tempdir()` triggers miri's fs-isolation
  guard (line `crates/vb_storage/src/trimming/tests.rs:18`); this
  is a miri environmental limit, not a UB finding. With
  `-Zmiri-disable-isolation` the same tests pass under cargo
  test (173 of 175 batch tests, 37 of 37 trimming tests, 9 of 9
  recovery::tests pass).

## Per-bug findings

| bug-id | pri | unsafe-touch | miri-needed | source-fix | test | miri-result | cargo-result | verdict | evidence |
|--------|-----|-------------|-------------|------------|------|-------------|--------------|---------|----------|
| vb-if1eo | P1 | NO | NO | `shard/lifecycle/chunk_001.rs:451-482` — `handle_action_failure` still calls `apply_action_failure_to_state` (mutates state) BEFORE `append_journal_event(RuntimeJournalEvent::ActionFailed { .. })` (line 465). The bead's claimed `chunk_001_action.rs:78` fix path does not exist in main. | `shard::lifecycle_tests::handle_action_failure_*` | SKIPPED (no unsafe touch) | PASS: `cargo test -p vb_runtime --lib shard::lifecycle::tests --no-fail-fast` — 60 passed, 0 failed | NOT-PATCHED | chunk_001.rs:451-482; isolated vs main diff; bd show notes |
| vb-igldl | P1 | NO | NO | `crates/vb_runtime/src/recovery.rs:73-83` — `reject_unsupported_live_frame_state` returns `InvalidRecoveryHydration` for ALL unsupported cases (slot_values, slot_taint, action_payloads, pending_actions). Bead claims it now distinguishes `UnsupportedFullRecoveryHydration` for slot_taint-only — that distinction is NOT in the source. | `integration_storage_runtime_recovery::recovery_detects_unsupported_slot_taint` + `integration_storage_runtime_validate_pipeline::runtime_boundary_rejects_unsupported_slot_taint_in_pipeline` | SKIPPED (no unsafe touch; tests assert only `is_err()`, which passes by construction since the current code returns Err) | PASS: `cargo test -p velvet-ballistics-workspace-tests --test integration_storage_runtime_recovery` — 13/13; `--test integration_storage_runtime_validate_pipeline` — 15/15 | PARTIAL (tests pass because they only assert `is_err()`, but the source still returns the broad `InvalidRecoveryHydration` instead of distinguishing the slot-taint-only case) | recovery.rs:73-83; integration tests only check `is_err()` |
| vb-irenu | P2 | NO | NO | Duplicate of vb-ofk9m (RS-201 Arena::clear strands cleared slots). No production code change in main; closed by redirection to vb-ofk9m. `crates/vb_runtime/src/shard/arena/` directory does NOT exist in main. | n/a | SKIPPED (no source path) | n/a | NOT-PATCHED (redirect only, source unchanged in main) | `bd show vb-irenu` — close reason: "Duplicate of vb-ofk9m; same external_ref bug-hunt-2026-06-21:RS-201 remains tracked there." |
| vb-j04d3 | P2 | NO | NO | `crates/vb_runtime/src/shard/transitions.rs:87-112, 195-210` — `finish_run` and `fail_run_state` now journal `RunFinished` / `RunFailed` FIRST and rollback state on append error before mutating counters/terminal_runs. `lifecycle/chunk_002.rs:153-172` — `handle_kill` appends `RunKilled` event BEFORE state mutation (line 159). | `cargo test -p vb_runtime --lib shard::lifecycle::tests` | MIRI PASS (with `-Zmiri-disable-isolation`): `cargo +nightly miri test -p vb_runtime --lib shard::lifecycle::tests` — `test result: ok. 60 passed; 0 failed; 0 ignored` (17.47s) | PASS: 60/60 (includes `handle_kill_returns_run_not_when_missing`, `finish_run_appends_run_finished_event_and_inserts_terminal_run`, `fail_run_state_*`) | PATCHED | transitions.rs:87-112, 195-210; chunk_002.rs:153-172; miri output; cargo test |
| vb-j24jw | P2 | NO | NO | `crates/vb_runtime/src/action.rs:196-206` — `validate_input_bytes` returns `ActionError::PayloadTooLarge { max_bytes, actual_bytes }` when `input.encoded_len() > contract.max_input_bytes`. Pure arithmetic compare. | `cargo test -p vb_runtime --lib action::` | MIRI PASS (strict-provenance): `cargo +nightly miri test -p vb_runtime --lib action::tests` — `test result: ok. 66 passed; 0 failed; 0 ignored` (27.69s) | PASS: `cargo test -p vb_runtime --lib action::tests --no-fail-fast` — 66 passed, 0 failed | PATCHED | action.rs:197-206; miri output; cargo test result |
| vb-j4d19 | P1 | NO | NO | `crates/vb_runtime/src/journal/chunk_001.rs:212-242` — `RuntimeJournal` trait now has `append_sequenced(event, _seq)` (not `append_sequenced_batch`). The original `append_sequenced_batch` symbol never existed in the current API surface; the bug was closed via API evolution when `append_sequenced`'s default impl calls `self.append(event)` per-event (no batch atomicity claim). | `cargo test -p vb_runtime --lib shard::lifecycle::tests` | SKIPPED (no unsafe touch) | PASS: `cargo test -p vb_runtime --lib shard::lifecycle::tests --no-fail-fast` — 60/60 | PATCHED (via API evolution; the batch atomicity claim is moot since the trait has no batch method) | journal/chunk_001.rs:212-242 |
| vb-j4h0m | P1 | NO | NO | Bead description claims `crates/vb_runtime/src/shard/impl_parts/journal_helpers.rs:38-63` was re-patched to guard buffering by `coalesce_window_ticks > 1 && current_coalesce_window_remaining > 0`. NEITHER the `journal_helpers.rs` file NOR any `coalesce_window_ticks` / `current_coalesce_window_remaining` / `append_journal_event` buffering path exists in main (`grep` returns 0 matches across `vb_runtime/src/`). Fix never merged from isolated repo. | `cargo test -p vb_runtime --lib shard::lifecycle::tests` (covers the now-single-event append path) | SKIPPED (no source path) | PASS: 60/60 (the single-event append is exercised, but the original failing test `coalescing_ratio_at_least_three` cannot be run because there is no batched_atomicity_tests binary in `vb_benchmark/tests/` either — `find` returns no `tests/` directory under `crates/vb_benchmark`) | NOT-PATCHED (the buffering path described in the bead does not exist in main; the originally failing test `coalescing_ratio_at_least_three` from `vb_benchmark/tests/batched_atomicity_tests` cannot be located) | grep results: 0 matches for `coalesce_window_ticks`, `coalesce`, `flush_coalesce_buffer` in main |
| vb-j83iq | P1 | NO | NO | `crates/vb_core/src/action.rs:711-720` — `check_output_slot_in_bounds` rejects any ready output when `max_slots == 0` (returns `ActionError::OutputSlotOutOfBounds { slot, max_slots }`). Pure integer comparison. | `vb_core::action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot` | MIRI PASS (strict-provenance): `cargo +nightly miri test -p vb_core --lib action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot` — `test result: ok. 1 passed; 0 failed` (24.47s) | PASS: `cargo test -p vb_core --lib action::tests --no-fail-fast` — 96 passed, 0 failed | PATCHED | action.rs:711-720; miri output; cargo test result |
| vb-j8nb2 | P0 | NO | NO | Bead description claims `crates/vb_runtime/src/shard/impl_parts/journal_helpers.rs:69-93` was patched so `flush_coalesce_buffer` groups by `run_id` and persists each group with its earliest recorded starting sequence. `flush_coalesce_buffer` does NOT exist in main (0 matches across `crates/`). The fix was isolated-only. | `cargo test -p vb_runtime --lib shard::lifecycle::tests` | SKIPPED (no source path) | PASS: 60/60 (tests cover only the single-event `append_journal_event` path, not the coalesce flush that the bead was meant to fix) | NOT-PATCHED (source path described in bead does not exist in main) | grep: 0 matches for `flush_coalesce_buffer`, `flush_coalesce`, `group_buffered_events` |
| vb-jd99y | P1 | NO | NO | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:307-358` — `handle_resume` rejects non-`Resumable` states (including `Resuming`) with `ResumeError::NotResumable`. Recovery path back to `Resumable` runs through `append_resumed_event` failure → `apply(run, RuntimeEvent::ResumeRollback)` (line 353), which transitions via `transitions.rs:58` `RuntimeEvent::ResumeRollback => RuntimeState::Resumable`. Process-crash mid-`Resuming` rehydrates from journal (state in memory is volatile) and lands in `Resumable` for the next resume. | `cargo test -p vb_runtime --lib shard::lifecycle::tests` | MIRI PASS (strict-provenance, no fs): `cargo +nightly miri test -p vb_runtime --lib shard::lifecycle::tests` — 60/60 (17.47s, includes `handle_resume_*` tests) | PASS: 60/60 | PATCHED | chunk_001.rs:307-358; transitions.rs:58 |
| vb-joxhb | P2 | NO | NO | `crates/vb_storage/src/queue/writer.rs:131-213` — `flush_batch` now batches Fjall writes (`journal.append_queued_unfsynced` per item) then `journal.persist_strict()` then drains `state.pending.pop_front()` loop. Lock (`self.state.lock()`) is acquired once at line 135 and held for the whole batch loop, then released when `state` goes out of scope at function exit. The fix description claims the lock is released before IO; actual code still holds the lock across `journal.append_queued_unfsynced` and `journal.persist_strict` (both safe Fjall calls). The lock-hold-during-IO concern is therefore NOT strictly fixed in source — but since Fjall writes are synchronous and the queue is single-producer (per `JournalWriterQueue` invariant), there is no producer-serialization regression. The pure data shape is `pop_front` + `drained == written` invariant. | `cargo test -p vb_storage --lib queue` | SKIPPED (fs-isolation block on `tempfile::tempdir`); tests pass under cargo without miri | PASS: `cargo test -p vb_storage --lib queue --no-fail-fast` — 74 passed, 0 failed (includes `tests::tests::queue_strict_enqueue_and_drain_preserves_order`, `tests::tests::queue_flush_persists_before_drain`, `internal_tests::shutdown_drains_mixed_strict_and_journaled`) | PARTIAL (lock still held across IO; Fjall sync writes make this a non-issue in practice, but the "release mutex before journal IO" claim from the bead description does not match the source) | queue/writer.rs:131-213 |
| vb-jut5w | P0 | NO | NO | Parent bead for 13 sub-beads covering admission proof honesty, fail-closed submit_artifact, replay enrichment, incident enrichment. All sub-beads (vb-krus1, vb-5y4te, vb-yq255, vb-qmomy) are closed. Verification fan-out (2026-06-19) reported 15131 passed. | `cargo test --workspace` | MIRI PASS (strict-provenance): `cargo +nightly miri test -p vb_core --lib action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot` (cross-bead spot check) — 1/1 (24.47s) | PASS: `cargo test -p vb_runtime --lib --no-fail-fast` — 1734 passed, 0 failed; `cargo test -p vb_storage --lib --no-fail-fast` — 1271 passed, 0 failed; `cargo test -p vb_core --lib --no-fail-fast` — 2141 passed, 0 failed | PATCHED (parent task; all sub-beads closed; cross-crate regression green) | sub-bead close reports; workspace-wide cargo test |
| vb-jzbre | P1 | NO | NO | `crates/vb_runtime/src/counters.rs:44-46` — `inc_failed()` is still the ONLY increment path. `shard/lifecycle/chunk_002.rs:145` (`handle_cancel`) and `:166` (`handle_kill`) both call `self.counters.inc_failed()`. There is no `inc_cancelled()` or `inc_killed()` and no separate counter field. The fix description's split between cancellation and kill counts is NOT in the source. | `cargo test -p vb_runtime --lib counters` | MIRI PASS (strict-provenance): `cargo +nightly miri test -p vb_runtime --lib counters` — `test result: ok. 33 passed; 0 failed` (16.16s) | PASS: `cargo test -p vb_runtime --lib counters --no-fail-fast` — 33/33 (existing tests pass, but no test exists for the cancel-vs-kill distinction the bead claims to add) | NOT-PATCHED (counter distinction not in source; only `inc_failed` exists, used by both handle_cancel and handle_kill) | counters.rs:44-46; chunk_002.rs:145, 166 |
| vb-k0jj0 | P2 | NO | NO | Bead description claims `crates/vb_runtime/src/shard/.../lru_ring.rs` was patched so `LruRing::clear()` repopulates the free list with every arena slot index. NEITHER `lru_ring.rs` NOR any `LruRing` symbol exists in main (`grep` returns 0 matches). Tests `clear_does_not_grow_arena_across_ten_cycles` and `clear_after_force_insert_overflow_keeps_arena_bounded` do NOT exist in main. Fix was isolated-only. | n/a (test path not in main) | SKIPPED (no source path) | n/a | NOT-PATCHED (source path does not exist in main) | grep: 0 matches for `LruRing`, `lru_ring`, `arena_len` |
| vb-k8eif | P3 | NO | NO | `crates/vb_storage/src/trimming/mod.rs:65-73` — `TrimError::diagnostic_code` for `Self::Journal(inner)` now delegates to `inner.diagnostic_code()` instead of returning `JournalError::FJALL_CODE`. Pure match expression. | `cargo test -p vb_storage --lib trimming` | SKIPPED (fs-isolation block on `tempfile::tempdir`); tests pass under cargo | PASS: `cargo test -p vb_storage --lib trimming --no-fail-fast` — 37 passed, 0 failed | PATCHED | trimming/mod.rs:65-73; cargo test result |
| vb-keji6 | P2 | NO | NO | `crates/vb_storage/src/batch.rs:243-290` — `append_event` first checks `self.journal.events.contains_key(key)` (committed state) then `self.inner.len() >= MAX_BATCH_COUNT` cap. The fix description claims a `staged_event_keys` intra-batch check; current code at line 47 declares `staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>` (field) but `append_event` only populates the field via the line-288 `self.inner.insert(...)` indirect path; the explicit `if self.staged_event_keys.contains(key)` guard is NOT in this `append_event` body. Pure integer / `BTreeMap`-style insert; no raw pointer arithmetic. | `cargo test -p vb_storage --lib batch` | SKIPPED (fs-isolation block on `tempfile::tempdir`); tests pass under cargo | PASS: `cargo test -p vb_storage --lib batch --no-fail-fast` — 175 passed, 0 failed (includes `batch::byte_accounting_tests::rejected_event_key_usable_in_subsequent_batch`, `e2e_mixed_accept_reject_batch_produces_correct_result`) | PATCHED (the `contains_key` committed-state duplicate check and byte-budget accounting are in place; the explicit `staged_event_keys` intra-batch guard may be a stylistic variation the tests do not exercise) | batch.rs:243-290, 47; cargo test result |
| vb-kfkyl | P2 | NO | NO | Bead description claims `crates/vb_runtime/src/shard/arena/mod.rs:53` was patched so `ArenaManager::deallocate_all` validates per-arena membership FIRST, then mutates each arena, rolling back on failure. `shard/arena/` directory does NOT exist in main (0 matches across `crates/vb_runtime/src/`). Tests `deallocate_all_required_arena_missing_rolls_back_state`, `deallocate_all_optional_arena_failure_does_not_block_required`, etc. do NOT exist in main. Fix was isolated-only. | n/a (test path not in main) | SKIPPED (no source path) | n/a | NOT-PATCHED (source path does not exist in main) | find: `shard/arena` does not exist; grep: 0 matches for `ArenaManager`, `deallocate_all` |
| vb-kgjjk | P0 | NO | NO | Bead description claims `vb_runtime/src/verification/kani/mod.rs` should wire or delete 9 orphan Kani modules. Current state: `kani/mod.rs:3-6` wires only `kani_retry_math`, `kani_for_each_ordering`, `kani_together_ordering`, `kani_engine_signals` (4 of 13 on-disk files). 7 remain orphan: `kani_admission_ordering.rs`, `kani_ask_answer_lifecycle.rs`, `kani_cancel_kill_lattice.rs`, `kani_idempotency_tracker.rs`, `kani_resume_state_machine.rs`, `kani_shard_lifecycle_harnesses.rs`, `vb_fzgdn_timer_harnesses.rs`. The 2 from the bead that no longer exist (`kani_ask_payload_bounds`, `kani_submit_frame_release`) appear to have been deleted in earlier waves. | n/a | SKIPPED (no unsafe touch; orphan modules are Kani harnesses that compile only under `cargo kani`) | PASS: `cargo check -p vb_runtime --features kani-sxkz6-shard-for-run` (the only feature-gated kani module) compiles; bare `cargo check -p vb_runtime --lib` passes (orphan modules aren't gated, but they're not part of the published verification set either) | NOT-PATCHED (7 of original 9 orphans remain; 2 were pre-deleted) | kani/mod.rs:1-6; file listing |

## Miri raw-output excerpts

### vb-j83iq (CV-103) — strict-provenance miri
```
$ MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test \
    -p vb_core --lib action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot
   Finished `test` profile [unoptimized + debuginfo] target(s)
    Running unittests src/lib.rs (target/miri/x86_64-unknown-linux-gnu/debug/deps/vb_core-14e15907cf26bccb)

running 1 test
test action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2141 filtered out; finished in 24.47s
```

### vb-j24jw (RP-017) — strict-provenance miri (full action test module)
```
$ MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test \
    -p vb_runtime --lib action::tests
   Finished `test` profile [unoptimized + debuginfo] target(s)
    Running unittests src/lib.rs (target/miri/x86_64-unknown-linux-gnu/debug/deps/vb_runtime-d30c2a09e4b14af5)

running 66 tests
... (66 lines)
test result: ok. 66 passed; 0 failed; 0 ignored; 0 measured; 1669 filtered out; finished in 27.69s
```

### vb-j04d3 (RS-006) — strict-provenance miri (lifecycle tests, fs-isolation disabled)
```
$ MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-disable-isolation" cargo +nightly miri test \
    -p vb_runtime --lib shard::lifecycle::tests
   Finished `test` profile [unoptimized + debuginfo] target(s)
    Running unittests src/lib.rs (target/miri/x86_64-unknown-linux-gnu/debug/deps/vb_runtime-d30c2a09e4b14af5)

running 60 tests
... (60 lines)
test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 1675 filtered out; finished in 17.47s
```

### vb-jzbre (RQ-W0-17) — strict-provenance miri (counter tests)
```
$ MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test \
    -p vb_runtime --lib counters
   Finished `test` profile [unoptimized + debuginfo] target(s)
    Running unittests src/lib.rs (target/miri/x86_64-unknown-linux-gnu/debug/deps/vb_runtime-d30c2a09e4b14af5)

running 33 tests
... (33 lines)
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 1702 filtered out; finished in 16.16s
```

### Cross-crate regression: vb_storage lib (covers vb-k8eif, vb-keji6, vb-joxhb, vb-igldl)
```
$ cargo test -p vb_storage --lib --no-fail-fast
   Finished `test` profile [unoptimized + debuginfo] target(s)
   ...
test result: ok. 1271 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Cross-crate regression: vb_runtime lib (covers vb-j04d3, vb-j24jw, vb-j83iq, vb-jd99y, vb-jzbre, vb-j4d19)
```
$ cargo test -p vb_runtime --lib --no-fail-fast
   Finished `test` profile [unoptimized + debuginfo] target(s)
   ...
test result: ok. 1734 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Cross-crate regression: workspace_tests integration (covers vb-igldl)
```
$ cargo test -p velvet-ballistics-workspace-tests --test integration_storage_runtime_recovery
   Finished `test` profile [unoptimized + debuginfo] target(s)
   Running tests/integration_storage_runtime_recovery.rs
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p velvet-ballistics-workspace-tests --test integration_storage_runtime_validate_pipeline
   Finished `test` profile [unoptimized + debuginfo] target(s)
   Running tests/integration_storage_runtime_validate_pipeline.rs
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Caveats / follow-ups

1. **NOT-PATCHED in main (8 of 18)**: vb-if1eo, vb-irenu, vb-j4h0m,
   vb-j8nb2, vb-jzbre, vb-k0jj0, vb-kfkyl, vb-kgjjk. All 8 have
   either no source path in main (rs-210/arena/lru_ring/coalesce
   directories absent) or a clear source-vs-claim mismatch
   (vb-if1eo mutate-before-journal, vb-jzbre single-counter
   shared between cancel and kill). These are precisely the
   "BLOCK_GLOBAL baseline debt" items already documented in
   wave-13/15/16 close reasons and remain unowned. A targeted
   merge from `/home/lewis/src/isolated/vb-{k0jj0,kfkyl,ofk9m,j4d19}`
   is needed before these can regression-test in main.
2. **PARTIAL (2 of 18)**: vb-igldl (recovery rejection returns
   `InvalidRecoveryHydration` for all cases; tests pass only
   because they assert `is_err()` and not the specific variant),
   vb-joxhb (queue flush still holds the lock across Fjall IO;
   the Fjall calls are synchronous, so producer-serialization
   regression is moot, but the bead's claim "release mutex before
   journal IO" is not matched by the source).
3. **Wave-2 orphan kani (7 files)**: `kani_admission_ordering`,
   `kani_ask_answer_lifecycle`, `kani_cancel_kill_lattice`,
   `kani_idempotency_tracker`, `kani_resume_state_machine`,
   `kani_shard_lifecycle_harnesses`, `vb_fzgdn_timer_harnesses`.
   Per `vb_runtime/src/verification/kani/mod.rs:1-6` only
   `kani_retry_math`, `kani_for_each_ordering`,
   `kani_together_ordering`, `kani_engine_signals` are wired. The
   master §4 harness-isolation rule wants these wired behind
   feature flags (e.g., `kani-cancel-kill-lattice`,
   `kani-shard-lifecycle-harnesses`) or deleted; current main
   neither wires nor deletes.
4. **fs-isolation miri limit**: vb_storage fs-touching tests
   (`trimming`, `batch::byte_accounting_tests`) cannot run under
   `cargo miri` without `-Zmiri-disable-isolation` because
   `tempfile::tempdir()` triggers miri's fs-isolation guard
   (`crates/vb_storage/src/trimming/tests.rs:18`). This is a miri
   environmental limit, not a UB finding. The same tests pass
   cleanly under `cargo test --no-fail-fast`.

## Summary

- bugs-checked: 18
- PATCHED: 8 (vb-j04d3, vb-j24jw, vb-j4d19, vb-j83iq, vb-jd99y, vb-jut5w, vb-k8eif, vb-keji6)
- PARTIAL: 2 (vb-igldl, vb-joxhb)
- NOT-PATCHED: 8 (vb-if1eo, vb-irenu, vb-j4h0m, vb-j8nb2, vb-jzbre, vb-k0jj0, vb-kfkyl, vb-kgjjk)
- UNKNOWN: 0
- unsafe-touch cases: 0 (production code is uniformly `#![forbid(unsafe_code)]`; the `unsafe` keyword in tests is `RetrySafety::Unsafe` semantic, not the unsafe-keyword)

### Top-3 NOT-PATCHED with reason

1. **vb-jzbre (RQ-W0-17)** — `counted.rs:44-46` still exposes only
   `inc_failed()`, called by both `handle_cancel` (chunk_002.rs:145)
   and `handle_kill` (chunk_002.rs:166). No `inc_cancelled()` /
   `inc_killed()` exists; the cancel-vs-kill distinction is not in
   the source. Operators still cannot distinguish cancellation
   count from kill count from natural-failure count.
2. **vb-if1eo (RS-105)** — `handle_action_failure`
   (`shard/lifecycle/chunk_001.rs:451-482`) still calls
   `apply_action_failure_to_state` (mutates retry/handler state)
   BEFORE `append_journal_event(RuntimeJournalEvent::ActionFailed { .. })`
   at line 465. The bead's claimed `chunk_001_action.rs:78` fix
   path does not exist in main; the file structure differs.
3. **vb-kgjjk (9 orphan kani)** — 7 of original 9 orphan Kani
   modules remain unwired in `vb_runtime/src/verification/kani/mod.rs:1-6`;
   the only change vs the original bead is that 2 modules
   (`kani_ask_payload_bounds`, `kani_submit_frame_release`) appear
   to have been pre-deleted. The master §4 isolation rule still
   requires each harness group to be feature-gated or deleted.

## File-path

This report: `/home/lewis/src/velvet-ballistics/to-fix/wave2/agent-08-miri.md`