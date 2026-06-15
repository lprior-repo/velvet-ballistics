# Test Gap Analysis: `vb_boundary_inventory`

**Scope:** `crates/vb_boundary_inventory/src/` (production) vs `crates/vb_boundary_inventory/src/tests/` (tests)
**Files analyzed:** `api.rs`, `parser.rs`, `validation.rs`, `record.rs`, `types.rs`, `inventory.rs`, `status.rs`
**Test files analyzed:** `api_tests.rs`, `parser_tests.rs`, `validation_tests.rs`, `error_tests.rs`, `property_tests.rs`

---

## HIGH Severity (3 gaps)

### H-1: `ValidatedBoundaryInventory::PartialEq` — empty records, different `discovered_boundary_count`
- **File:** `inventory.rs:34-43` → `inventory.rs:110-114` (`count_matches`)
- **Untested behavior:** When both `records` vectors are empty, `eq` falls back to comparing `discovered_boundary_count`. Two inventories with `records = []` but `discovered_boundary_count = 0` vs `discovered_boundary_count = 5` must NOT be equal.
- **Missing test:** No test in any test file calls `==` on two `ValidatedBoundaryInventory` instances. The property tests only assert `.discovered_boundary_count` field equality, not struct-level `PartialEq`.
- **Edge case to exercise:** `empty_with_discovered_boundary_count(0) == empty_with_discovered_boundary_count(5)` must be `false`.
- **Why it matters:** Incorrect equality would cause false-positive deduplication, incorrect inventory matching in comparison operations, and subtle logic errors.

### H-2: `ValidatedBoundaryInventory::PartialEq` — records differ but `discovered_boundary_count` matches
- **File:** `inventory.rs:110-114` (`count_matches`)
- **Untested behavior:** When records are non-empty, `count_matches` returns `left.records == right.records`. It does NOT fall back to `discovered_boundary_count` when records differ. So two inventories with different records but the same `discovered_boundary_count` must NOT be equal.
- **Missing test:** No test verifies this fallback behavior. The `PartialEq` impl has three code paths (both `None` review status, one `None`, both present) plus the records-vs-count fallback — none are directly tested.
- **Edge case:** Two inventories with 2 records each (same count) but different record `id` values — must be `false`.
- **Why it matters:** Same as H-1.

### H-3: `collect_directory_markers` — permission denied during filesystem scan
- **File:** `api.rs:177-187` (`collect_directory_markers`)
- **Untested behavior:** When `fs::read_dir(absolute)` fails with an I/O error (e.g., permission denied on a subdirectory), the function maps to `BoundaryInventoryError::WorkspaceNotDiscoverable`. This error path is never exercised.
- **Missing test:** No test creates a subdirectory with restricted permissions inside a valid workspace.
- **Edge case:** Valid workspace + subdirectory with `chmod 000` — `fs::read_dir` fails, should return `WorkspaceNotDiscoverable`.
- **Why it matters:** Real-world filesystem scenarios (permissions, locked files) silently fail with the wrong error code.

---

## MEDIUM Severity (10 gaps)

### M-1: `collect_file_markers` — permission denied reading marker file
- **File:** `api.rs:195-196` (`collect_file_markers`)
- **Untested behavior:** When `fs::read_to_string(absolute)` fails (e.g., permission denied), maps to `WorkspaceNotDiscoverable`. Never exercised.
- **Edge case:** Marker file with `chmod 000` inside valid workspace → `WorkspaceNotDiscoverable`.
- **Why it matters:** Same as H-3.

### M-2: `decoder_surface_omitted` — `boundary-surfaces.txt` read error
- **File:** `api.rs:136-137` (`decoder_surface_omitted`)
- **Untested behavior:** When `fs::read_to_string(config)` fails, maps to `WorkspaceNotDiscoverable`. Existing tests (lines 160-178, 1311-1355 of api_tests.rs) only test the case where the file exists AND is readable.
- **Edge case:** File exists but permission denied → `WorkspaceNotDiscoverable`.
- **Why it matters:** Realistic error path never exercised.

