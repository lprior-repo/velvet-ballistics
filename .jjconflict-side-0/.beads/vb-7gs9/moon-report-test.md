# Moon Test Gate Report - vb-7gs9

**Date:** 2026-05-09
**Command:** `moon run :test`
**Result:** FAILED (timeout + process failure)

## Exit Code
Non-zero (process failed / timed out after 300s)

## Summary

The `moon run :test` gate **failed** with two distinct issues:

### 1. Timeout
The command exceeded the 300 second (5 minute) timeout and was terminated.

### 2. Process Failure in `nightly-feature-gate`
```
Error: process::failed
  × Process git failed: terminated
```

The `nightly-feature-gate` task failed because a git process was terminated unexpectedly, printing a large list of git hashes (likely from a git log or diff operation that got killed).

## Key Observations

- `supply-chain` succeeded (vetting passed, unsafe code metrics generated)
- Multiple "Failed to parse file" warnings in `.claude/worktrees/*` directories - these are from other agent worktrees and do not affect the main build
- `nightly-feature-gate` task failed with git process termination

## Failure Category
`process_failure` (git process terminated during nightly-feature-gate)
