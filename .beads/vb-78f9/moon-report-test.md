# Moon Test Report - vb-78f9

**Date:** 2026-05-09
**Command:** `moon run :test`
**Result:** FAILED (timeout)

## Summary

The `moon run :test` command failed due to a timeout during the `nightly-feature-gate` task.

## Tasks Executed

1. **installing rust nightly-2026-04-28** - Completed
2. **velvet-ballistics:agent-cli-contract** - Cached (277050d8)
3. **velvet-ballistics:supply-chain** - Completed (1m 56s, 69222e5f)
   - Vetting Succeeded (403 exempted)
   - Advisories OK, bans OK, licenses OK, sources OK
   - Unsafe code metrics collected
4. **velvet-ballistics:nightly-feature-gate** - FAILED

## Failure Details

**Error:** `Process git failed: terminated`

The `nightly-feature-gate` task runs git operations to check for nightly feature usage. The git process was terminated, likely due to a timeout (the command exceeded the 5-minute timeout limit).

## Exit Code

Non-zero (process timeout/external process failure)

## Failure Category

`infrastructure-timeout`