### M-3: `validate_source_path` — path escapes via `..`
- **File:** `validation.rs:85-93` (`validate_source_path`)
- **Untested behavior:** The function checks `path.starts_with("crates")` etc. but does NOT reject `..` traversal. A path like `"crates/../../../etc/passwd"` passes validation.
- **Edge case:** Record with `source_path = "crates/../../etc/passwd"` — passes validation but would resolve outside the workspace when joined with workspace path.
- **Why it matters:** Security-relevant: allows crafting records that reference files outside the workspace boundary.

### M-4: `stable_id` — path with repeated special characters
- **File:** `api.rs:230-233` (`stable_id`)
- **Untested behavior:** `stable_id` replaces `/`, `.`, `_` with `-` individually. It does NOT canonicalize paths first. Path `"crates//test//src//lib.rs"` (double slashes) produces a different ID than `"crates/test/src/lib.rs"`.
- **Edge case:** Two paths with different separator patterns (single vs double slashes, trailing slashes) produce different IDs for the same logical file.
- **Why it matters:** Non-deterministic ID generation for equivalent paths could cause duplicate boundaries.

### M-5: `parse_record` — whitespace-only source_path silently accepted
- **File:** `parser.rs:40-43` (`parse_record`)
- **Untested behavior:** `source_path = "   "` (3 spaces). `PathBuf::from("   ")` produces a path where `as_os_str().is_empty()` returns `false`, so the parser accepts it.
- **Missing test:** The test `parse_inventory_source_path_only_whitespace` (parser_tests.rs:492-503) expects success but doesn't assert anything meaningful — it's a "does it not panic" test, not a "is this valid" test.
- **Why it matters:** Semantic correctness — whitespace-only source paths are meaningless and should probably be rejected.

### M-6: `inventory_completion_status` — `discovered_boundary_count` with `usize::MAX`
- **File:** `api.rs:86-88` (`inventory_completion_status`)
- **Untested behavior:** When `records.is_empty()` and `discovered_boundary_count = usize::MAX`, the check `discovered_boundary_count != 0` is `true`, and `IncompleteDiscoveryInput` is returned. But no test exercises this extreme edge case.
- **Edge case:** `empty_with_discovered_boundary_count(usize::MAX)` → should return `IncompleteDiscoveryInput`.
- **Why it matters:** Boundary condition at the extreme of the `usize` domain.

### M-7: `review_status_matches` — both `None` in `PartialEq` context
- **File:** `inventory.rs:103-108` → `inventory.rs:38-40`
- **Untested behavior:** `review_status_matches(None, None)` returns `true`. When both inventories have `review_status = None`, the review status comparison passes regardless of other fields.
- **Edge case:** `ValidatedBoundaryInventory::with_schema_version(1)` == `ValidatedBoundaryInventory::with_schema_version(99)` when records are both empty — should be `false` (schema version differs) OR `true` (review_status both None, records both empty, but count_matches also passes since both counts are 0). This needs explicit testing.
- **Why it matters:** Tests the full `PartialEq` logic chain.

### M-8: `is_risky_boundary` — `UnsafeAdjacentDependency` with `BoundaryRisk::None`
- **File:** `api.rs:94-100` (`is_risky_boundary`)
- **Untested behavior:** The function returns `true` for `UnsafeAdjacentDependency` regardless of risk level. While `required_evidence` is tested with this class at api_tests.rs:374-387, it uses `BoundaryExposure::none()`. **However**, the `is_risky_boundary` function is a private function and is NOT tested in isolation — it's only tested through the `required_evidence` integration path.
- **Edge case:** Direct call to `is_risky_boundary` with all class/risk combinations should be tested.
- **Why it matters:** Private function not directly tested; if its logic changes, no unit test would catch it.

### M-9: `validate_external_reference` — valid bead ID in external provenance context
- **File:** `validation.rs:133-138` (`validate_external_reference`)
- **Untested behavior:** `validate_external_reference("vb-abc123")` should return `Ok(())` because `valid_bead_id("vb-abc123")` is true. This is tested indirectly through `validate_inventory_approved_with_external_evidence_ok` (api_tests.rs:1216-1228), but no **direct** unit test exists for this function.
- **Edge case:** Direct call with various bead IDs and non-bead strings.
- **Why it matters:** Coverage gap for a validation function — it's only tested through integration, not in isolation.

