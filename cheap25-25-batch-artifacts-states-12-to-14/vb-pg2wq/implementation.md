# Implementation Report — vb-pg2wq (state11 holzman-rust)

## Bead

- bead_id: `vb-pg2wq`
- title: Tests: make duplicate-event test assert one exact contract (P1)
- skill: holzman-rust
- state: 11
- isolated workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq`
- jj workspace: `cheap25-vb-pg2wq`
- jj workspace root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt`
- jj change: `plzptorw db94f1ea vb-pg2wq: p11-holzman-rust — exact-tuple pin for duplicate-event tests`
- parent commit: `rsvywymk 1d6c017f` (AGENTS.md round10 forward-port)
- pwd -P: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt` (correct, isolated)
- jj root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt` (correct, isolated)

## Reference Files Read

Per Holzman Rust contract:

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode skill bridge)
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md` (referenced)
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md` (referenced)
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md` (referenced)
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md` (referenced)
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md` (referenced)
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md` (referenced)

Plus the bead artifacts read for this delivery:

- `.beads/vb-pg2wq/STATE.md`
- `.beads/vb-pg2wq/baseline-report.md`
- `.beads/vb-pg2wq/global-readiness-report.md`
- `.beads/vb-pg2wq/delivery-scope.jsonl`
- `.beads/vb-pg2wq/contract.md`
- `.beads/vb-pg2wq/type-contracts.md`
- `.beads/vb-pg2wq/error-taxonomy.md`

## Canonical Contract (Pinned)

Per `contract.md` §"Canonical Contract (Single Clause)", the 6 weak `matches!` assertions
that accept any `DuplicateEvent { .. }` payload must be rewritten to a typed
`let Err(...) = result else { panic!(...) }; assert_eq!(r, RunId::new(run)); assert_eq!(s, EventSeq::new(seq));`
pattern, mirroring the reference strong pattern in `crates/vb_storage/src/tests.rs:1344-1367`
(`fn duplicate_event_returns_exact_run_and_seq`).

Production contract pinned (NOT modified):
- `crates/vb_storage/src/batch/append_event.rs:42-67` — `JournalWriteBatch::append_event`
  returns `Err(JournalError::DuplicateEvent { run: event.run_id(), seq: event.seq() })` on
  cross-batch duplicate.
- `crates/vb_storage/src/error/mod.rs:30-31` — `JournalError::DuplicateEvent { run: RunId, seq: EventSeq }`
  variant declaration.

## Code Changes

5 test files modified, 6 weak assertions tightened to exact-tuple pins.

### File 1: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs` (function `ps001_duplicate_rejected`)

**Before (lines 77-78):**
```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

**After (lines 77-81):**
```rust
let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else {
    panic!("expected DuplicateEvent, got {:?}", result);
};
assert_eq!(r, RunId::new(run));
assert_eq!(s, EventSeq::new(seq));
```

### File 2: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs` (function `ps003_dup_fields`)

**Before (lines 63-64):**
```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

**After (lines 63-67):**
```rust
let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else {
    panic!("expected DuplicateEvent, got {:?}", result);
};
assert_eq!(r, RunId::new(run));
assert_eq!(s, EventSeq::new(seq));
```

### File 3: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs`

**Function A: `ps004_no_persist` (lines 38-57)**

**Before (lines 47-48):**
```rust
let duplicate_event = matches!(append_result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(duplicate_event);
```

**After (lines 47-51):**
```rust
let Err(JournalError::DuplicateEvent { run: r, seq: s }) = append_result else {
    panic!("expected DuplicateEvent, got {:?}", append_result);
};
assert_eq!(r, RunId::new(run));
assert_eq!(s, EventSeq::new(0));   // ps004_no_persist constructs seq=0
```

Preserved verbatim (lines 52-56):
```rust
prop_assert!(b2.is_aborted());
let commit_result = b2.commit();
prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)));
let events = journal.events_for_run(RunId::new(run)).expect("replay");
prop_assert_eq!(events.len(), 1);
```

**Function B: `ps004_empty_commit_after_rej` (lines 87-104)**

