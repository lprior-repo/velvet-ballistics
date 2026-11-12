# vb-16jfp cancel_kill_lattice_tests repair — Final candidate

## Changes made to `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs`

### 1. Fixed padding contract capability declarations (root cause)
- Modified `action_contract` to accept optional `required: Option<Capability>` parameter
- Padding contracts (IDs 0-6) pass `None` → empty `required_capabilities`
- Real action contract (ID 7) passes `Some(required_capability(current))`
- Fixes `AdmissionCapabilityDenied` at submit — all 18 tests now reach runtime logic

### 2. Eliminated all forged ActionTicket helper calls
- Removed dead `action_ticket(run, action)` helper function entirely
- All six affected tests now capture genuine tickets via `runtime.lookup_pending_action_ticket(run, 0)` BEFORE cancel/kill
- Tests affected: HP-3, HP-4, C4-complete, C4-fail, C4-stale, K5
- Removed unused imports: `ActionTicket`, `SeqNo`

### 3. Asserted genuine ticket identity and pending removal
- All six tests assert `lookup_pending_action_ticket(run, 0).is_none()` after cancel/kill
- Proves the pending action was genuinely removed from the shard's index

### 4. Strengthened all six tests with exact typed errors and journal invariants
- Complete-after-cancel tests (HP-4, C4-complete, C4-stale, K5): assert `Err(RuntimeError::RunNotFound)`
- Fail-after-cancel tests (HP-3, C4-fail): assert `Err(RuntimeError::InvalidActionCompletion)`
- All six tests capture journal state before stale action attempt and assert full snapshot equality after
- Missing-run tests (cancel_missing_run, kill_missing_run): assert `Err(RuntimeError::RunNotFound)`
- No more vague `is_err()` checks — exact error variant and journal non-mutation verified

### 5. Replaced panic! branches with assert_eq!
- All six typed-error match+panic branches replaced with `assert_eq!(tick_result, Err(RuntimeError::...))`
- No panic! calls remain in the test file

### 6. Replaced let _ = with assert_eq!
- `let _ = runtime.complete_action_with_output(...)` replaced with `assert_eq!(..., Ok(()))`
- No ignored fallible results remain

### 7. Replaced ActionName::new(...).unwrap() with typed fallible helper
- Added `test_action_name()` helper that returns `Result<ActionName, String>`
- Propagates error via `?` in `action_contract`

### 8. Replaced journal length comparisons with full snapshot equality
- All journal comparisons now use full `assert_eq!(journal_after, journal_before, ...)` instead of length-only
- Stronger invariant: no event additions OR removals allowed

## Test results
```
running 18 tests
All 18 passed; 0 failed; 0 ignored; 0 measured
Exit code: 0
No compiler warnings
```

## Command
```bash
PROPTEST_CASES=256 cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_tests
```

## Evidence
- Full test run log: `.beads/vb-16jfp/evidence/cli-agent/test-run-corrected.log`
- JJ change: `ulqvmuyw 1e12ef61`
- Remote SHA: `1e12ef61`
- Bookmark: `codex/vb-16jfp-cli-candidate` (pushed to origin)
- Changed path: `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` (217 insertions, 43 deletions)
- Evidence tracked via `jj file track --include-ignored .beads/vb-16jfp/evidence/cli-agent/`

## Notes on error semantics discovered
- `complete_action_with_output` after cancel/kill → tick returns `RunNotFound` (run moved to `terminal_runs`, `run_state_get` returns `None`)
- `fail_action` after cancel → tick returns `InvalidActionCompletion` (shard processes `RuntimeActionFailed` against stale run state)
- Missing run cancel/kill → tick returns `RunNotFound` (typed error from C2 boundary control)