### M-10: `classify_boundary` — idempotency across all fields
- **File:** `api.rs:30-40` (`classify_boundary`)
- **Untested behavior:** The test at api_tests.rs:853 (`classify_boundary_stability_idempotent`) checks `id` equality but also verifies `class`, `source_path`, and `exposure.risk`. **However**, it only tests ONE marker/class combination (`extern-c-boundary`). It does NOT test idempotency for all 7 marker types.
- **Edge case:** Call `classify_boundary` with each of the 7 markers twice and verify ALL fields match both times.
- **Why it matters:** Only 1 of 7 marker paths is tested for full idempotency.

---

## LOW Severity (12 gaps)

### L-1: `class_from_marker` — empty marker string
- **File:** `api.rs:218-228` (`class_from_marker`)
- **Missing test:** `classify_boundary(BoundaryCandidate::new("x", ""))` — empty marker should fall through to `_unknown => UnknownBoundaryClass`. Tests have `"nonexistent-marker-xyz"` but not `""`.
- **Why:** Defensive correctness.

### L-2: `parse_inventory` — JSON null in boundaries array
- **File:** `parser.rs:36-55` (`parse_record`)
- **Missing test:** `{"schema_version": 1, "boundaries": [null]}` — `as_object()` returns `None` → `InventoryParseFailure`. Tests have `"not an object"` string but not JSON null, number, boolean.
- **Why:** Defensive JSON parsing.

### L-3: `parse_inventory` — JSON number/boolean in boundaries array
- **File:** `parser.rs:36-55`
- **Missing test:** `{"schema_version": 1, "boundaries": [42]}` or `{"schema_version": 1, "boundaries": [true]}`.
- **Why:** Defensive JSON parsing.

### L-4: `parse_inventory` — schema version as u32::MAX (via u64)
- **File:** `parser.rs:28-33` (`parse_schema_version`)
- **Missing test:** JSON number `4294967295` (u32::MAX) — valid u64, not equal to 1, so `SchemaVersionUnsupported`. The test `parse_inventory_schema_version_u32_max_rejected` exists at parser_tests.rs:571-577. **Already covered — remove gap.**
- **Correction:** Already tested. Skip.

### L-5: `ReviewStatus::from_serialized` — empty string
- **File:** `types.rs:180-185`
- **Missing test:** `from_serialized("")` → `Other("")`. No test for this edge case.
- **Why:** Semantic correctness.

### L-6: `FreshnessMarker::new` — all-zero versions
- **File:** `types.rs:161-166`
- **Missing test:** `FreshnessMarker::new(0, 0, 0)` — should be constructible. The `validate_freshness` function checks `evidence_version < source_version` which would be `0 < 0 = false`, so freshness would be valid.
- **Why:** Boundary condition.

### L-7: `EvidenceReference::free_text` — not tested in isolation
- **File:** `types.rs:146-149`
- **Missing test:** `EvidenceReference::free_text("")` → `FreeText("")`. Only tested indirectly through `validate_inventory_free_text_evidence_rejected`.
- **Why:** Constructor not directly tested.

### L-8: `BoundaryCandidate::new` — both fields empty
- **File:** `types.rs:56-63`
- **Missing test:** `BoundaryCandidate::new("", "")` — both source_path and marker empty. Property test `boundary_candidate_new_roundtrip` generates both strings but the regex `[a-z]*` (via property test default) can generate empty strings, though `boundary_candidate_path_preserves_slashes` assumes non-empty.
- **Why:** Edge case.

### L-9: `ValidatedBoundaryInventory::empty_with_discovered_boundary_count` — accessor only
- **File:** `inventory.rs:93-100`
- **Missing test:** The property test `inventory_completion_status_empty_record_count` (property_tests.rs:176-179) tests the accessor but not the full struct construction.
- **Why:** Constructor path not directly verified.

### L-10: `record.review_status()` — `Present(ReviewStatus::Other(...))`
- **File:** `record.rs:95-100`
- **Missing test:** `review_status()` with `ReviewStatus::Other("pending")` should return `Some("pending")`. Tests only check `Present(Approved)` and `Missing`.
- **Why:** Missing variant coverage.

