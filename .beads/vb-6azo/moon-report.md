# Moon CI Gate Report

**Bead:** vb-6azo
**Workspace:** /home/lewis/src/Velvet-ballistics/vb-6azo-ws (does not exist - ran from main workspace)
**Date:** 2026-05-09

## Commands Run

### 1. `moon run :quick`
**Status:** PASS
**Exit Code:** 0
**Notes:** Installed rust nightly-2026-04-28

### 2. `moon run :test`
**Status:** PASS
**Exit Code:** 0
**Summary:** 10301 tests run, 10301 passed, 0 skipped
**Duration:** ~7m 35s

### 3. `moon ci`
**Status:** TIMEOUT
**Exit Code:** N/A (command exceeded 600000ms timeout)
**Notes:**
- `moon ci` is the built-in moon command (not a task) that runs all affected tasks in CI
- Was running: `velvet-ballastics:miri`, `velvet-ballastics:feature-powerset`, `velvet-ballastics:bench-build`, `velvet-ballastics:doc`
- Miri tests were still executing after 3+ minutes when output was truncated
- The miri task alone was expected to take ~348s based on output

## Failure Details

| Gate | Pass/Fail | Notes |
|------|-----------|-------|
| :quick | PASS | |
| :test | PASS | All 10301 tests passed |
| :ci | TIMEOUT | Exceeded 10 minute timeout |

## Notes

- The bead workspace `vb-6azo-ws` does not exist at `/home/lewis/src/Velvet-ballistics/vb-6azo-ws`
- Gates were executed against the main velvet-ballistics workspace instead
- `moon ci` runs multiple heavy tasks (miri, feature-powerset, bench-build, doc) and would require longer timeout to complete