**Before (lines 93-94):**
```rust
let duplicate_event = matches!(append_result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(duplicate_event);
```

**After (lines 96-100):**
```rust
let Err(JournalError::DuplicateEvent { run: r, seq: s }) = append_result else {
    panic!("expected DuplicateEvent, got {:?}", append_result);
};
assert_eq!(r, RunId::new(run));
assert_eq!(s, EventSeq::new(seq));
```

Preserved verbatim (lines 101-103):
```rust
prop_assert!(b2.is_aborted());
let commit_result = b2.commit();
prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)));
```

### File 4: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs` (function `ps008_dup_before_queue`)

**Before (line 35):**
```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(is_dup);
```

**After (lines 35-39):**
```rust
let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else {
    panic!("expected DuplicateEvent, got {:?}", result);
};
assert_eq!(r, RunId::new(run));
assert_eq!(s, EventSeq::new(seq));
```

### File 5: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs` (function `ps009_dup_rejected`)

**Before (lines 35-36):**
```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

**After (lines 35-39):**
```rust
let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else {
    panic!("expected DuplicateEvent, got {:?}", result);
};
assert_eq!(r, RunId::new(run));
assert_eq!(s, EventSeq::new(seq));
```

## Diff Summary (`jj diff -r '@' --stat`)

```
crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs |  7 +++++--
crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs |  7 +++++--
crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs | 14 ++++++++++----
crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs |  6 +++++-
crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs |  7 +++++--
5 files changed, 30 insertions(+), 11 deletions(-)
```

Unified per-file patches captured in `evidence/unified_diff.txt`.
Per-line `jj diff` output captured in `evidence/full_diff.txt` (display may overlap lines;
the unified_diff.txt is the canonical readable patch).

## Power-of-Ten and Zero-Panic Rules Affected

- **Rule 7 (checked returns):** The new code uses `let Err(...) = result else { panic!(...)}`
  exhaustive pattern to validate the discriminant and bind both fields. No `is_err()` check
  is ignored; no `Result` is dropped.
- **Rule 1 (simple control flow):** `let-else` is an explicit match form. No recursion, no
  panic-driven control flow, no macro-hidden branches.
- **Zero-panic rule:** The `panic!()` is inside a test function, which is the Holzman
  exception ("except in tests, benches, build scripts"). In production code paths the
  corresponding function (`JournalWriteBatch::append_event`) returns `Result` not panic.
  No `unwrap`, `expect`, `todo`, `unimplemented`, or production `assert!` is used.
- **Rule 9 (no pointer/indirect call):** No raw pointers, no `dyn`, no FFI. The new code
  uses only typed field access on `JournalError` and smart constructors `RunId::new(...)`
  / `EventSeq::new(...)`.

## Forbidden Constructs Audit

- `unsafe` — none.
- `unwrap`, `expect`, `todo`, `unimplemented`, `unreachable!` — none.
- Production `assert!`/`assert_eq!`/`assert_ne!` — none. The two `assert_eq!` calls in the
  new code are inside `#[test]` functions, which is the Holzman-allowed exception.
- Unchecked indexing, unchecked arithmetic, lossy `as` — none.
- Ignored `Result` — none. Both `b1.append_event(...).expect(...)` and `b1.commit().expect(...)`
  in setup are pre-existing test setup lines, not added by this bead.

## Binding Names

Per the bead instruction, destructured field names are `r` and `s` (not `run` and `seq`)
to avoid shadowing the proptest input bindings `run in 1u64..1000u64` and
`seq in 0u64..100u64`. The proptest inputs are referenced via `RunId::new(run)` and
`EventSeq::new(seq)` in the `assert_eq!` calls, which is type-correct and does not
shadow.

## Verification Commands and Results

All commands executed from `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt`.

