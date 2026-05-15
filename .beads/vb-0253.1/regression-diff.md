# regression-diff.md — vb-0253.1

## Summary

No new failures introduced by this bead. All 85 failing tests are pre-existing, documented in `baseline-report.md`.

---

## New Failures: NONE

All bead-specific tests (8 tests across 5 obligations) pass.

---

## Pre-existing Failures: 85 tests

- Documented in: `baseline-report.md`, `STATE.md state_10_evidence`
- Category: pre-existing, unrelated to `vb-0253.1`
- Not introduced by `ShardCommandQueue` implementation

---

## Classification

- **BLOCK_LOCAL**: N/A — no new failures
- **BLOCK_REGRESSION**: N/A — no regression
- **BLOCK_RELEASE**: N/A
- **REQUIRED_OBLIGATION_FAIL**: N/A — all required obligations either PASS or WAIVED
- **DEFERRED_GLOBAL**: 85 pre-existing failures (unrelated to this bead; documented)

---

## Blocking Status

**NOT BLOCKING** — all bead obligations passed or justifiably waived.
