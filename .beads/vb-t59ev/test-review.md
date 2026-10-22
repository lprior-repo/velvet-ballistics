# Test Review — vb-t59ev decomposition and Kani fixtures

Reviewer: LOCAL5090 independent test reviewer
Workspace: `/home/lewis/src/isoloated/velvet-ballistics-vb-t59ev-local5090-review`
Git root == JJ root == verified.

**STATUS: APPROVED**

## Test counts verified against evidence JSON

| Test group | Claimed | Verified (reproduced) | Exit |
|------------|--------:|---------------------:|-----:|
| vb_compile digest unit (--features test-util) | 123 | 123 | 0 |
| vb_runtime run_checkpoint | 2 | 2 | 0 |
| vb_runtime recovery | 37 | 37 | 0 |
| vb_runtime shard | 683 | 683 | 0 |
| durable_retry_run_attempt_authority | 4 | 4 | 0 |

All counts match handoff evidence. No tests dropped by decomposition.

## Kani evidence — honest, not proof closure

Evidence reviewed from `.beads/vb-ga13x/evidence/` as provided by author; not independently rerun:
- `kani-ticket-compile.json`: exit 0, 506.1s, 1 harness, 4 covers — actual proof pass
- `kani-replay-deterministic.json`: exit 124, 840.1s, timed out — NOT a pass, correctly documented
- Kani replay timeout (124/840s) preserved as open item; not proof closure for behavior-affecting claims

## Test quality — oracle not weakened

The `durable_retry_run_attempt_authority` test now has a stronger oracle:
- Previously: `tick_all().ok()` silently ignored any failure (weak)
- Now: asserts exact `InvalidActionCompletion` rejection for stale ticket replay
- Completion path still asserts `InvalidActionCompletion` for old ticket replay (unchanged)
- Run-attempt mismatch falls through per-action stale arm to invalid-completion rejection (documented behavior)

## Source length — all files under limit

`scripts/check-source-length.sh`: `over_limit=0` for all categories. All six decomposed production files now below 300 lines. 195 production warnings remain (files between 240-300 lines) but none exceed the limit.

## Limits

- Integration tests (--test targets) verified via evidence JSON counts only; not rerun locally
- Full test suite (--all-features) not run; root's canonical moon ci responsibility
- Test compile gates (--no-run) verified via evidence JSON; lib tests run directly