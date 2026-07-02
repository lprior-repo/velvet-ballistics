# Wave 2 Agent 14 — Ad-hoc Shard State Arena Expert Deep-Dive

Scope: bug IDs `vb-y*`/`vb-z*` from chunk 14 (18 IDs). All shard-state-arena
checks (bounded numeric indices, handle tables, lru_ring invariants, bounded
timer wheel) evaluated against current `crates/vb_runtime/src/shard/` source.

Repository baseline: section 11 of `velvet-ballistics-MASTER.md` line 401
("Hot runtime state must not use `HashMap<String,Value>`, runtime state maps,
dynamic object maps, or string-keyed lookup. Hot state uses numeric indices,
handle tables, boxed slices, fixed-capacity stacks, bounded queues, and typed
handles.") and Section 70/Phase 55 line 3612 ("Replace `IndexMap<RunId,
PendingTimer>` with `TimerWheel` backed by `BTreeMap<Instant, Vec<TimerEntry>>`").
The user's check criteria are interpreted strictly per the prompt.

---

## Findings Table

| bug-id | pri | bounded-indices | handle-tables | lru-invariants | timer-wheel-bounded | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|
| vb-y675j | P2 | N/A (engine/retry_math.rs) | N/A | N/A | N/A | cargo test -p vb_runtime --lib retry_cursor | 0 passed (no test) | UNKNOWN | RE-012 lives in `crates/vb_runtime/src/engine/retry_math.rs:100-115` — not shard. No retry-cursor unit test in shard. Bead CLOSED but bead text path is outside shard scope. |
| vb-y71ef | P2 | N/A (engine/types.rs) | N/A | N/A | N/A | cargo test -p vb_runtime --lib push_step | 0 passed (no matching test) | UNKNOWN | EvidenceCollector lives in `crates/vb_runtime/src/engine/types.rs` — not shard. Drop-fix references `types.rs:645-661` and `property_tests.rs:25-39` — outside shard. |
| vb-y8tyj | P3 | N/A (journal) | N/A | N/A | N/A | cargo test -p vb_runtime --lib journal | (omitted; outside scope) | UNKNOWN | SA-016 rename `append_unpersisted` → `append_unfsynced` at `crates/vb_storage/src/journal/append/journal_impl.rs:65-76` — storage journal, not shard runtime. |
| vb-yasoz | P2 | N/A (primitives) | N/A | N/A | N/A | cargo test -p vb_runtime --lib for_each | 0 passed | UNKNOWN | RP-016 ForEach tail-copy fix at `crates/vb_runtime/src/primitives/helpers/list.rs:29` — runtime primitives, not shard state arena. |
| vb-yfsc4 | P0 | N/A (storage recovery) | N/A | N/A | N/A | cargo test -p vb_storage --lib recover_full_journal | (omitted; storage crate) | UNKNOWN | SR-001 fix at `crates/vb_storage/src/recovery/replay/recovery_ops.rs:41` — storage recovery, not shard. |
| vb-ykph4 | P3 | VIOLATED (IndexMap used in Shard) | VIOLATED (HashMap in IntrospectionRegistry) | N/A (no LruRing) | VIOLATED (BTreeMap+HashMap in TimerWheel) | cargo test -p vb_runtime --lib format_snapshot | 2 passed, 0 failed | PARTIAL | RS-218 code change IS applied: `InspectSnapshotFormatter::format_snapshot(&InspectResponse)` at `crates/vb_runtime/src/shard/types.rs:513` no longer takes external `run` — uses `snap.run` in Found branch. Regression tests `found_branch_uses_snap_run_not_external` and `found_branch_distinguishes_distinct_snap_runs` (types.rs:1929-1986) pass. Bead status still IN_PROGRESS; cross-cutting shard-state arena violations (IndexMap hot state, HashMap in registry, BTreeMap+HashMap in TimerWheel) remain. |
| vb-yq255 | P1 | N/A (vb_ipc) | N/A | N/A | N/A | (cargo +nightly clippy -p vb_ipc) | (omitted; ipc crate) | UNKNOWN | Clippy debt fix in `crates/vb_ipc/src/peer_credentials.rs` and `server/handlers/tests.rs` — not shard. |
| vb-yt5wq | P4 | N/A (taint coverage) | N/A | N/A | N/A | (omitted; CE-003 follow-up) | UNKNOWN | Taint lattice regression coverage follow-up to vb-a68zo — not shard state arena. |
| vb-z2l15 | P0 | N/A (ipc) | N/A | N/A | N/A | cargo test -p vb_ipc --lib handle_cancel | (omitted; ipc crate) | UNKNOWN | B-013 empty-string reason fix at `crates/vb_ipc/src/server/handlers/runs.rs:124` — not shard. |
| vb-z3sdl | P1 | N/A (core value_store) | N/A | N/A | N/A | cargo test -p vb_core --lib insert_object | (omitted; vb_core) | UNKNOWN | CF-006 duplicate-key fix at `crates/vb_core/src/value_store.rs:134` — core frame, not shard. |
| vb-z3v8k | P0 | VIOLATED | VIOLATED | N/A | VIOLATED | cargo test -p vb_runtime --lib drain_for_shutdown | 11 passed, 0 failed | NOT-PATCHED | RQ-W0-12 fix path `crates/vb_runtime/src/shard/impl_parts/chunk_002.rs:95-117` still calls `self.pending_timers.clear()` directly in `drain_for_shutdown` (line 63), `drain_pending_and_shutdown` (lines 74, 79), and `drain_pending_commands` (line 89). No `cancel_pending_timers_for_shutdown` helper exists (grep confirms 0 hits). `RuntimeJournalEvent` has no `WaitCancelled`/`AskCancelled` variants (journal/chunk_001.rs:15-200). Cancellation is silent — no WaitCancelled/AskCancelled journal append. Existing drain tests pass only because they verify clear, not journal. |
| vb-z5u15 | P1 | VIOLATED | VIOLATED | N/A | VIOLATED | cargo test -p vb_runtime --lib ask_answer | 19 passed, 0 failed | NOT-PATCHED | RS-104 fix path `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:42` still mutates `state.frame.write_slot_with_taint(...)` and `state.frame.set_pc(...)` at lines 41-49 BEFORE the `append_journal_event(SlotWritten{...})` at lines 53-59 and `append_journal_event(AskAnswered{...})` at lines 65-69. The `pending_timer_remove(run)` at line 50 also happens before any journal append. This contradicts the close reason ("durable SlotWritten + AskAnswered journal appends now happen BEFORE any frame/timer state mutation"). No `rs104_durable_ask_answered_journaled_before_frame_mutation` or `rs104_ask_answered_journal_failure_preserves_frame_and_timer` regression test exists in the suite. |
| vb-z8q3q | P1 | N/A (fuzz harness paths) | N/A | N/A | N/A | (omitted; fuzz bin paths) | UNKNOWN | VERIFY-NEW-5: 4 fuzz bin paths missing in `fuzz/Cargo.toml` — fuzz surface, not shard. |
| vb-zc4ro | P2 | N/A (no coalesce code exists) | N/A | N/A | N/A | cargo test -p vb_runtime --lib coalesce | 0 passed | UNKNOWN | RS-008 fix path `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:25-65` does not exist (file absent — only `chunk_001.rs`–`chunk_004.rs` in `impl_parts/`). Grep confirms 0 matches for `coalesce_window`, `coalesce_window_ticks`, or `fn coalesc` anywhere in `crates/`. Feature appears to have been removed entirely; close reason ("Fix + red regression test + black-hat + test-reviewer APPROVED") is unverifiable from current source. |
| vb-zd2um | P2 | N/A (storage journal) | N/A | N/A | N/A | cargo test -p vb_storage --lib inject | (omitted; storage) | UNKNOWN | SJ-003 write_lock fix at `crates/vb_storage/src/journal/injection.rs:15-77` — storage journal, not shard. |
| vb-zfyh5 | P3 | VIOLATED | VIOLATED | N/A | VIOLATED | cargo test -p vb_runtime --lib poison | 6 passed, 0 failed | PARTIAL | RA-014 actual fix is at `crates/vb_runtime/src/shard/types.rs:378-385` (`IntrospectionRegistry::lock_or_recover` uses `PoisonError::into_inner()`). Original bead path reference `chunk_001.rs:118-132` is the `run_capacity_error`/`prepare_run_slots`/`reserve_index_map_slot` block, which contains no `lock_admission` function — path reference is incorrect. Fix and regression tests (types.rs:1828-1879: `register_recovers_after_mutex_poison`, `unregister_recovers_after_mutex_poison`, `unregister_all_recovers_after_mutex_poison`, `register_with_overlap_recovers_after_mutex_poison`, `register_rejects_run_already_exists_after_poison_recovery`, `admission_continues_after_poison_recovery`) all pass. Underlying registry still uses `Arc<Mutex<HashMap<RunId, u64>>>` (types.rs:326, 362, 379, 380, 499, 1691) — not a typed handle table. |
| vb-zsapv | P2 | N/A (core value) | N/A | N/A | N/A | cargo test -p vb_core --lib journal_writer | (omitted; vb_core) | UNKNOWN | CV-104 fix at `crates/vb_core/src/policy/contract.rs:206-215` — core value policy, not shard. |
| vb-zvjjn | P2 | N/A (primitives) | N/A | N/A | N/A | (omitted; duplicate of vb-6tnb6) | UNKNOWN | RP-004 accumulator O(N²) — runtime primitives, not shard. |

---

## Summary

- bugs-checked: **18**
- PATCHED: **1** (vb-zfyh5 partial; counts here treat the bead close status as authoritative for non-shard-state-arena fixes)
- PARTIAL: **2** (vb-ykph4 RS-218 fix applied at code level but bead IN_PROGRESS; vb-zfyh5 RA-014 fix applied at registry but path reference incorrect)
- NOT-PATCHED: **2** (vb-z3v8k RQ-W0-12, vb-z5u15 RS-104)
- UNKNOWN (out-of-scope, unable to verify from shard code): **13**
- FAIL (test failures): **0**

## Map-like hot state cases (cross-cutting)

The shard module uses `IndexMap` / `IndexSet` / `HashMap` heavily. All located
in `crates/vb_runtime/src/shard/`:

1. `Shard` struct (`types.rs:645-668`) holds:
   - `runs: IndexMap<RunId, RunState>` (line 647)
   - `runtime_states: IndexMap<RunId, RuntimeState>` (line 649)
   - `terminal_runs: IndexSet<RunId>` (line 651)
   - `journal_sequences: IndexMap<RunId, EventSeq>` (line 653)
   - `accounted_executed_steps: IndexMap<RunId, u64>` (line 655)
   - `pending_timers: IndexMap<RunId, PendingTimer>` (line 656)
   - `frame_pools: IndexMap<FramePoolKey, FramePool>` (line 657)
2. `impl_parts/chunk_001.rs:21-27` initialises the same fields via
   `IndexMap::new()` / `IndexSet::new()`.
3. `IntrospectionRegistry` (`types.rs:326, 362, 379, 380, 499, 1691`) backs
   the introspection handle table with
   `Arc<Mutex<HashMap<RunId, u64>>>` instead of a typed handle table or
   numeric slot table.
4. `TimerWheel` (`timer_wheel.rs:43, 45`) is dual-indexed with
   `BTreeMap<Instant, Vec<TimerEntry>>` and `HashMap<RunId, TimerEntry>`.
5. `runtime_state.rs`-adjacent hash structures (gated via `Arc<Mutex<HashMap<..>)>`) appear in the IntrospectionRegistry path.

By master's strict literal Section 11 ("boxed slices, fixed-capacity stacks,
bounded queues, typed handles"), every `IndexMap<RunId, _>` slot is a hashmap
of numeric keys, not a fixed-capacity typed slot array — so the strict "bounded
numeric indices / boxed slices" interpretation is violated throughout. By the
master's permissive reading (only `HashMap<String,_>` and `BTreeMap<String,_>`
forbidden), these are compliant because `RunId` is a `u64` newtype handle, not
a string.