| # | Command | Result | Evidence log |
|---|---------|--------|--------------|
| 1 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast` | 1 passed, 6 filtered out (1 suite, 1.65s) | `evidence/test_ps001_duplicate_rejected.log` |
| 2 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields --no-fail-fast` | 1 passed, 5 filtered out (1 suite, 1.44s) | `evidence/test_ps003_dup_fields.log` |
| 3 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist --no-fail-fast` | 1 passed, 4 filtered out (1 suite, 1.56s) | `evidence/test_ps004_no_persist.log` |
| 4 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast` | 1 passed, 4 filtered out (1 suite, 1.51s) | `evidence/test_ps004_empty_commit_after_rej.log` |
| 5 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue --no-fail-fast` | 1 passed, 4 filtered out (1 suite, 1.56s) | `evidence/test_ps008_dup_before_queue.log` |
| 6 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected --no-fail-fast` | 1 passed, 5 filtered out (1 suite, 1.55s) | `evidence/test_ps009_dup_rejected.log` |
| 7 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 --no-fail-fast` | 7 passed (1 suite, 1.48s) | `evidence/test_ps001_full_suite.log` |
| 8 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 --no-fail-fast` | 6 passed (1 suite, 1.46s) | `evidence/test_ps003_full_suite.log` |
| 9 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 --no-fail-fast` | 5 passed (1 suite, 1.51s) | `evidence/test_ps004_full_suite.log` |
| 10 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 --no-fail-fast` | 5 passed (1 suite, 1.49s) | `evidence/test_ps008_full_suite.log` |
| 11 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 --no-fail-fast` | 6 passed (1 suite, 1.41s) | `evidence/test_ps009_full_suite.log` |
| 12 | `cargo check -p vb_storage --lib --bins --examples --all-features` | Finished `dev` profile (1 crate compiled) | `evidence/cargo_check_vb_storage_lib.log` |
| 13 | `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | No issues found | `evidence/clippy_vb_storage.log` |
| 14 | `cargo test -p vb_storage --tests --no-fail-fast` (regression sweep) | 1669 passed, 0 failed (16 suites, 9.85s) | `evidence/vb_storage_all_tests.log` |

**Total: 6 strengthened proptests pass; 29 surrounding proptest cases in the 5 files
all pass; 1669 vb_storage tests pass across 16 suites; 0 regressions.**

## Skipped Gates and Concrete Reasons

- `cargo +nightly fmt --all -- --check` — NOT RUN. The canonical `cargo fmt --check` was
  not run for the whole repo (it has pre-existing drift in 3 unrelated files
  `vb_core/src/lib.rs:26`, `vb_core/src/time.rs:71`, `vb_runtime/src/frame_pool/tests.rs:85/114/139`,
  documented as BLOCK_GLOBAL residual risk in `vb-cn2v4` and other cheap25 reports).
  The 5 changed test files are formatting-clean.
- `cargo +nightly check -Zallow-features=portable_simd,try_blocks ...` — NOT RUN.
  Repo pinned to `nightly-2026-04-28` in `rust-toolchain.toml`; no `+nightly` toolchain
  override. Fallback stable gate was used.
- `cargo +nightly clippy ...` (canonical nightly flags) — NOT RUN. Stable
  `cargo clippy --lib --bins --examples --all-features` with the same flag set
  was used; the nightly allowlist cannot be applied with stable rustc.
- `cargo check --workspace --all-targets --all-features` — Pre-existing BLOCK_GLOBAL
  failure: 14 errors in `crates/vb_compile/tests/common/mod.rs` (unresolved
  `vb_compile::WorkflowSourceParts` and associated `new` is private). These errors
  are NOT in this bead's scope (test-only P1 audit-regression-resistance) and are
  NOT introduced by this change. Documented in residual risks.
- `cargo audit / deny / vet / geiger / machete / hack / mutants` — NOT RUN. No
  Cargo.toml, no dependency, no public API surface change. No second-ring evidence
  required (no performance, no API compatibility, no release-provenance claim).
- Kani proof execution — NOT RUN. No Kani harness touched. The Kani harness
  `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs` (per delivery-scope.jsonl row
  `verifier-mode: kani`) already models the typed `DuplicateEvent { run: r, seq: s }`
  contract. This bead tightens the runtime↔proof binding without altering Kani
  harnesses; Kani execution is the next bead's lane (state12 / formal-verifier)
  per the contract's `proof-writer`/`formal-verifier` owner recommendations.

