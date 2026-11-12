# Moon :test Gate Report for vb-2bok

**Date:** 2026-05-09
**Command:** `moon run :test`
**Exit Code:** 1 (timeout/process terminated)

## Summary

The Moon `:test` gate was executed but failed. The process was terminated after exceeding the 300 second timeout.

## Stages Completed

1. `velvet-ballistics:agent-cli-contract` - cached (277050d8)
2. `velvet-ballistics:supply-chain` - running for ~60s before termination
3. `velvet-ballistics:nightly-feature-gate` - started

## Failure Details

```
Error: process::failed
  × Process git failed: terminated
```

The git process within the `supply-chain` task was terminated, causing the overall command to fail.

## Output Location

Full output saved to: `/home/lewis/.local/share/opencode/tool-output/tool_e0f1f1a33001q8mRWBKb94pXza`

## Recommendation

The test command exceeded the 300 second timeout. Consider:
- Increasing the timeout for the `:test` task
- Investigating why the `supply-chain` task is taking excessive time
- Checking for git process issues in the environment
