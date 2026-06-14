# Test Suite Review: `vb_boundary_inventory`

**Crate**: `vb_boundary_inventory`
**Test Files**: 5 modules, 261 total tests (241 `#[test]`/proptest + 20 Kani harnesses)
**Reviewer**: test-reviewer agent
**Date**: 2026-06-14

---

## CRITICAL Findings (must fix before any approval)

### C1. Property test asserts a false property — will fail on every run
**File**: `src/tests/property_tests.rs:85-88`
**Test**: `boundary_candidate_marker_empty_allowed`

```rust
fn boundary_candidate_marker_empty_allowed(marker: String) {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", marker);
    prop_assert_eq!(candidate.marker.len(), 0);  // <-- ALWAYS false for non-empty marker
}
```

**Problem**: The proptest `marker: String` generates arbitrary non-empty strings. The test asserts the marker length is always 0, which contradicts what was passed in. This is not a valid property — it asserts the constructor destroys the input. **This test has never passed.**

**Fix**: Either remove it or change to a real roundtrip property:
```rust
prop_assert_eq!(candidate.marker, marker);
```

---

### C2. Error variant Debug tests test derive macros, not behavior (13 tests)
**File**: `src/tests/error_tests.rs:11-87`
**Tests**: `error_variant_workspace_not_discoverable` through `error_variant_review_status_invalid`

**Problem**: Each test creates an error variant and asserts `format!("{:?}", err) == "VariantName"`. These test the `#[derive(Debug)]` behavior, not any domain behavior. If you deleted the entire `BoundaryInventoryError` enum, these would fail — but they also fail if you add a single field to any variant. They have zero mutation-test value and prove nothing about the actual codebase behavior.

**Disposition**: Delete all 13. They are code smell, not evidence. The 9 other error tests (equality, hash, size, Send/Sync, Result context) are sufficient to prove the enum is well-formed.

---

### C3. Error hash tests are redundant with equality tests (4 tests)
**File**: `src/tests/error_tests.rs:152-319`
**Tests**: `error_hash_consistency`, `error_hash_all_unique`, `error_hash_all_13_variants_in_set`, `error_hash_collision_free_across_13_variants`

**Problem**: For unit-variant enums, unique equality implies unique hashes. All 4 of these tests are redundant with `error_eq_all_variants_unique` (line 102-128). The hash uniqueness test at line 291 is especially overblown — iterating through 13 variants and collecting hashes into a HashSet is 50 lines of code that does what the 27-line equality test already proves.

**Disposition**: Delete `error_hash_consistency`, `error_hash_all_13_variants_in_set`, and `error_hash_collision_free_across_13_variants`. Keep only `error_hash_all_unique` as a compact proof that all 13 variants hash differently.

---

## HIGH Severity Findings

### H1. Property test with unused bool parameter — dead weight
**File**: `src/tests/property_tests.rs:90-96`
**Test**: `classify_boundary_unknown_marker_produces_unknown_class`

```rust
fn classify_boundary_unknown_marker_produces_unknown_class(_flag: bool) {
    let candidate = BoundaryCandidate::new("crates/test/src/lib.rs", "unknown-marker");
    ...
}
```

**Problem**: The `_flag: bool` parameter is proptest-generated but never used. The test input is hardcoded ("unknown-marker"). This is not a property test — it's a regular `#[test]` wearing a proptest costume. It proves nothing that the existing `classify_boundary_rejects_unrecognized_marker` (api_tests.rs:950-958) doesn't already prove.

**Disposition**: Delete. Redundant with existing unit test.

---

### H2. Property test asserts a property that fails for valid inputs
**File**: `src/tests/property_tests.rs:80-83`
**Test**: `boundary_candidate_path_preserves_slashes`

```rust
fn boundary_candidate_path_preserves_slashes(path: String) {
    let candidate = BoundaryCandidate::new(path.clone(), "test-marker".to_string());
    prop_assert!(!candidate.source_path.as_os_str().is_empty());
}
```

**Problem**: Proptest generates strings including the empty string. If `path == ""`, then `candidate.source_path` will be empty, and this assertion fails. There is no `prop_assume!` guard. This test will fail periodically during shrinking.

**Fix**: Add `prop_assume!(!path.is_empty());` at the top.

