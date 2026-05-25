# Baseline Report: vb-jpq7.3

## Bead

- `bd show vb-jpq7.3`: `IN_PROGRESS`, P0 bug, assignee Lewis.
- Acceptance criteria: typed propagation for storage/recovery errors, taint read failures fail closed, bounded replay, explicit shutdown durability `Result`, and no silent fallible-result discard.

## Starting Issue Evidence

The bead description cited these source defects:

- `journal/replay.rs` erased `latest_durable_snapshot_seq` errors via `.ok().flatten().unwrap_or(EventSeq::new(0))`.
- `events_for_run` collected an unbounded `Vec` and scanned/decode-skipped pre-snapshot events.
- `hydrate_support.rs` defaulted failed `read_taint` to `Taint::Clean`.
- `FjallJournal::Drop` discarded `persist(SyncAll)` errors.

## Current Baseline Commands

- `/usr/bin/git status --short --branch` -> branch `main...origin/main [ahead 1]`, scoped Rust/test edits plus unrelated rustfmt blocker files restored out of working tree.
- `bd show vb-jpq7.3` -> `IN_PROGRESS`.
- `bd show vb-llab` -> `IN_PROGRESS`, discovered compile blocker for action queue; current branch already contains the compile fix used by the scoped runtime test.

## Global Readiness Baseline

- `rustup run nightly-2026-04-28 cargo fmt --all -- --check` -> PASS on live rerun.
- `moon ci` -> FAIL/BLOCK_GLOBAL. Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e53cb9935001x2youOsXWkFzMl`.
- `moon ci` failed `velvet-ballistics:panic-surface` on production `unreachable!(...)` in `crates/vb_codegen/src/parity.rs:438` and `:444`.
- `moon ci` failed `velvet-ballistics:check` on `-D warnings` dead-code errors in unrelated workspace tests: `vb_test_compile_error_quality_behavior.rs:33`, `vb_test_runtime_lifecycle_state_behavior.rs:53`, `:127`, and `:231`.

## Regression Classification

- Local gates listed in `verification-ledger.jsonl` pass for touched crates and behavior tests.
- Canonical `moon ci` remains `BLOCK_GLOBAL`; it is not evidence of scoped vb-jpq7.3 behavior failure, but it blocks final landing until repaired under one or more explicit prerequisite beads or explicitly waived by the release owner.
