# vb-wqh0e Test Review: vb-ct31x Durable Emergency-Deny Marker Storage Seam

## Reviewer: LOCAL3090 independent TEST reviewer

## Verdict: APPROVED

Zero open findings. All 13 contract-specified tests pass with exact assertions.

## Scope

Test review only of the emergency-deny marker storage seam implementation in vb_storage. No production code changes, no test modifications, no proof harness work.

## Test File Manifest (SHA-256)

```
b851b4c737c9b03e78198da47564c2a52444d4f7c9f4db6e828a20947d960547  crates/vb_storage/src/journal/emergency_deny.rs
a82754ae76c52a4a816f0f95e135083e1e16739dc6df6c34ac33054caf5ef22c  crates/vb_storage/src/journal/emergency_deny_tests.rs
0ac5fbe9feba724acdbec4f4e4b56fd7741ceeed7d6affc08282bfe0135f4589  crates/vb_storage/src/batch/emergency_deny.rs
007570831ca8ab251a770a760c49b7a9d560e9d174540bfc7ef9acc6f7d07161  crates/vb_storage/src/constants.rs
200f92982b6f801855adff2204d05e501a09619e774c841c626089ad15298a67  crates/vb_storage/src/error/codes.rs
```

## Test Coverage Verification

All 13 contract-specified tests are present and pass:

| # | Test Name | Contract Requirement | Result |
|---|-----------|---------------------|--------|
| 1 | `new_store_is_inactive` | New store is Inactive | ✓ Pass |
| 2 | `activate_returns_active_state` | Activation writes and returns Active | ✓ Pass |
| 3 | `repeated_activation_is_idempotent` | Idempotent repeated activation (exact equality) | ✓ Pass |
| 4 | `marker_persists_across_journal_close` | Persistence across journal drop and reopen (cold restart) | ✓ Pass |
| 5 | `separate_stores_are_isolated` | Store isolation | ✓ Pass |
| 6 | `malformed_marker_wrong_length_errors` | Malformed marker (wrong length, 5-byte [1,0,0,0,0]) | ✓ Pass |
| 7 | `malformed_empty_marker_errors` | Malformed marker (empty, 0-byte) | ✓ Pass |
| 8 | `malformed_trailing_bytes_errors` | Malformed marker (trailing, 2-byte [1,0]) | ✓ Pass |
| 9 | `unsupported_version_errors` | Unsupported version (byte 99) | ✓ Pass |
| 10 | `precommit_batch_failure_returns_error_not_success` | Failed persist returns error, not success | ✓ Pass |
| 11 | `retry_after_injected_failure` | Retry after injected precommit failure | ✓ Pass |
| 12 | `already_active_retry_performs_strict_persistence` | Already Active retry still performs strict persistence | ✓ Pass |
| 13 | `activation_does_not_corrupt_run_events` | Original run event history unchanged after activation | ✓ Pass |

## Exact Assertion Quality

All tests assert exact values, not generic success:

- **State equality**: `assert_eq!(state, EmergencyDenyState::Inactive)` / `EmergencyDenyState::Active` — exact enum variant matching
- **Error variant matching**: Tests use match expressions to verify exact `JournalError::EmergencyDenyMarkerMalformed` and `JournalError::EmergencyDenyMarkerUnsupportedVersion { version }` variants
- **Version byte preservation**: Test verifies `assert_eq!(version, 99)` for the exact version byte
- **Raw byte preservation**: Malformed/unsupported tests verify exact raw bytes preserved after rejection (`stored_bytes == Some(bad_value.to_vec())`)
- **Injected fault exactness**: `expect_injected_batch_error` verifies `JournalError::Fjall(fjall::Error::Io(e))` with `ErrorKind::Other` and exact message "batch commit test hook failure" — proves PRECOMMIT failure path, not fsync
- **Event identity**: Test uses exact event vector comparison, not just length

## Negative Findings (What Tests Would Catch If Deleted)

The test suite would fail if any of these implementation behaviors were removed:

- `new_store_is_inactive` would fail if missing key returned error instead of Inactive
- `activate_returns_active_state` would fail if activate did not write or return Active
- `repeated_activation_is_idempotent` would fail if repeated calls produced different states
- `marker_persists_across_journal_close` would fail if marker was not persisted durably
- `separate_stores_are_isolated` would fail if stores shared state
- All malformed/unsupported tests would fail if validation was skipped or errors were suppressed
- `precommit_batch_failure_returns_error_not_success` would fail if activation returned success despite write failure
- `retry_after_injected_failure` would fail if retry path was broken
- `already_active_retry_performs_strict_persistence` would fail if activation shortcut when already Active (would not hit injected fault)
- `activation_does_not_corrupt_run_events` would fail if emergency deny keyspace writes affected run event data

## Anti-Pattern Checks

- ✓ No `.expect()`, `.unwrap()`, `.panic()` in test code (verified via grep)
- ✓ No Runtime dispatch logic in this storage slice (verified by module boundary)
- ✓ No physical power failure claims (tests use Fjall's SyncAll persistence, not power simulation)
- ✓ No broad generic errors or error suppression (all tests assert exact error variants)
- ✓ No fake shadow models (tests use real FjallJournal instances with temp directories)
- ✓ Tests use `Result` return types with `?` operator, not ad-hoc error handling
- ✓ TempDir retained for fixture lifetime (not dropped prematurely)
- ✓ Test module declared in `crates/vb_storage/src/journal/mod.rs` at line 21

## Commands Executed

```bash
# Test run
timeout 600 rustup run nightly-2026-04-28 cargo test -p vb_storage emergency_deny --all-features -- --test-threads=1
# Result: 13 passed; 0 failed; 0 ignored; 0 measured; 1617 filtered out; finished in 0.12s

# Fmt check
timeout 120 rustup run nightly-2026-04-28 cargo fmt --check -- crates/vb_storage/src/journal/emergency_deny.rs crates/vb_storage/src/journal/emergency_deny_tests.rs crates/vb_storage/src/batch/emergency_deny.rs
# Result: no output (all files pass fmt)
```

## Evidence Files

- `.beads/vb-wqh0e/evidence/REVIEW_V4_01_test_output.txt` — full test output
- `.beads/vb-wqh0e/evidence/REVIEW_V4_02_fmt_check.txt` — fmt check output
- `.beads/vb-wqh0e/evidence/REVIEW_V4_03_hash_manifest.txt` — SHA-256 hash manifest

## Conclusion

The emergency-deny marker storage seam test suite comprehensively covers all 13 contract requirements with exact assertions, proper error variant matching, raw byte preservation verification, and correct fault injection testing. No test is redundant or would pass if its corresponding implementation branch were deleted. The test suite is APPROVED.
