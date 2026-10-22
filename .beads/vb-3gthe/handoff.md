# vb-3gthe: ActionTicket run_attempt fixture migration

## Status: In Progress

### Completed
- Fixed 4 duplicate `run_attempt` argument errors in `crates/vb_core/src/action/tests.rs` (lines 1839, 1840, 1854, 1903)
- Added missing `run_attempt` field to ActionTicket initializers in:
  - `crates/vb_storage/src/codec/tests.rs:29`
  - `crates/vb_storage/src/recovery/tests.rs:115`
  - `crates/vb_storage/src/recovery/tests.rs:2454`
- Updated outdated test assertion in `crates/vb_storage/src/codec/tests/replay_integrity.rs` to account for new kind 37 (RetryCheckpoint)
- Migrated Kani symbolic ticket fixtures in `crates/vb_core/src/kani_idempotency_gates.rs` (lines 169, 646, 652) to use `kani::any()` for run_attempt
- All compilation passes: `cargo check --tests --workspace` exit code 0

### Test Results
- vb_core lib tests: PASS
- vb_storage lib tests: PASS
- vb_runtime lib tests: PASS
- Full workspace test suite: RUNNING (bg_2)

### Evidence
- `.beads/vb-3gthe/evidence/check_after_fixes.json` - compilation passes
- `.beads/vb-3gthe/evidence/test_vb_core.json` - vb_core tests pass
- `.beads/vb-3gthe/evidence/test_vb_storage2.json` - vb_storage tests pass
- `.beads/vb-3gthe/evidence/test_vb_runtime.json` - vb_runtime tests pass
- `.beads/vb-3gthe/evidence/test_all_final.json` - full workspace tests (in progress)

### Notes
- All edits are in test/bench/fuzz files and cfg(test) sections only
- No production behavior changes
- Kani fixtures use `kani::any()` for arbitrary run_attempt generation (not constant 1)
- No new unwrap/expect/panic/unsafe/lintallow added
- No Git/JJ mutations or publication done