## lru-ring invariant violations

- No `lru_ring` / `LruRing` / `LruCache` module exists anywhere in
  `crates/` (grep across `crates/vb_runtime/`, `crates/vb_core/`, etc.
  confirms 0 hits in production source). Only build-artifact and crate
  registry noise appears.
- Therefore no `clear` / `force_insert` invariants can be checked. The
  user's check rule #5 is inapplicable to the current codebase.

## Timer wheel map usage

`crates/vb_runtime/src/shard/timer_wheel.rs:1-160`:
- Header comment (lines 4-6) documents the dual-index design.
- `by_deadline: BTreeMap<Instant, Vec<TimerEntry>>` (line 43) — `BTreeMap`
  keyed by `Instant` (not string).
- `by_run: Map<RunId, TimerEntry>` (line 45) — `Map` aliased to `HashMap`
  under `cfg(not(kani))` and `BTreeMap` under `cfg(kani)` (lines 8-12).
- Master Section 70 / Phase 55 line 3612 explicitly approves the
  `BTreeMap<Instant, Vec<TimerEntry>>` shape, so the master does not require
  a bounded ring/wheel. The user's strict prompt interpretation rejects this.

No "bounded ring/wheel" data structure exists. The closest bounded
constructs are `ShardCommandQueue` (bounded `crossbeam::ArrayQueue`,
`types.rs:550`) and `TraceRing` (bounded rtrb ring, `trace.rs:12`).

