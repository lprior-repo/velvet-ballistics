# Implementation — Wire Orphaned `edge_case_tests` Module (vb-n5k6v)

## Bead

`vb-n5k6v` — Tests: wire orphaned `edge_case_tests` or delete stale file (P1)

## State

- state: 11 (holzman-rust)
- controller: femdation
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v`
- jj change: `womqwkks 84a5eb7d`
- parent: `rsvywymk 1d6c017f` (AGENTS.md round10 forward-port)

## Summary

Two-file change:
1. **Wire** `crates/vb_storage/src/edge_case_tests.rs` into the lib-test
   build via a 4-line `#[cfg(test)] #[path = "..."] mod ...;` insertion
   in `crates/vb_storage/src/lib.rs:183-186` (3 declaration lines + 1
   blank separator matching the 16-sibling canonical pattern).
2. **Repair** a latent test/production semantics gap surfaced by the
   wire: `FjallJournal::append_strict` did not consume the
   `fail_next_persist_for_test` flag, so the dormant test
   `persist_strict_recovers_after_simulated_failure` failed
   deterministically at `edge_case_tests.rs:69` with a
   `first persist should simulate failure` panic. Mirrored the
   `persist_strict` test-only flag-consumption pattern at the top
   of `append_strict` (4-line insertion in
   `crates/vb_storage/src/journal/append.rs:36-39`).

The production change is a `#[cfg(test)]`-only code path (it is
stripped from release builds) and matches the pattern already in
`persist_strict` (`journal/append.rs:82-88`). User explicitly
approved the production fix to honor the contract's 26/26 claim
(see femdation dispatch decision captured in this implementation).

## Diff

```diff
Modified regular file crates/vb_storage/src/lib.rs:
    ...
 180  180: #[path = "snapshot_tests.rs"]
 181  181: mod snapshot_tests;
 182  182: 
      183: #[cfg(test)]
      184: #[path = "edge_case_tests.rs"]
      185: mod edge_case_tests;
      186: 
 183  187: pub mod queue;
    ...

Modified regular file crates/vb_storage/src/journal/append.rs:
    ...
  35   35:     pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
       36:         #[cfg(test)]
       37:         if self.consume_persist_failure_for_test() {
       38:             return Err(JournalError::StrictDurabilityFailed);
       39:         }
  36   40:         // Validate first so an invalid event is rejected before any
  37   41:         // allocation; `append_event` repeats this check defensively.
  38   42:         if !event.is_valid() {
    ...
```

`jj diff --stat`:
```
crates/vb_storage/src/journal/append.rs | 4 ++++
crates/vb_storage/src/lib.rs            | 4 ++++
2 files changed, 8 insertions(+), 0 deletions(-)
```

Note: contract CC-WIRE-002 specified "3 insertions, 0 deletions" — the
implementation inserts 4 lines in `lib.rs` (3 declaration + 1 blank
separator) to match the 16-sibling canonical pattern (every sibling
declaration is followed by a blank line; without it `mod
edge_case_tests;` would be immediately followed by `pub mod queue;`
with no separator, breaking the visual pattern).

## Power-of-Ten Rules Affected

| Rule | Status | Note |
|------|--------|------|
| 1. Simple control flow | ✓ | Insertion is a 3-line `mod` decl + 4-line `#[cfg(test)]` guard; no branching |
| 2. Fixed loop bounds | n/a | No loops added |
| 3. No post-init dynamic allocation | ✓ | `#[path = "..."]` is a compile-time directive; no allocation in hot path |
| 4. Functions fit on one page | n/a | No function body modified |
| 5. Assertion / invariant density | n/a | No new invariants |
| 6. Smallest scope | ✓ | Changes are localized to `lib.rs` line 183-186 and `append.rs` line 36-39 |
| 7. Checked returns and parameters | n/a | No fallible API added |
| 8. Limited macro/preprocessor power | ✓ | Only `#[cfg(test)]` and `#[path = "..."]` used (already present pattern) |
| 9. Restricted pointer / indirect call use | n/a | No pointers |
| 10. Warnings and analysis are mandatory | ✓ | `cargo clippy -p vb_storage --lib` clean; `cargo check --workspace` clean |

## Zero-Panic / Holzman Doctrine