### L-11: `review_status.serialized()` — `ReviewStatus::Other("custom")`
- **File:** `types.rs:189-194`
- **Missing test:** `ReviewStatus::Other("custom-review".into()).serialized()` → `Some("custom-review")`. Tests only check `Approved` and `Waived`.
- **Why:** Missing variant coverage.

### L-12: `BoundaryRecordDraft::new` — all `FieldState` combinations
- **File:** `record.rs:79-91`
- **Missing test:** No test exercises the `BoundaryRecordDraft::new` constructor with all `FieldState::Present(...)` fields set explicitly, verifying each field is preserved through the conversion from `BoundaryRecordParts`.
- **Why:** Constructor not directly tested in isolation.

---

## Summary Table

| ID | Severity | File:Function | Gap Summary |
|----|----------|---------------|-------------|
| H-1 | HIGH | inventory.rs:34-43 | `ValidatedBoundaryInventory::eq` — empty records, different count must not be equal |
| H-2 | HIGH | inventory.rs:110-114 | `count_matches` — different records with same count must not be equal |
| H-3 | HIGH | api.rs:177-187 | `collect_directory_markers` — permission denied not tested |
| M-1 | MEDIUM | api.rs:195-196 | `collect_file_markers` — permission denied not tested |
| M-2 | MEDIUM | api.rs:136-137 | `decoder_surface_omitted` — read error not tested |
| M-3 | MEDIUM | validation.rs:85-93 | `validate_source_path` — `..` traversal not rejected |
| M-4 | MEDIUM | api.rs:230-233 | `stable_id` — path normalization with repeated special chars |
| M-5 | MEDIUM | parser.rs:40-43 | `parse_record` — whitespace-only source_path silently accepted |
| M-6 | MEDIUM | api.rs:86-88 | `inventory_completion_status` — `usize::MAX` discovered count |
| M-7 | MEDIUM | inventory.rs:103-108 | `review_status_matches` — both None in PartialEq |
| M-8 | MEDIUM | api.rs:94-100 | `is_risky_boundary` — not tested in isolation |
| M-9 | MEDIUM | validation.rs:133-138 | `validate_external_reference` — not tested in isolation |
| M-10 | MEDIUM | api.rs:30-40 | `classify_boundary` — idempotency only tested for 1 of 7 markers |
| L-1 | LOW | api.rs:218-228 | `class_from_marker` — empty marker string |
| L-2 | LOW | parser.rs:36-55 | `parse_record` — JSON null in boundaries array |
| L-3 | LOW | parser.rs:36-55 | `parse_record` — JSON number/boolean in boundaries array |
| L-4 | LOW | types.rs:180-185 | `ReviewStatus::from_serialized` — empty string |
| L-5 | LOW | types.rs:161-166 | `FreshnessMarker::new` — all-zero versions |
| L-6 | LOW | types.rs:146-149 | `EvidenceReference::free_text` — not tested in isolation |
| L-7 | LOW | types.rs:56-63 | `BoundaryCandidate::new` — both fields empty |
| L-8 | LOW | inventory.rs:93-100 | `empty_with_discovered_boundary_count` — constructor not verified |
| L-9 | LOW | record.rs:95-100 | `review_status()` — `Other` variant not tested |

---

## Notable Coverage Observations

1. **Extensively covered:** `discover_boundaries` (all marker types, surface files), `classify_boundary` (all 7 markers), `required_evidence` (all risk variants), `validate_inventory` (all error types), `parse_inventory` (JSON edge cases, schema versions), `validate_evidence_reference_bytes` (bead IDs, external provenance, path validation), `BoundaryInventoryError` (all 13 variants, equality, hashing).

2. **Notably absent:** Any direct test of `PartialEq` for `ValidatedBoundaryInventory`. The struct has a custom `PartialEq` implementation with multiple code paths, but no test exercises it directly.

3. **Private function coverage:** `is_risky_boundary`, `validate_external_reference`, `review_status_matches`, `count_matches` are all private functions with no direct unit tests. They're tested only through public API integration.

4. **Filesystem error paths:** None of the three `fs::read_to_string` / `fs::read_dir` error paths are tested (api.rs lines 137, 178, 196).

5. **Property tests:** Well-covered for type roundtrip invariants, but don't exercise error paths or equality semantics.