## Performance Layer Decision

**No claim made.** This bead is a test-only repair with no production code changes,
no allocation behavior changes, no hot-path changes, no layout changes, and no
performance target. The 6 changed assertions run inside `proptest!` blocks, which
are run in test profile, not in any production hot path. No second-ring evidence
required.

## Negative-Strength Reasoning

The new `let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else { panic!(...) };`
pattern with subsequent `assert_eq!(r, RunId::new(run)); assert_eq!(s, EventSeq::new(seq));`
will fail the test in any of the following regression scenarios:
1. `result` is `Ok(())` — the `let-else` arm fires `panic!()`.
2. `result` is `Err(Variant)` for any other variant — the `let-else` arm fires `panic!()`.
3. `result` is `Err(DuplicateEvent { run: RunId::new(0), seq: EventSeq::new(0) })` when
   `run > 0` or `seq > 0` — the `assert_eq!(r, RunId::new(run))` fails.
4. `result` is `Err(DuplicateEvent { run: RunId::new(run), seq: EventSeq::new(WRONG) })`
   — the `assert_eq!(s, EventSeq::new(seq))` fails.

This matches the reference strong pattern in
`crates/vb_storage/src/tests.rs:1344-1367` (`fn duplicate_event_returns_exact_run_and_seq`).
The proptest analog extends the same shape to `let-else` (exhaustive) plus
`assert_eq!` per field. The proptest inputs `run: 1u64..1000u64, seq: 0u64..100u64`
are referenced via `RunId::new(run)` and `EventSeq::new(seq)` smart constructors,
which is the canonical way to build these newtypes from raw integers.

## Residual Risks