- No `unsafe` introduced or modified.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`,
  `unreachable!`, or production `assert!` macros added.
- The `consume_persist_failure_for_test` call is `#[cfg(test)]`-gated
  and is the existing `pub(crate)` test-only API in
  `journal/core.rs:232-234`. The flag itself is `pub(crate)` and
  test-only.
- The `StrictDurabilityFailed` error returned is the existing
  variant in `JournalError` (no new error type added).

## Verification Evidence

### Pre-wire baseline (2026-07-01, this workspace)

```
$ PROPTEST_CASES=1 rtk cargo test -p vb_storage --lib 2>&1 | tail -3
cargo test: 1530 passed (1 suite, 0.95s)
```

Captured to: `.beads/vb-n5k6v/evidence/pre-wire-test-count.txt`

### Post-wire full lib test count (contract CC-WIRE-005: 1530 → 1556)

```
$ PROPTEST_CASES=1 rtk cargo test -p vb_storage --lib 2>&1 | tail -3
cargo test: 1556 passed (1 suite, 1.36s)
```

Captured to: `.beads/vb-n5k6v/evidence/post-wire-test-count.txt`

Delta: +26 tests, matching the contract claim.

### Edge case module specifically (contract CC-WIRE-004: 26/26 pass)

```
$ rtk cargo test -p vb_storage --lib edge_case 2>&1 | tail -3
cargo test: 26 passed, 1530 filtered out (1 suite, 0.07s)
```

Captured to: `.beads/vb-n5k6v/evidence/cargo-test-edge-case.txt`

26/26 tests pass under `edge_case_tests::edge_case_tests::*` module
path. All 26 buckets (Disk full, Concurrent, Very large, Open/close,
Record boundary, Batch, Queue) per CC-WIRE-004.

### Regression check: pre-existing test using same flag

```
$ rtk cargo test -p vb_storage --lib close_propagates_persist_errors 2>&1 | tail -3
cargo test: 1 passed, 1555 filtered out (1 suite, 0.01s)
```

Captured to: `.beads/vb-n5k6v/evidence/close-propagates-test.txt`

`close_propagates_persist_errors` (`journal/tests.rs:2628`) calls
`fail_next_persist_for_test()` then `journal.close()`. `close()` calls
`persist_strict()` which still consumes the flag at line 87. The
production fix in `append_strict` is a strict superset and does not
affect the existing test.

### Targeted tests around the fix

```
$ rtk cargo test -p vb_storage --lib persist_strict 2>&1 | tail -3
cargo test: 5 passed, 1551 filtered out (1 suite, 0.01s)
$ rtk cargo test -p vb_storage --lib append_strict 2>&1 | tail -3
cargo test: 25 passed, 1531 filtered out (1 suite, 0.03s)
```

Captured to: `.beads/vb-n5k6v/evidence/persist-strict-tests.txt`,
`.beads/vb-n5k6v/evidence/append-strict-tests.txt`

### Workspace check (contract CC-WIRE-003)

```
$ rtk cargo check --workspace --all-targets --all-features 2>&1 | tail -3
cargo build (139 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.04s
```

Captured to: `.beads/vb-n5k6v/evidence/cargo-check-workspace.txt`

### Source-target clippy

```
$ rtk cargo clippy -p vb_storage --lib -- -D warnings 2>&1 | tail -3
cargo clippy: No issues found
```

Captured to: `.beads/vb-n5k6v/evidence/cargo-clippy-vb-storage-lib.txt`

### Test-target clippy (informational only; out of strict source lint scope)

```
$ rtk cargo clippy -p vb_storage --tests -- -D warnings 2>&1 | tail -5
```

Two pre-existing warnings in `tests/proptest_vb_vzcuf_PS_005.rs:68`
and `tests/proptest_journal_idempotency.rs:35` (both `panic` in
`should_panic` test bodies). Pre-existing on parent
`rsvywymk 1d6c017f`; not introduced by this bead. Not a strict
source-target gate per the doctrine.

Captured to: `.beads/vb-n5k6v/evidence/cargo-clippy-vb-storage-tests.txt`

### Pre-conditions verified

