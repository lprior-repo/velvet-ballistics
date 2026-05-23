# Black-Hat Review: vb-jpq7.3

Verdict: REJECT

## Findings

### P0 — Global readiness is false: canonical `moon ci` fails

- Evidence command: `moon ci`
- Evidence output path: `/home/lewis/.local/share/opencode/tool-output/tool_e53c93f080011OXtUZqF4iQWUL`
- Result: 2 failed tasks, 12 skipped.
- Contract: `/velvet-ballistics-MASTER.md` lines 45-60, 82-97 and AGENTS.md "Build And CI" make first-party zero-panic/zero-warning gates and `moon ci` canonical.

Blockers observed in the run:

1. `velvet-ballastics:panic-surface` fails on production panic surface:
   - `crates/vb_codegen/src/parity.rs:438`
   - `crates/vb_codegen/src/parity.rs:444`
   - Both use `unreachable!(...)`, violating the repository no-panic rule for production code.
2. `velvet-ballastics:check` fails with `-D warnings` dead-code errors in workspace tests:
   - `crates/workspace_tests/tests/vb_test_compile_error_quality_behavior.rs:33`
   - `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs:53`
   - `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs:127`
   - `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs:231`

No P0 release-blocker bead gets approved while canonical CI is red.

### P1 — Readiness artifact is stale and contradicts live evidence

- `.beads/vb-jpq7.3/global-readiness-report.md:5-18` claims global landing is blocked by `cargo fmt --all -- --check` and points to old format output.
- Live review command `rustup run nightly-2026-04-28 cargo fmt --all -- --check` produced no output and succeeded.
- The actual blocker is now `moon ci` panic/check failure, not formatting. The bead evidence pack is therefore not a trustworthy readiness record.

## Scoped Contract Checks That Did Pass In Review

- `rustup run nightly-2026-04-28 cargo fmt --all -- --check`: PASS.
- `bash scripts/check-ignored-fallible-results.sh`: PASS / `NoViolationFound`.
- Source inspection confirmed the original scoped defects are addressed in the touched storage paths:
  - `crates/vb_storage/src/journal/replay.rs:24` propagates snapshot lookup with `?`, not `.ok().flatten().unwrap_or(...)`.
  - `crates/vb_storage/src/journal/replay.rs:19-31` exposes bounded replay and default delegation.
  - `crates/vb_storage/src/journal/replay.rs:51-63` starts scanning at the tail key and enforces sequence/limit checks.
  - `crates/vb_storage/src/trimming/logic.rs:34-48` decodes snapshot payload and verifies run/seq before trusting key authority.
  - `crates/vb_storage/src/recovery/hydrate_support.rs:209-214` fails closed on non-uninitialized taint read failures.
  - `crates/vb_storage/src/journal/append.rs:27-33` and `crates/vb_storage/src/journal/core.rs:150-152` expose explicit strict persist/close result paths.

## Mandated Fixes Before Approval

1. Remove/replace production `unreachable!(...)` in `crates/vb_codegen/src/parity.rs:438` and `:444` with typed error flow.
2. Repair the `velvet-ballastics-workspace-tests` dead-code failures reported by `moon ci`.
3. Rerun `moon ci` and record the passing evidence in `.beads/vb-jpq7.3/verification-ledger.jsonl` / readiness artifact.
4. Update `.beads/vb-jpq7.3/global-readiness-report.md` so it reflects live blockers instead of stale fmt failure.

## Residual Risks

- The scoped storage/recovery fix looks directionally correct, but no approval is possible while global zero-panic and compile gates fail.
- The evidence pack currently overstates readiness because it does not include the live `moon ci` failure.