---

### H3. Property tests with unused bool parameters (4 tests)
**File**: `src/tests/property_tests.rs`
**Tests**:
- `boundary_exposure_none` (line 163): `_flag: bool` unused
- `field_state_from_option_none` (line 278): `_flag: bool` unused
- `field_state_missing_as_ref_prop` (line 286): `_flag: bool` unused
- `field_state_missing_map_prop` (line 295): `_flag: bool` unused

**Problem**: These all follow the pattern `fn name(_flag: bool) { /* deterministic, no use of _flag */ }`. They are regular unit tests in proptest clothing. The `_flag` parameter is dead weight.

**Disposition**: Delete the parameter. These are deterministic properties that should use no input parameters.

---

### H4. Validation test asserts `is_err()` without checking error variant
**File**: `src/tests/validation_tests.rs:283-288`
**Test**: `validate_root_relative_path_rejected`

```rust
fn validate_root_relative_path_rejected() {
    let text = "crates/test/src/lib.rs";
    let result = validate_evidence_reference_bytes(text.as_bytes());
    // File likely doesn't exist relative to manifest dir -> error
    assert!(result.is_err());  // <-- ANY error, not just InvalidEvidencePath
}
```

**Problem**: The comment says "File likely doesn't exist" — but this depends on the test environment. If the file exists (e.g., in CI or if the crate has matching files), the test would pass with `Ok(RepoLocal{...})` instead of `Err`, making `is_err()` fail. The test should assert the specific error variant, not just "some error".

**Fix**: Assert `InvalidEvidencePath` explicitly:
```rust
assert_eq!(result.unwrap_err(), BoundaryInventoryError::InvalidEvidencePath);
```

---

### H5. Test comments leak implementation details
**File**: `src/tests/validation_tests.rs:40-47, 228-235`
**Tests**: `validate_bead_id_uppercase_rejected`, `validate_bead_id_suffix_with_uppercase_rejected`

```rust
// Uppercase not allowed in suffix - falls through to repo_local path
assert!(result.is_err());  // <-- doesn't check which error
```

**Problem**: The comment describes the implementation path ("falls through to repo_local path") rather than the intended behavior. The test also only asserts `is_err()` instead of the specific error variant.

**Disposition**: Assert `InvalidEvidencePath` and rewrite the comment to describe behavior, not implementation.

---

### H6. API test assertion is trivially true
**File**: `src/tests/api_tests.rs:289-299`
**Test**: `classify_boundary_id_path_normalization`

```rust
// IDs may differ due to different paths
assert!(!classified1.id.is_empty());
assert!(!classified2.id.is_empty());
```

**Problem**: The comment says "IDs may differ" but the test never asserts they differ. It only asserts both IDs are non-empty, which is trivially true for any classified boundary. The test name promises normalization behavior but asserts nothing about it.

**Fix**: Add `prop_assert_ne!(classified1.id, classified2.id);` or similar if that's the actual property being tested.

---

## MEDIUM Severity Findings

### M1. Property test roundtrip for empty marker — broken assertion
**File**: `src/tests/property_tests.rs:85-88`
See C1 above.

---

### M2. Test name is misleading — "all_safe_boundaries" uses CAbi (risky) class
**File**: `src/tests/api_tests.rs:604-617`
**Test**: `inventory_completion_status_all_safe_boundaries`

```rust
fn inventory_completion_status_all_safe_boundaries() {
    let record1 = make_valid_record("test-id-1"); // CAbi class
    let record2 = make_valid_record("test-id-2"); // CAbi class
    ...
}
```

**Problem**: CAbi (C ABI) boundaries are not "safe" — they carry `BoundaryRisk::Multiple` exposure. The test name claims "safe boundaries" but creates risky boundaries.

**Disposition**: Rename test to `inventory_completion_status_multiple_caabi_boundaries` or use `BoundaryClass::Unknown` if truly testing "safe" boundaries.

---

### M3. Multiple redundant property tests for FieldState
**File**: `src/tests/property_tests.rs`
**Tests**: `field_state_from_option_some` (225), `field_state_as_ref` (233), `field_state_map` (242), `field_state_from_option_none` (278), `field_state_missing_as_ref_prop` (286), `field_state_missing_map_prop` (295)