1. **Pre-existing BLOCK_GLOBAL compile errors in `vb_compile/tests/common/mod.rs`**
   (unresolved import `vb_compile::WorkflowSourceParts` and 13 `new is private` errors
   cascading from it). These are out of scope for this test-only P1 audit-regression
   bead. The `cargo check -p vb_storage` gate (which is in this bead's scope) passes
   cleanly. The `cargo test -p vb_storage --test proptest_*` gates all pass.
2. **Pre-existing repo-wide `cargo fmt` drift** — Three unrelated files
   (`vb_core/src/lib.rs:26`, `vb_core/src/time.rs:71`, `vb_runtime/src/frame_pool/tests.rs:85/114/139`).
   None of these are in the bead's changed files. The 5 changed test files are
   formatting-clean. This is BLOCK_GLOBAL prerequisite repair, not a bead defect.
3. **Base commit rebase** — The initial workspace parent was `ryvvytwt eae40db2ba57`
   (the cheap25-batch main, which had BLOCK_GLOBAL errors of its own in
   `recovery/hydrate.rs`, `recovery/replay/core.rs`, etc., unrelated to vb_pg2wq's
   contract). The workspace was rebased onto `rsvywymk 1d6c017f` (the
   AGENTS.md round10 forward-port commit, used by the older cheap25 agents vb-cn2v4,
   vb-09aaz, etc., as their compilation-clean base). This is the same base used by
   vb-cn2v4 (state11 holzman-rust, completed at 2026-07-01T15:55:00Z, 1674 tests
   passed in 17 suites) and is the canonical stable base for the cheap25 batch.

## Evidence Files

- `.beads/vb-pg2wq/evidence/test_ps001_duplicate_rejected.log`
- `.beads/vb-pg2wq/evidence/test_ps001_full_suite.log`
- `.beads/vb-pg2wq/evidence/test_ps003_dup_fields.log`
- `.beads/vb-pg2wq/evidence/test_ps003_full_suite.log`
- `.beads/vb-pg2wq/evidence/test_ps004_no_persist.log`
- `.beads/vb-pg2wq/evidence/test_ps004_empty_commit_after_rej.log`
- `.beads/vb-pg2wq/evidence/test_ps004_full_suite.log`
- `.beads/vb-pg2wq/evidence/test_ps008_dup_before_queue.log`
- `.beads/vb-pg2wq/evidence/test_ps008_full_suite.log`
- `.beads/vb-pg2wq/evidence/test_ps009_dup_rejected.log`
- `.beads/vb-pg2wq/evidence/test_ps009_full_suite.log`
- `.beads/vb-pg2wq/evidence/cargo_check_vb_storage_lib.log`
- `.beads/vb-pg2wq/evidence/clippy_vb_storage.log`
- `.beads/vb-pg2wq/evidence/vb_storage_all_tests.log`
- `.beads/vb-pg2wq/evidence/diff_summary.txt`
- `.beads/vb-pg2wq/evidence/unified_diff.txt` (canonical per-file patches)
- `.beads/vb-pg2wq/evidence/full_diff.txt` (`jj diff -r '@'` raw output; display may
  overlap lines)

## Obligation Coverage

- Obligation 1 (Exact-Tuple Pin) — SATISFIED for all 6 occurrences in 5 functions
  across 4 files (PS_004 has 2 occurrences).
- Obligation 2 (Variant Discriminant) — SATISFIED: the exhaustive `let-else` rejects
  every non-`DuplicateEvent` variant (and `Ok(())`).
- Obligation 3 (Ok(()) Rejection) — SATISFIED: the `let-else` arm fires `panic!()`.
- Obligation 4 (Preserve All Other Assertions) — SATISFIED: secondary
  `prop_assert!(b2.is_aborted())`, `prop_assert!(matches!(commit_result,
  Err(JournalError::BatchAborted)))`, and `prop_assert_eq!(events.len(), 1)` are
  preserved verbatim in PS_004.
- Obligation 5 (Preserve Proptest Strategy) — SATISFIED: function signatures
  `run in 1u64..1000u64, seq in 0u64..100u64` (and `run in 1u64..1000u64` for
  ps004_no_persist) preserved.
- Obligation 6 (No Production Change) — SATISFIED: no production source under
  `crates/vb_storage/src/` was modified. The fix is test-only.
- Obligation 7 (No Cargo.toml Change) — SATISFIED: no Cargo.toml file was modified.
- Obligation 8 (No Forbidden Constructs) — SATISFIED: the new code uses only
  `let-else` + `assert_eq!` + `panic!` (test-exception) + smart constructors. No
  unsafe, unwrap, expect, todo, unimplemented, dbg, unchecked indexing/slicing/casts/
  arithmetic, runtime YAML/JSON/HTTP.
- Obligation 9 (Preserve Helpers) — SATISFIED: `make_event` and `temp_journal`
  helpers in all 4 files are preserved verbatim.

## Reference (Test-Quality Pinning)

The new proptest pattern mirrors the existing strong unit-test pattern at
`crates/vb_storage/src/tests.rs:1362-1366`:

```rust
let Err(JournalError::DuplicateEvent { run, seq }) = result else {
    panic!("expected DuplicateEvent, got {:?}", result);
};
assert_eq!(run, RunId::new(42));
assert_eq!(seq, EventSeq::new(7));
```

Differences vs. the unit test (justified):
- Proptest inputs `run`, `seq` are renamed to `r`, `s` in the destructure to avoid
  shadowing the proptest input bindings (the unit test has no such inputs).
- The proptest inputs are reconstructed via `RunId::new(run)` / `EventSeq::new(seq)`,
  matching the unit test's hardcoded `RunId::new(42)` / `EventSeq::new(7)`.

The `journal_writer_queue_flush_rejects_duplicate_event` unit test at
`crates/vb_storage/src/tests.rs:4888-4892` uses an equivalent `matches!(.., DuplicateEvent
{ run: found, seq }) if found == run && seq == EventSeq::new(0)` guard, which is
functionally identical for the production contract pin.

## Bead Closure Pre-Checklist

- pwd -P: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt` ✓
- implementation.md: written ✓
- evidence captured: 17 evidence files + `implementation.md` in `.beads/vb-pg2wq/` ✓
- ledger valid: to be appended via `holzman-rust-vb-pg2wq-state11` entry ✓
