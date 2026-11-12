# vb-qz2yn Integration Repair v18

Local5090 integration/test repair authoring complete. All native source edits applied.
Awaiting root compile/format/execute to confirm actual behavior.

## Changes Applied

1. **JJ conflict resolved** in `crates/vb_storage/src/admission/tests.rs`
   - Preserved all 1190 lines of existing test code
   - Kept all three child module hooks: acceptance_sequence_tests, acceptance_sequence_failure_tests, acceptance_sequence_concurrency_tests

2. **Compilation fix** in `acceptance_sequence_failure_tests.rs`
   - `compiled_empty`: removed erroneous `.iter().map_err()` (iter() returns iterator, not Result)
   - Now uses `journal.compiled_ir.iter().next().is_some()` for empty check

3. **Borrow fix** in `batch_commit_failure_is_precommit_no_state_change`
   - Changed `matches!(result, ...)` to `matches!(&result, ...)` with valid 2024 match ergonomics
   - Removed `ref` from inner pattern; preserved exact IoOther check

4. **Policy parameter and exact equality** in `verify_artifact_readback`
   - Added explicit `policy: RuntimePolicy` parameter
   - Changed field-by-field comparison to `decoded != *expected` (full AcceptedArtifact Eq)
   - Preserved canonical digest, proof digest, and record digest checks
   - Updated all callers: Strict for postcommit tests, Relaxed for near-MAX overflow tests
   - Reduced file from 314 to 304 lines by removing verbose field comparisons and blank lines

5. **Keyspace count assertions** in `crates/vb_storage/src/tests.rs`
   - Line 726: `assert_eq!(FjallJournal::declared_keyspaces().len(), 12)` (was 11)
   - Line 1651: `assert_eq!(FjallJournal::declared_keyspaces().len(), 12)` (was 11)
   - Existing test at 2267 already expects 12; left unchanged

6. **Workspace test updates**
   - `fjall_keyspace_manifest_tests.rs`: count 11->12, message updated, added "artifact_acceptance" to required names array
   - `restate_doctor_key_decode_tests.rs`: renamed test to "returns_twelve", count 11->12, updated comment, added artifact_acceptance contains check

## Verification Status

- [ ] Compile: `cargo build --all-targets`
- [ ] Format: `cargo fmt --all --check`
- [ ] Execute: `cargo test -p vb_storage --lib -- acceptance_sequence`
- [ ] Execute: `cargo test -p vb_storage --lib -- keyspace`
- [ ] Execute: `cargo test -p workspace_tests --test fjall_keyspace_manifest_tests`
- [ ] Execute: `cargo test -p workspace_tests --test restate_doctor_key_decode_tests`

Root must run full compile and test suite to confirm actual behavior before declaring success.