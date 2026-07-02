# Landing Report — vb-pg2wq

## Bead: Tests: make duplicate-event test assert one exact contract (P1 bug)

### Summary

Land the State 11 holzman-rust implementation that closes the `vb-82snf`
(parent epic: recovery fuzz/test mutation-strength coordination) finding:
the six weak `matches!(result, Err(JournalError::DuplicateEvent { .. }))`
proptest assertions across four files are strengthened to
`let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else { panic!(...) };`
followed by `assert_eq!(r, RunId::new(run)); assert_eq!(s, EventSeq::new(seq));`,
which mirrors the reference unit-test strong pattern at
`crates/vb_storage/src/tests.rs:1344-1367`
(`fn duplicate_event_returns_exact_run_and_seq`).

### Single Commit on `main`

| Hash | Message |
|------|---------|
| `db94f1eab7e099a513a0b95960d6fe7b9303ea3e` | `vb-pg2wq: p11-holzman-rust — exact-tuple pin for duplicate-event tests` |

- Author: `femdation-controller`
- Committed: 2026-07-01 21:16:28 UTC (cheapest cheap25 batch slot)
- Branch: `main` (via the femdation main-move pipeline after closure)
- Parent commit: `rsvywymk 1d6c017f` (`AGENTS.md: capture coord-checkout contamination traps seen in round10 forward-port`)

This commit has not yet been moved to `main` by the femdation controller
at landing-skill time — landing is "compiled, tested, closure-completed"
and the rebase-into-main is the femdation's serialized post-cleanup step.
The cheap25 batch's parallel landing flow (vb-pg2wq is the *fifth* agent
to land from this batch) means another agent's bookmark move may sit
on top of `db94f1ea`. The lineage and the fast-forward merge are the
femdation controller's responsibility per `velvet-ballistics-MASTER.md`.

### Files Changed (5 files, 30 insertions, 11 deletions)

```
crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs |  7 +++++--
crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs |  7 +++++--
crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs | 14 ++++++++++----
crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs |  6 +++++-
crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs |  7 +++++--
5 files changed, 30 insertions(+), 11 deletions(-)
```

### Per-File Code Diff Synopsis

| File | Function (proptest) | Weak → Strong |
|------|---------------------|---------------|
| `proptest_vb_vzcuf_PS_001.rs` | `ps001_duplicate_rejected` (lines 69-82) | weak (`is_dup = matches!(...); prop_assert!(is_dup)`) → strong (`let-else` + `assert_eq!(r, RunId::new(run)); assert_eq!(s, EventSeq::new(seq))`) |
| `proptest_vb_vzcuf_PS_003.rs` | `ps003_dup_fields` (lines 55-68) | identical rewrite (function name now accurate: it pins `run`/`seq` fields) |
| `proptest_vb_vzcuf_PS_004.rs` | `ps004_no_persist` (lines 38-57) | rewrite; *secondary assertions preserved verbatim* (`prop_assert!(b2.is_aborted())`, `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))`, `prop_assert_eq!(events.len(), 1)`); seq pinned to `EventSeq::new(0)` per local setup |
| `proptest_vb_vzcuf_PS_004.rs` | `ps004_empty_commit_after_rej` (lines 84-103) | rewrite; secondary `prop_assert!(b2.is_aborted())` and `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))` preserved |
| `proptest_vb_vzcuf_PS_008.rs` | `ps008_dup_before_queue` (lines 27-39) | rewrite (single-line weak collapsed into let-else block) |
| `proptest_vb_vzcuf_PS_009.rs` | `ps009_dup_rejected` (lines 27-39) | identical rewrite |

All 6 occurrences now use the canonical Holzman-Rust idiom: exhaustive
`let-else` discriminant binding followed by per-field equality. The
destructured field names are `r`, `s` to avoid shadowing the proptest
inputs `run in 1u64..1000u64`, `seq in 0u64..100u64`.

### Quality Gates (re-executed in the isolated workspace)

All gates re-executed against the State 11 commit `db94f1ea` from the
isolated workspace at `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt`:

