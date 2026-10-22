# Test Review Fix v12 - Actual Changes

## Actual Prior Execution
basic7new+2expectation tests passed. all-storage run FAILED one oldtestcount at src/tests.rs2272 (left12 right11). 1308 passed, 1 failed, 328 unrun.

## Changes Made

### 1. counter_persists_across_reopen (acceptance_sequence_tests.rs)
- Changed to retain first AcceptedArtifact outside first block
- After reopening, verify first artifact still readable via assert_readback_verified BEFORE second submit
- Explicit 1u64 BE bytes assertion retained
- Same TempDir kept alive across both opens

### 2. File split (acceptance_sequence_tests.rs + acceptance_sequence_boundary_tests.rs)
- Original file had 303 lines; added tests pushed to ~315
- Moved 3 boundary tests to new acceptance_sequence_boundary_tests.rs (116 lines)
- Added mod boundary_tests declaration with #[path] attribute
- Original file now ~194 lines with 4 core tests + helpers
- Boundary tests: corrupted_counter_length_7, corrupted_counter_length_9, counter_at_u64_max

### 3. malformed9 case (acceptance_sequence_tests.rs via boundary_tests)
- Added missing compiled_ir empty assertion (journal.compiled_ir.iter().next().is_none())
- Was already present in malformed7 case, missing in malformed9

### 4. declared_keyspaces_returns_twelve_entries (src/tests.rs)
- Renamed function from declared_keyspaces_returns_eleven_entries
- Updated comment to expect 12 keyspace names including artifact_acceptance
- Changed assert_eq!(keyspaces.len(), 11) to assert_eq!(keyspaces.len(), 12)
- Added assert_eq!(keyspaces.get(11), Some(&"artifact_acceptance"))
- All 11 original name/order assertions preserved exactly

### 5. assert_readback_verified helper (acceptance_sequence_tests.rs)
- Added explicit digest equality: computed == artifact.digest
- Added verification proof digest check: artifact.verification.digest == artifact.digest
- Added compiled workflow node count assertion via node_count()
- Decoded complete artifact Eq assertion retained
- No redundant policy assertions (API doesn't expose them)

## Pending Verification
- Re-run test suite to confirm all 7 new tests pass
- Confirm declared_keyspaces test now passes with 12 entries
- Independent test suite review needed before landing