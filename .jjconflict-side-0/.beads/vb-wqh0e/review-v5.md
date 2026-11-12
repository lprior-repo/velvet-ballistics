# Independent Test Review: vb-wqh0e (v5)

Reviewer: LOCAL3090
Date: 2026-10-22
Commit: 917f89d539f96a9f2b5d1f295c2c44d54258b3d8
Method: Static code inspection of actual 13 + 23 test files and related source.

## Test Inventory

- `crates/vb_storage/src/journal/emergency_deny_tests.rs`: 13 tests (549 lines)
- `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs`: 23 tests (590 lines), including 4 proptest blocks and 19 unit tests

## Findings

### 1. emergency_deny_tests.rs: All 13 Tests Correct

1. `new_store_is_inactive` - Verifies fresh journal has no marker
2. `activate_returns_active_state` - Activation writes and returns Active
3. `repeated_activation_is_idempotent` - Three activations produce exact equality
4. `marker_persists_across_journal_close` - Cold reopen retains Active state
5. `separate_stores_are_isolated` - Two journals, only one activated
6. `malformed_marker_wrong_length_errors` - 5-byte [1,0,0,0,0] rejected, exact bytes preserved
7. `unsupported_version_errors` - Version 99 rejected with typed version error
8. `malformed_empty_marker_errors` - 0-byte value rejected, empty preserved
9. `malformed_trailing_bytes_errors` - 2-byte [1,0] rejected, exact bytes preserved
10. `precommit_batch_failure_returns_error_not_success` - Injected fault returns exact Io error, state stays Inactive
11. `retry_after_injected_failure` - Failed activation, then successful retry
12. `already_active_retry_performs_strict_persistence` - No shortcut; fault injection proves persistence attempted even when already Active
13. `activation_does_not_corrupt_run_events` - Run event appended before activation remains intact with exact event identity

All tests use `Result<(), JournalError>` returns with `?` pattern; zero `.unwrap()`/`.expect()`. The `expect_injected_batch_error` helper validates exact error type (JournalError::Fjall(fjall::Error::Io) with ErrorKind::Other and exact message). Malformed/unsupported tests read back the exact raw bytes to prove no silent overwrite.

### 2. fjall_keyspace_manifest_tests.rs: 11 Required Names Verified

`declared_keyspaces_count` asserts exactly 11 entries.
`declared_keyspaces_contains_required_names` explicitly lists and verifies all 11 names including `emergency_deny`.
`declared_keyspaces_no_duplicates` verifies uniqueness.

Remaining 20 tests cover prefix distinctness (proptest + unit), big-endian ordering, encode determinism, exact byte lengths, cross-keyspace non-collision, and sequence boundaries. 5 test blocks are intentionally commented as BLOCKED waiting for `try_decode_key` implementation.

### 3. Minor Gap: No Concurrent Activation Test

The constant-marker design (no sequence counter) makes concurrent activation low-risk, but a test with two threads both calling `activate_emergency_deny()` would provide additional coverage. Not blocking given the idempotent nature of the write.

## Conclusion

**STATUS: APPROVED**

All 13 emergency_deny tests are deterministic, well-structured, and cover the critical paths: new state, activation, idempotency, persistence, isolation, malformed/unsupported marker detection with byte preservation, precommit fault handling, retry behavior, and cross-keyspace isolation. The 11 required keyspace names including `emergency_deny` are explicitly verified in the manifest tests.