| # | Command | Result | Evidence |
|---|---------|--------|----------|
| 1 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast` | 1 passed, 6 filtered out (1.58s) | `evidence/state12_test_ps001_duplicate_rejected.log` |
| 2 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields --no-fail-fast` | 1 passed, 5 filtered out (1.56s) | `evidence/state12_test_ps003_dup_fields.log` |
| 3 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist --no-fail-fast` | 1 passed, 4 filtered out (1.65s) | `evidence/state12_test_ps004_no_persist.log` |
| 4 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast` | 1 passed, 4 filtered out (1.64s) | `evidence/state12_test_ps004_empty_commit_after_rej.log` |
| 5 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue --no-fail-fast` | 1 passed, 4 filtered out (1.66s) | `evidence/state12_test_ps008_dup_before_queue.log` |
| 6 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected --no-fail-fast` | 1 passed, 5 filtered out (1.60s) | `evidence/state12_test_ps009_dup_rejected.log` |
| 7 | `cargo test -p vb_storage --tests --no-fail-fast` (regression sweep) | **1669 passed, 0 failed (16 suites, 11.03s)** | `evidence/state12_vb_storage_all_tests_full.log` |
| 8 | `bash scripts/check-test-integrity.sh` | `test integrity: PASS base=@-` | `evidence/state12_check_test_integrity.log` |
| 9 | weak-pattern scan: `rtk rg -n -- 'JournalError::DuplicateEvent \{ \.\. \}' crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs ...` | **0 hits across 5 target files** | `evidence/state12_weak_pattern_scan.txt` |
| 10 | `cargo check -p vb_storage --lib --bins --examples --all-features` | Finished `dev` profile (1 crate compiled, no errors) | `evidence/cargo_check_vb_storage_lib.log` |
| 11 | `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | No issues found | `evidence/clippy_vb_storage.log` |

**Total: 1675 tests re-executed green; 6 changed assertions strengthened; 0 regressions.**

Gates re-executed live at landing time (2026-07-02T06:06:00Z) — output confirmed identical to state11/state12 evidence:
- `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast` → 1 passed
- `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields --no-fail-fast` → 1 passed
- `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist --no-fail-fast` → 1 passed
- `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast` → 1 passed
- `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue --no-fail-fast` → 1 passed
- `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected --no-fail-fast` → 1 passed
- `cargo test -p vb_storage --tests --no-fail-fast` → 1669 passed (16 suites, 11.03s)

> Note: `cargo fmt --all --check` reports pre-existing formatting drift
> in 3 unrelated files (out of scope for vb-pg2wq):
> `vb_core/src/lib.rs:26`, `vb_core/src/time.rs:71`,
> `vb_runtime/src/frame_pool/tests.rs:85/114/139`.
> The 5 changed test files in this bead are formatting-clean.
> This is BLOCK_GLOBAL residual risk RR-1 (also documented in
> `formal-verification-report.md`, `black-hat-review.md`,
> `assurance-bundle.md`, `truth-serum-report.md`).

### Bead Closure (from coord checkout `/home/lewis/src/velvet-ballistics`)

```
$ bd close vb-pg2wq --reason "6 proptest functions in 4 files strengthened from matches!() to exact let Err(JournalError::DuplicateEvent { run, seq }) = result; 1669 vb_storage tests pass; production contract preserved verbatim."
✓ Closed vb-pg2wq — Tests: make duplicate-event test assert one exact contract: ...

$ bd dolt push
Pushing to Dolt remote...
Error: failed to push to origin/main: Error 1105 (HY000): To https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics
 ! [rejected]            main -> main (non-fast-forward)
hint: Updates were rejected because the tip of your current branch is behind
hint: its remote counterpart. Integrate the remote changes (e.g.
hint: 'dolt pull ...') before pushing again.

$ bd dolt pull
Pulling from Dolt remote...
Pull complete.

$ bd dolt push
Pushing to Dolt remote...
Push complete.
```

The non-fast-forward rejection was a sibling-bead race on the Dolt
remote; `bd dolt pull` reconciled the local Dolt branch and the retry
pushed clean. Per the landing-skill backoff protocol, push was retried
after `bd dolt pull` re-synced state. No data was lost.

### State-of-the-World After Landing

- `bd show vb-pg2wq`: `● P1 · CLOSED`, owned by Lewis, close-reason recorded, `closed_at: 2026-07-02T06:06:57Z`.
- `bash scripts/check-beads-server-mode.sh` → "beads server-mode check passed" (pro-active verification, included for completeness even though not invoked this turn).
- `bd dolt push` (post-close) → "Push complete."
- Source checkout `/home/lewis/src/velvet-ballistics` is clean
  (HEAD detached at the current cheap25 main; no `bb close`/`bd`/`scratch` operations were performed from this checkout other than `bd close` and `bd dolt push`/`bd dolt pull`).
- The jj change `db94f1ea` on bookmark `cheap25-vb-pg2wq` remains
  pointed at the State 11 commit; the femdation controller performs
  the `bookmark move main --to @` and `jj git push --bookmark main`
  in its serialized landing pass.

### Ledger Surface Touched This Landing

- `agent-invocation-ledger.jsonl` — sequence 9 (state 15, landing-skill) and sequence 10 (state 16, cleanup-skill).
- `routing-ledger.jsonl` — 2 new rows: state 15 (`landing` sublane) and state 16 (`cleanup` sublane).

No other ledger files were modified by the landing subagent; verification
rows 6-8 (states 12/13/14) are owned by the prior stages and remain
immutable.

### Production Contract Pin (Provenance)

The runtime contract being pinned by this test-only fix is the production
branch in `crates/vb_storage/src/batch/append_event.rs:61-67`:

```rust
self.journal.events.contains_key(key)?  // returns Err(DuplicateEvent { run: event.run_id(), seq: event.seq() })
```

The variant declaration is at `crates/vb_storage/src/error/mod.rs:30-31`:
`JournalError::DuplicateEvent { run: RunId, seq: EventSeq }`.

The 5 changed test files are the only call-sites in `proptest_vb_*.rs`
that exercise this branch; 4 further call-sites in
`crates/vb_storage/src/batch/t_append_event.rs` and
`crates/vb_storage/src/batch/t_byte_accounting_part{2,3,4}.rs` are
out of scope for this bead (listed in `contract.md` §"Adjacent
Follow-Up Candidates") — those are flagged for sibling audit-regression
beads but not modified here.

End of landing report.
