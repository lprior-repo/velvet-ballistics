# Wave 5 — Agent 01 (holzman-rust B) Deep-Pass Review

**Scope:** bug IDs `vb-1rqz7.2`, `vb-1rqz7.32`, `vb-1rqz7.4`
**Working dir:** `/home/lewis/src/velvet-ballistics`
**Toolchain:** `nightly-2026-04-28-x86_64-unknown-linux-gnu`
**Date:** 2026-06-24

---

## Scope-mismatch note

The task template prescribes `cargo test -p vb_ipc --lib <test_name>`, but all three beads in this chunk are **storage** bugs (`vb_storage`), not IPC bugs. `vb_storage` is the correct package; the named regression tests live (per the close reasons) at `crates/vb_storage/src/journal/regression_tests_vb_1rqz7.rs`. That file **does not exist** (verified via `rtk find` and `rtk grep`). All targeted cargo runs below use `-p vb_storage`.

---

## Per-Bug Findings

| bug-id | pri | source-fix | test | targeted-cmd | result | verdict | evidence | holzman-violation |
|---|---|---|---|---|---|---|---|---|
| `vb-1rqz7.2` (SJ-003) | P0 | `crates/vb_storage/src/journal/injection.rs:15-54` — `inject_raw_event` and `inject_seq_gap` insert directly into `self.events` without acquiring `self.write_lock` and without a `contains_key` duplicate check. Compare to the canonical write path at `crates/vb_storage/src/journal/internal.rs:38-59` (`append_unfsynced`) which DOES use `self.write_lock` + `if self.events.contains_key(key)?` and returns `JournalError::DuplicateEvent`. The `WriteLockPoisoned` / `DuplicateEvent` machinery exists in `journal/core.rs:66` and `error/artifact.rs:67` but is bypassed by the injection path. | `inject_raw_event_rejects_duplicate_key` and `inject_seq_gap_rejects_duplicate_key` — neither test exists. `rtk find regression_tests_vb_1rqz7*` returns 0 matches; `rtk grep` for both test names returns 0 matches across all `*.rs` files. `rtk grep` for any test of `inject_raw_event` / `inject_seq_gap` returns 0 hits. | `cargo test -p vb_storage --lib inject_raw_event_rejects_duplicate_key --no-fail-fast` | `running 0 tests / 0 passed; 0 failed; 1273 filtered out` — test not compiled. | **NOT-PATCHED** | `journal/injection.rs:30,52` (`self.events.insert(key.to_vec(), value)?;` with no lock and no dedup); `journal/internal.rs:39-49` is the canonical pattern that the injection path should mirror; `journal/core.rs:66,120` (`write_lock: Mutex<()>` field exists); absence of `journal/regression_tests_vb_1rqz7.rs`. | None in the current code (no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`/`unsafe`); but the fix never landed, so the Holzman question is moot. The EARS God-Rule is still violated by the omission (no duplicate protection on a public injection API). |
| `vb-1rqz7.32` (SR-004) | P0 | `crates/vb_storage/src/recovery/replay/core.rs:223-235` — `load_snapshot` still conflates `Ok(None)` (key absent = missing snapshot) with `Err(JournalError::PostcardDecodeFailed)` (snapshot present but corrupt). The match arm is literally `Ok(None) | Err(crate::JournalError::PostcardDecodeFailed) => Err(RecoveryError::CorruptSnapshot { run, seq })`. The fix should split these: `Ok(None)` should return a distinct "missing snapshot" error (e.g. a new `RecoveryError::MissingSnapshot { run, seq }` variant or reuse `NoRecoveryData`), and only the `PostcardDecodeFailed` arm should map to `CorruptSnapshot`. The `RecoveryError` enum (`recovery/types.rs:101-114`) currently has `NoRecoveryData` and `CorruptSnapshot` but **no `MissingSnapshot` variant**. | No test exists for the missing-vs-corrupt distinction. `rtk grep` for `load_snapshot` finds only the function definition + a comment in `core.rs:222`. The `recovery/tests.rs:2415, 2481` assertions that test `CorruptSnapshot` only exercise the PostcardDecodeFailed path indirectly. | `cargo test -p vb_storage --lib load_snapshot 2>&1 \| tail -10` (no such test) | `running 0 tests` — function is uncovered. Existing `CorruptSnapshot` tests pass but do not exercise the conflation. | **NOT-PATCHED** | `recovery/replay/core.rs:230` (`Ok(None) | Err(PostcardDecodeFailed) => CorruptSnapshot`); `recovery/types.rs:101-114` (no `MissingSnapshot` variant); `journal/snapshots.rs:33-45` (`snapshot()` returns `Ok(None)` when key absent). | None — the `match` is exhaustive on the two paths and uses the typed `JournalError`/`RecoveryError` enums; the issue is that the fix never landed. |
| `vb-1rqz7.4` (SR-001) | P0 | `crates/vb_storage/src/recovery/replay/core.rs:196-219` — `recover_full_journal` still calls `journal.events_for_run(run)?;` (line 203). `events_for_run` is implemented in `journal/replay.rs:53-55` as `events_for_run_bounded(run, EventReplayLimit::DEFAULT)`, and `events_for_run_bounded` (`replay.rs:72-85`) consults `self.latest_durable_snapshot_seq(run)` and starts at `next_seq(snapshot_seq)` — the snapshot-tail optimization. The close reason claims `recover_full_journal` should call a new `events_for_run_full` that reads from `EventSeq::ZERO`. **There is no `events_for_run_full` function in the source** (`rtk grep events_for_run_full` returns 0 matches). Only `events_for_run`, `events_for_run_bounded`, and `events_for_run_from` exist. | `recover_full_journal_reads_history_before_snapshot` — does not exist. `rtk find regression_tests_vb_1rqz7*` returns 0 matches; `rtk grep` for the test name returns 0 hits. The only test that covers `recover_full_journal` is `recover_full_journal_returns_no_recovery_data_when_empty` (`recovery/tests.rs:1772-1783`) which exercises the empty-journal path, NOT the snapshot-tail path. | `cargo test -p vb_storage --lib recover_full_journal_reads_history_before_snapshot --no-fail-fast` | `running 0 tests / 0 passed; 0 failed; 1273 filtered out` — test not compiled. Adjacent `recover_full_journal_returns_no_recovery_data_when_empty` PASSES but does not cover the bug. | **NOT-PATCHED** | `recovery/replay/core.rs:203` (`journal.events_for_run(run)?;` — the bug site); `journal/replay.rs:77-83` (snapshot-tail start position); absence of `events_for_run_full` symbol; absence of `journal/regression_tests_vb_1rqz7.rs`. | None — the existing `recover_full_journal` body has no `unwrap`/`expect`/`panic`/`unchecked_index`/`unchecked_cast`; the only Holzman concern is that `events_for_run_bounded` itself uses `saturating_add` (`replay.rs:48`) which is correct. The fix is simply missing. |

---

## Counts

- **bugs-checked:** 3
- **PATCHED:** 0
- **NOT-PATCHED:** 3 (`vb-1rqz7.2`, `vb-1rqz7.32`, `vb-1rqz7.4`)
- **PARTIAL:** 0
- **UNKNOWN:** 0

---

## Top-3 NOT-PATCHED with reason

1. **`vb-1rqz7.2` (SJ-003, P0) — injection path bypasses write lock and dedup.** `crates/vb_storage/src/journal/injection.rs:15-54` directly calls `self.events.insert(...)` with no `self.write_lock` acquisition and no `contains_key` check. The canonical write path in `journal/internal.rs:38-59` (`append_unfsynced`) already implements the right pattern (`lock().map_err(|_| WriteLockPoisoned)?; if self.events.contains_key(key)? { Err(DuplicateEvent) }`). The injection path is a `pub` API on `FjallJournal` and can be called by any holder of a `&FjallJournal` — duplicate `(run, seq)` keys can be silently overwritten, breaking the storage invariant that "Storage rows are never silently overwritten" (KIRK invariant for SJ-003). Fix: copy the lock + `contains_key` pattern from `append_unfsynced` into both `inject_raw_event` and `inject_seq_gap`, and add a duplicate-rejection test in `journal/regression_tests_vb_1rqz7.rs` (the file the close reason claims was added but does not exist).

2. **`vb-1rqz7.4` (SR-001, P0) — `recover_full_journal` still uses snapshot-tail replay.** `crates/vb_storage/src/recovery/replay/core.rs:203` still calls `journal.events_for_run(run)?;`. The implementation of `events_for_run` (`journal/replay.rs:53-55`) delegates to `events_for_run_bounded` which calls `self.latest_durable_snapshot_seq(run)` and starts replay at `next_seq(snapshot_seq)` — so any events written **before** a snapshot's seq are silently skipped. The close reason claims a new `events_for_run_full` function was added that reads from `EventSeq::ZERO`; that function does not exist (`rtk grep events_for_run_full` = 0 hits). The named regression test `recover_full_journal_reads_history_before_snapshot` does not exist either. The only adjacent test that passes (`recover_full_journal_returns_no_recovery_data_when_empty` at `recovery/tests.rs:1772`) covers the empty-journal case and never touches the snapshot path. Fix: add `pub fn events_for_run_full(&self, run: RunId) -> Result<Vec<JournalEvent>, JournalError>` to `journal/replay.rs` that calls `self.events_for_run_from(run, EventSeq::new(0), EventSeq::new(0), EventReplayLimit::DEFAULT)`, switch `recover_full_journal` to use it, and add the regression test (run must have pre-snapshot events that are present after `recover_full_journal`).

3. **`vb-1rqz7.32` (SR-004, P0) — `load_snapshot` conflates missing with corrupt.** `crates/vb_storage/src/recovery/replay/core.rs:230` uses the pattern `Ok(None) | Err(JournalError::PostcardDecodeFailed) => Err(RecoveryError::CorruptSnapshot { run, seq })`, mapping both "key absent" and "decode failure" to the same `CorruptSnapshot` error. A caller that probes for a snapshot and gets `CorruptSnapshot` cannot tell whether the snapshot was lost (retriable, or recoverable from journal replay) or whether on-disk bytes are corrupted (operator intervention required). The `RecoveryError` enum (`recovery/types.rs:101-114`) has no `MissingSnapshot` variant; the fix would need to add one (or repurpose `NoRecoveryData { run }` with a more specific field) and split the match. `load_snapshot` has **zero direct test coverage** — `rtk grep "load_snapshot"` finds only the function definition and a comment.

---

## DEEP-DIVE disagreements with prior waves

| wave / agent | prior verdict | this review | reason for disagreement |
|---|---|---|---|
| wave1 `11-validation-wave-1.md:103` | `vb-1rqz7.4` NOT-PATCHED (top-15 list) | **agree** | `recover_full_journal` at `recovery/replay/core.rs:203` still uses `events_for_run`; no `events_for_run_full`; no `recover_full_journal_reads_history_before_snapshot` test. |
| wave1 `11-validation-wave-1.md` (paraphrased: `vb-1rqz7.32` not in top-15) | NOT-PATCHED is implicit from "still uses `Ok(None) | Err(PostcardDecodeFailed)`" pattern that was known in wave 1 | **agree** | `load_snapshot` match arm is unchanged from wave 1; `RecoveryError::MissingSnapshot` variant still absent. |
| wave2 `11-validation-wave-2.md:53` | `vb-1rqz7.2` NOT-PATCHED (Phantom Closures table — "Lacking `write_lock`/`contains_key`") | **agree** | `injection.rs` still calls `self.events.insert` directly; `internal.rs:38-49` proves the right pattern exists in the same module; regression file still missing. |
| wave2 `agent-00-holzman-rust-A.md:25` | `vb-1rqz7.2` NOT-PATCHED | **agree** | Same evidence: no `write_lock` acquisition, no `contains_key`, no regression test. |
| wave3 (inferred from wave4 `agent-01-holzman-rust-B.md:53` and the prevailing consensus) | `vb-1rqz7.4` NOT-PATCHED in wave 1 still; wave 3 re-confirmed | **agree** | Close reason was reopened-or-bounced at every wave; source still has the bug. |
| wave3 `agent-01-holzman-rust-B.md` (if covered) | coverage status of `vb-1rqz7.32` | **no disagreement possible — not previously surfaced** | `load_snapshot` conflation is a P0 issue that the close reason claims was fixed; wave 1 and wave 2 surfaced `vb-1rqz7.4` and `vb-1rqz7.2` but not `vb-1rqz7.32`. This is consistent with the fix never landing: had the fix landed, the variant split would have been visible. |

---

## Second-order issues observed

1. **All three beads share the same phantom-closure pattern.** The close reasons describe a fix and name a regression test file (`journal/regression_tests_vb_1rqz7.rs`); neither the fix nor the file exists. The bead was closed on intent, not on evidence. This is the textbook "Phantom Closure" anti-pattern from the wave 2 report (`11-validation-wave-2.md:44-57`): the close reason names symbols that don't exist. Three beads on the same parent (`vb-1rqz7`) closed on the same phantom pattern is a strong signal that the parent bead itself should be re-opened and the children re-issued under a real evidence requirement (e.g. "must include a `regression_tests_vb_1rqz7.rs` file verified by `rtk find`").

2. **The injection path is a public API on `FjallJournal` (`pub fn inject_raw_event` / `pub fn inject_seq_gap` at `journal/injection.rs:15, 37`).** It is documented as a "DANGER" disaster-recovery tool but is reachable from any crate that imports `vb_storage::FjallJournal`. Without lock + dedup, two concurrent callers in the same process can race `events.insert` on the same key; the LSM-tree merge on the same key is safe but the *intent* (idempotent injection) is broken. The canonical `append_unfsynced` path proves the lock + `contains_key` pattern was known; the injection path was just never updated.

3. **`events_for_run_bounded`'s snapshot-tail optimization is the correct behavior for normal hot-path replay.** The bug is that `recover_full_journal` reuses it. The fix is to add a separate `events_for_run_full` (reads from `EventSeq::ZERO`, validates the first event's `seq() == 0` or skips if no events) and route `recover_full_journal` through it. A single shared helper with a `start_at_snapshot: bool` parameter would be a more invasive change and risks touching every other call site of `events_for_run`; the named-function split is the smaller, safer fix and matches the close reason's stated design.

4. **`RecoveryError` enum growth.** Adding a `MissingSnapshot { run, seq }` variant (the most natural fix for SR-004) will require updating the `Display` impl in `recovery/types.rs`, the `From` impls in `error/`, and every match in `recovery/replay/summary.rs` and `recovery/hydrate.rs` (5+ sites at minimum). The wave 1 report (`11-validation-wave-1.md:115`) notes "Workspace lint clean: `clippy::unwrap_used = forbid`, `clippy::panic = forbid`" — adding a new variant should not break those lints, but every existing match on `RecoveryError` is `#[non_exhaustive]`-aware and will not need wildcard updates if the project does not use exhaustive matches on it.

5. **No Holzman regressions in any of the three NOT-PATCHED paths.** The current code at `injection.rs`, `recovery/replay/core.rs` (lines 196-235), and `journal/replay.rs` is free of `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`/`unsafe`/unchecked-index/unchecked-arithmetic (verified by `rtk grep -nE "unwrap\(\)|expect\(|panic!|todo!|unimplemented!|dbg!|unsafe " /home/lewis/src/velvet-ballistics/crates/vb_storage/src/journal/injection.rs` and the same against `recovery/replay/core.rs` — both return zero matches). The dominant failure mode here is **incomplete/phantom fixes**, exactly as the wave 1 Holzman section (`11-validation-wave-1.md:112-118`) concluded: "Dominant failure mode is **incomplete or phantom fixes**, not Holzman regressions."

---

## Output file

`/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-01-holzman-rust-B.md`
