# State 11: Regression Diff

## Baseline (pre-bead)
- moon ci: 4 failed, 6 completed, 11 skipped
- Failed: fuzz-smoke, fmt, lint-src, check
- Passed: beads-server-mode, workspace-assertions, agent-cli-contract, lint-src (lint), nightly-feature-gate, fmt (lint), source-length, miri
- Skipped: test, coverage, hardened-build, mutants-smoke, bench-build, feature-powerset, doc-test, maxperf, maxperf-native, nightly-feature-cargo-probe, doc

## Current (post-bead)
- moon ci: 6 failed, 5 completed, 11 skipped
- Failed: fuzz-smoke, fmt, lint-src, check, **miri (NEW)**, workspace-assertions (NEW)
- Passed: beads-server-mode, agent-cli-contract, nightly-feature-gate, fmt (lint), source-length
- Skipped: test, coverage, hardened-build, mutants-smoke, bench-build, feature-powerset, doc-test, maxperf, maxperf-native, nightly-feature-cargo-probe, doc

## Changes

### moon ci: +2 new failures (BLOCK_REGRESSION)

| Gate | Baseline | Current | Change |
|------|----------|---------|--------|
| miri | PASS | **FAIL** | NEW — non-exhaustive match on ValidationError |
| workspace-assertions | PASS | **FAIL** | NEW — assertion failure (verify if pre-existing or new) |

### Test counts: +44 new tests

| Category | Baseline | Current | Delta |
|----------|----------|---------|-------|
| Production binding | 0 | 55 passed | +55 (NEW) |
| Proptest properties | 0 | 17 passed | +17 (NEW) |
| Integration tests | 0 | 8 passed, 22 failed | +30 (NEW) |
| Kani harnesses | 0 | 0 found | +9 written, 0 discovered |
| **Total** | **0** | **56 pass, 22 fail** | **+78** |

### Compilation errors: +0 new (all pre-existing)
- cargo check: 8 errors (all in xtask/src/shell.rs, pre-existing)
- No new compilation errors introduced by contracts.rs

### Clippy errors: +15 new (BLOCK_LOCAL)
- contracts.rs: 6 arithmetic side-effects + 6 indexing panics + 1 as cast + 3 unused vars + 2 unused values + 2 suggestions = 20 warnings (3 are errors with -D)
- shell.rs: 5 pre-existing unused functions

## Verdict

**This bead introduced 2 new moon ci failures and 22 integration test failures.**

The BLOCK_REGRESSION (miri gate) is caused by adding 3 new ValidationError variants to vb_validate without updating the exhaustive match arms in diag_render.rs and diagnostic.rs. This is a simple fix: add 3 missing match arms.

The 22 integration test failures are caused by `discover_contracts()` returning 0 files in temp directory context — a path resolution bug in `collect_cue_files()`.

The clippy errors in contracts.rs are quality issues in new code (BLOCK_LOCAL).

The Verus spec compilation failures are type errors in the spec file (BLOCK_LOCAL).

**Net assessment: This bead is NOT ready to land due to BLOCK_REGRESSION on miri gate.**
