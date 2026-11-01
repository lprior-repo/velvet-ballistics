# Contract — vb-0p2nl: Collapse owned-continuation vs transport test-suite clone cluster

## Scope

Consolidate the near-clone shared fixtures of exactly two suites:

- `crates/workspace_tests/tests/vb_f1_owned_continuation/` (8 modules, 21 tests)
- `crates/workspace_tests/tests/vb_f1_owned_continuation_transport/` (12 modules, 19 tests)

into one canonical shared module set. Total behavior coverage before: 21 + 19 = 40 tests,
plus 19 of those re-run nested inside `vb_f1_owned_continuation_carrier` (41 tests there).
After: identical counts.

## Out of scope (explicitly NOT consolidated)

- Test bodies are distinct coverage, not clones: continuation tests exercise
  `RunContinuationV1::capture` / `from_bytes`; transport tests exercise
  `capture_transport` / `from_transport_bytes`, hostile wire layouts, golden wire
  comparisons, store-bound preflights, and action-guard fixtures. Merging or
  deleting any of them would destroy coverage.
- `vb_f1_owned_continuation_carrier` reuses the transport entry file as
  `pub mod component_fixture` (for `carrier_fixture`). Both entry files
  (`vb_f1_owned_continuation.rs`, `vb_f1_owned_continuation_transport.rs`) stay;
  only the four shared support modules collapse to one copy. Carrier nesting
  (19 nested tests) is unchanged.

## Clone inventory (measured, not estimated)

| File pair | Status |
|---|---|
| `evidence.rs` (96 lines each) | byte-identical |
| `support.rs` (184 vs 196 lines) | transport = continuation + `fixture_source_with_blob` + `at_cut_with_blob` (18 diff lines) |
| `arenas.rs` (42 vs 45 lines) | transport generalizes `original` to `original_with_blob`; `bundle` identical |
| `receipt.rs` (88 vs 100 lines) | transport factors `persist` through `persist_owned` with contract param; `load` identical |

## Design (scott-ddd-refactor: single parse boundary, trusted types downstream)

New `crates/workspace_tests/tests/vb_f1_owned_continuation_common/` holds the four
canonical modules. Both suite entries load them via `#[path]`; all test bodies keep
`use super::support::...` unchanged — zero test-body edits, so every assertion
survives textually.

Canonical semantics (superset, continuation-based defaults + transport extras):

- `arenas::{original, original_with_blob, bundle}` — `original` keeps exact
  `vec![3u8, 1, 4]` bytes; `original_with_blob(n)` keeps `vec![3u8; n]`.
- `support::{fixture, fixture_source, fixture_source_with_blob, at_cut,
  at_cut_with_blob, drive, expected, ...}` — `fixture_source` keeps continuation
  path (`arenas::original`).
- `receipt::{persist, persist_owned, Receipt}` — `persist` delegates to
  `persist_owned(..., &[])`, identical call to today.
- `evidence::assert_pages_and_order` — identical copy.

One normalized input detail: transport ordinary-path fixtures (`support::at_cut`,
12 call sites) previously built the seed blob as `[3, 3, 3]`
(`original_with_blob(w, 3)`); after consolidation they build `[3, 1, 4]`
(`arenas::original`). No assertion in either suite depends on blob bytes
(verified by grep: only byte-literal is the constructor itself; digests are
computed, never hardcoded; golden/hostile/resume comparisons are
self-consistent on one fixture). Explicit-blob paths (`at_cut_with_blob`,
`action_fixture`, carrier `carrier_fixture`) are byte-untouched.

## Acceptance

- `cargo test -p velvet-ballistics-workspace-tests --test vb_f1_owned_continuation`
  → 21 passed (before: 21).
- `... --test vb_f1_owned_continuation_transport` → 19 passed (before: 19).
- `... --test vb_f1_owned_continuation_carrier` → 41 passed (before: 41).
- Full `workspace_tests` package suite green; `cargo check --workspace
  --all-targets --all-features`, clippy zero-issue, `cargo fmt --check` clean.
- No `unwrap`/`expect`/`panic` introduced (none present in shared helpers today;
  `assert_*`/`ok_or` style preserved as-is — test-only code, Holzman-clean).
- Net line delta negative (~−450 lines of duplication removed).