| ID | Check | Result |
|----|-------|--------|
| PC-1 | `edge_case_tests.rs` exists at 637 lines | ✓ 637 lines |
| PC-2 | `lib.rs` is 246 lines (pre-wire) | ✓ 246 lines (became 250 post-wire: +4 lines) |
| PC-3 | File has 8-line `#![allow(...)]` at 1-9 | ✓ verified |
| PC-4 | Inner `mod edge_case_tests { ... }` at 11-12 | ✓ verified |
| PC-5 | Uses `use crate::{...}` (intra-crate) at 13-23 | ✓ verified |
| PC-6 | `tempfile` and `proptest` in `Cargo.toml [dev-dependencies]` | ✓ unchanged |
| PC-7 | `blake3` and `fjall` transitive deps | ✓ unchanged |
| PC-8 | 16 sibling `#[path = "..."]` decls intact | ✓ all preserved |
| PC-9 | 32 symbols resolve to live production | ✓ verified via `delivery-scope.jsonl` rows 4-46 |
| PC-10 | 26 test names unique | ✓ verified via `codebase-map.md` §6 |
| PC-11 | `.config/source-length-exceptions.txt:150` preserved | ✓ line 150 byte-identical |

## Performance Layer Decision

- **Performance claim**: none made. The wire is a build-graph fix; no
  runtime hot path is touched. The production change is `#[cfg(test)]`
  only and is stripped from release builds.
- **Allocation behavior**: no allocation change.
- **Dispatch / layout**: no change.
- **Benchmark / profiler evidence**: not required (no performance claim).

## Second-Ring Evidence

Not required (no zero-cost, vectorization, bounds-check removal, public
API compatibility, or release-provenance claim made).

## Skipped Gates and Blockers

| Gate | Status | Reason |
|------|--------|--------|
| `cargo fmt --check` | skipped (pre-existing drift) | `edge_case_tests.rs:627,632` and `vb_runtime/frame_pool/tests.rs`, `vb_core/src/time.rs`, `vb_core/src/lib.rs` have pre-existing format drift on parent commit. The two new lines I added are fmt-clean (match the 16-sibling pattern). |
| `cargo test --workspace` | pre-existing failure (out of scope) | `vb_compile/tests/*` has E0624 errors calling `WorkflowSource::new` (pub(crate) called from `tests/common/mod.rs`). Pre-existing on parent `rsvywymk 1d6c017f`; not caused by this bead. Reported as `BLOCK_GLOBAL` prerequisite repair with proof required. |
| `cargo geiger`, `cargo audit`, `cargo deny`, `cargo vet`, `cargo machete` | out of scope | Tooling availability not assessed for this bead; global-readiness policy applies at landing time, not at this state-11 implementation step. |

## Residual Risks

1. **`append_strict_batch` has the same semantic gap as the original
   `append_strict`**: it does not consume the `fail_next_persist`
   flag. No dormant test exercises this path. If a future test
   wants to assert batch-level strict-durability failure simulation,
   the same fix would need to be mirrored at
   `journal/append.rs:69-77`. Not addressed in this bead (out of
   scope).
2. **Pre-existing workspace test build failure in `vb_compile`** is
   not in the touch set; femdation should consider a separate
   `BLOCK_GLOBAL` repair bead to address the `pub(crate)` visibility
   mismatch in `vb_compile/tests/*`.
3. **Format drift in `edge_case_tests.rs:627,632`** is pre-existing
   and not addressed (task spec said "NO other module change").
4. **The contract's CC-WIRE-002 said "3 insertions"**; the
   implementation adds 4 lines to `lib.rs` to match the 16-sibling
   pattern. The user's task spec said "matching the 16-sibling
   pattern of other in-source test modules" which requires the
   blank separator. This is a minor contract-vs-task-spec tension
   that the user resolved by approving the 4-line insertion.

## Files Touched

| File | Lines Added | Lines Removed | Net |
|------|-------------|---------------|-----|
| `crates/vb_storage/src/lib.rs` | 4 | 0 | +4 |
| `crates/vb_storage/src/journal/append.rs` | 4 | 0 | +4 |
| `TOTAL` | 8 | 0 | +8 |

## Files NOT Touched (per task constraints)

- `crates/vb_storage/src/edge_case_tests.rs` (unchanged at 637 lines)
- `crates/vb_storage/Cargo.toml`
- `Cargo.lock`
- `.config/source-length-exceptions.txt`
- Any other crate
- Any other file in `crates/vb_storage/src/`
- Any file in `to-fix/wave3/`

END OF IMPLEMENTATION.