**Plus regular tests**: `field_state_from_none_returns_missing`, `field_state_missing_map_preserves_missing`, `field_state_missing_as_ref_preserves_missing` (property_tests.rs:476-493)

**Problem**: 9 tests (6 proptest + 3 regular) cover the same `FieldState` operations. The proptest versions add no value over the regular tests — they just generate a `String` and verify it roundtrips through `Present(v)`.

**Disposition**: Keep the 3 regular tests. Delete the 6 proptest versions.

---

### M4. Multiple redundant property tests for EvidenceReference
**File**: `src/tests/property_tests.rs`
**Tests**: `evidence_reference_repo_local_roundtrip` (193), `evidence_reference_free_text_roundtrip` (204), `evidence_reference_external_provenance` (214), `evidence_reference_repo_local_with_kind` (326), `evidence_reference_external_provenance_format` (362)

**Problem**: 5 proptest tests that each verify a single constructor's field is preserved. These are trivially true by construction (the enum variant stores the value directly). No actual computation or invariant is being tested.

**Disposition**: Consolidate to a single test per variant in the regular test section. Delete 3 proptest duplicates.

---

### M5. Multiple redundant proptest tests for ReviewStatus
**File**: `src/tests/property_tests.rs`
**Tests**: `review_status_from_serialized_other` (251), `review_status_serialized_other` (260), `review_status_from_serialized_then_serialized` (349)

**Plus regular tests**: `review_status_from_serialized_approved`, `review_status_from_serialized_waived`, `review_status_serialized_approved`, `review_status_serialized_waived`, `review_status_from_serialized_unique_values` (property_tests.rs:433-451, 544-551)

**Problem**: 8 tests across proptest and regular cover the same `ReviewStatus::from_serialized`/`.serialized()` roundtrip. The proptest versions add no value.

**Disposition**: Keep the 5 regular tests. Delete the 3 proptest versions.

---

## LOW Severity Findings

### L1. `parse_inventory_source_path_only_whitespace` documents questionable behavior
**File**: `src/tests/parser_tests.rs:491-503`
**Test**: `parse_inventory_source_path_only_whitespace`

**Problem**: Asserts that whitespace-only source paths are accepted. This may be intentional, but the test should explicitly document this is a design choice, not an oversight. Consider adding `// DESIGN: whitespace-only paths are accepted — the parser treats them as non-empty` comment.

### L2. `error_size_is_1_byte` — implementation detail test
**File**: `src/tests/error_tests.rs:357-361`
**Test**: `error_size_is_1_byte`

**Problem**: Asserts `size_of::<BoundaryInventoryError>() == 1`. This is an implementation detail that may change if variants gain fields. It has no behavioral significance.

### L3. `error_is_copy_and_clone` — duplicate of existing tests
**File**: `src/tests/error_tests.rs:363-370`

**Problem**: Tests Copy + Clone again, but `error_copy` (line 141-146) and `error_clone` (line 134-139) already cover this.

**Disposition**: Delete this test.

---

## Summary

| Severity | Count | Disposition |
|----------|-------|-------------|
| CRITICAL | 3 | Must fix: C1 will cause test failure, C2 & C3 are dead weight |
| HIGH | 6 | Must fix: C2/C3 deletion is prerequisite; H1-H6 are broken or redundant |
| MEDIUM | 5 | Should fix: consolidate redundant tests |
| LOW | 3 | Nice to fix: minor cleanup |

**Tests to delete outright**: ~50 tests (C1 broken test, C2's 13 debug tests, C3's 3 hash tests, H1's dead proptest, H3's 4 unused-bool tests, M3's 6 proptest FieldState tests, M4's 3 proptest EvidenceReference tests, M5's 3 proptest ReviewStatus tests, L3 duplicate)

**Tests to fix**: ~8 tests (H2 add prop_assume, H4 assert specific variant, H5 assert specific variant + rewrite comment, H6 add actual assertion, M2 rename test)

**Net effect**: Reduce test count from 261 to ~210 while improving quality. The deleted tests provide near-zero mutation resistance and prove derive-macro behavior, not domain behavior. The fixed tests will actually catch bugs instead of being dead or broken.