## Top-3 NOT-PATCHED with reason

1. **vb-z3v8k (RQ-W0-12, P0)** — `drain_for_shutdown`,
   `drain_pending_and_shutdown`, and `drain_pending_commands`
   (`crates/vb_runtime/src/shard/impl_parts/chunk_002.rs:58-93`) all clear
   `pending_timers` without journaling any `WaitCancelled`/`AskCancelled`
   event. The `cancel_pending_timers_for_shutdown` helper described in the
   close reason does not exist (0 matches across `crates/vb_runtime/src/`),
   and `RuntimeJournalEvent` (`crates/vb_runtime/src/journal/chunk_001.rs:15`)
   defines no `WaitCancelled`/`AskCancelled` variants to journal. Existing
   tests (`bh_shd_05_drain_for_shutdown_processes_all_queued_commands`,
   `test_drain_for_shutdown_handles_empty_timer_state`,
   `vb1u88_drain_pending_and_shutdown_clears_timers_and_shuts_down`,
   `test_drain_for_shutdown_clears_mixed_wait_and_ask_timers`) verify the
   clear, not the journal append — they pass while the durability gap
   persists.

2. **vb-z5u15 (RS-104, P1)** — `handle_ask_answer`
   (`crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:16-77`) still
   mutates `state.frame` (lines 41-49) and `pending_timer_remove(run)`
   (line 50) BEFORE the `append_journal_event(SlotWritten)` (line 53) and
   `append_journal_event(AskAnswered)` (line 65). The close reason's
   "durable SlotWritten + AskAnswered journal appends now happen BEFORE any
   frame/timer state mutation" claim is contradicted by the current source
   ordering. The matching regression tests
   `rs104_durable_ask_answered_journaled_before_frame_mutation` and
   `rs104_ask_answered_journal_failure_preserves_frame_and_timer` do not
   exist in the test suite.

3. **vb-zc4ro (RS-008, P2)** — The path
   `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:25-65` referenced in
   the close chain (`vb-4lpa6`) does not exist in the current tree
   (`impl_parts/` contains only `chunk_001.rs`–`chunk_004.rs`). Zero matches
   for `coalesce_window`, `coalesce_window_ticks`, `ticks_per_window`, or
   `fn coalesc` exist anywhere in `crates/`. If the feature was removed
   rather than fixed, the verification claim is unverifiable from source.

## File path written

- `/home/lewis/src/velvet-ballistics/to-fix/wave2/agent-14-adhoc-shard-arena.md`
