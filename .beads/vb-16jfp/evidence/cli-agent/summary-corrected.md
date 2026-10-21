# vb-16jfp cancel_kill_lattice_tests repair — Corrected candidate

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
- All six tests capture journal state before stale action attempt and assert journal length unchanged after
- No more vague `is_err()` checks — exact error variant and journal non-mutation verified

## Test results
```
running 18 tests
All 18 passed; 0 failed; 0 ignored; 0 measured
No compiler warnings
```

## Command
```bash
PROPTEST_CASES=256 cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_tests
```

## Evidence
- Full test run log: `.beads/vb-16jfp/evidence/cli-agent/test-run-corrected.log`
- JJ change: `ulqvmuyw 9f776699`
- Bookmark: `codex/vb-16jfp-cli-candidate` (pushed to origin)
- Changed path: `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` (217 insertions, 43 deletions)

## Notes on error semantics discovered
- `complete_action_with_output` after cancel/kill: run moved to `terminal_runs`, `run_state_get` returns `None`, tick returns `RunNotFound`
- `fail_action` after cancel/kill: `fail_action` takes `&self` and enqueues `RuntimeActionFailed`; shard processing returns `InvalidActionCompletion` because run state no longer matches
