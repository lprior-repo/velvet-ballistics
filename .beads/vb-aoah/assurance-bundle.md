# Assurance Bundle — vb-aoah (migration skeleton tests)

**Bead:** vb-aoah  
**Title:** storage: Add explicit migration skeleton and cleanup tests  
**Parent:** vb-8mdp (EPIC: Restate architecture steal plan)  
**Date:** 2026-05-27  
**State:** 14 — evidence-packaging + truth-serum  
**Status:** PENDING_PRODUCTION_CLOSURE (approved for test-first skeleton phase)

---

## Requirement-to-Evidence Traceability

### R1: Explicit migration moves old record shape into new keyspace and updates manifest

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Behavior test | `restate_explicit_migration_skeleton_tests.rs` lines 309-325 (validate_advance), 327-336 (try_cleanup) | PASS |
| Proptest | B8-B10 (verify-before-advance), B11-B13 (cleanup), B21 (manifest gate) | PASS |
| Unit test | `advance_from_verified_with_cleanup_done_succeeds` (L1073), `advance_from_cleaned_phase_succeeds` (L1085) | PASS |
| Contract model | Adapter `cleanup_then_advance` (L383-392) | PASS_ADAPTER |

### R2: Reopening after migration reads new records without invoking migration

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Proptest | B14 (`vb_aoah_reopen_after_migration_idempotent`, L707-712) | PASS |
| Proptest | B15 (`vb_aoah_reopen_counter_unchanged`, L716-727) | PASS |
| Unit test | `reopen_does_not_rerun_migration` (L1134-1139) | PASS |
| Unit test | `reopen_current_store_records_readable` (L1125-1130) | PASS |

### R3: Invalid input old schema at runtime open returns MigrationRequired

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Proptest | B1 (`vb_aoah_runtime_open_migration_required_no_side_effects`, L564-579) | PASS |
| Proptest | B4 (`vb_aoah_runtime_open_future_version_rejected`, L598-605) | PASS |
| Unit test | `runtime_open_future_version_returns_unsupported_schema_version` (L1002-1011) | PASS |
| Production parity | `crates/vb_storage/src/codec/validation.rs:10-17` (validate_schema_version) | ALIGNED |

### R4: Missing migration verification prevents manifest update

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Proptest | B8 (`vb_aoah_verify_before_manifest_advance`, L631-646) | PASS |
| Unit test | `advance_manifest_from_copied_phase_is_rejected` (L1033) | PASS |
| Unit test | `advance_manifest_from_planned_phase_is_rejected` (L1047) | PASS |
| Unit test | `advance_rejected_manifest_version_stays_old` (L1061) | PASS |

### R5: Empty old keyspace migration leaves manifest unchanged or records NoOp

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Proptest | B16 (`vb_aoah_empty_old_keyspace_explicit_noop`, L734-743) | PASS |
| Proptest | B17 (`vb_aoah_empty_noop_cannot_claim_verified`, L747-759) | PASS |
| Unit test | `migration_from_empty_old_keyspace_produces_noop` (L1143-1146) | PASS |

### R6: Partial old keyspace cleanup failure returns typed migration error

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Proptest | B12 (`vb_aoah_cleanup_nonempty_returns_typed_error`, L675-692) | PASS |
| Unit test | `cleanup_excess_records_returns_failed_with_remaining_count` (L1113-1121) | PASS |

### R7: Every supported old version names a migration function

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Proptest | B5 (`vb_aoah_migration_registry_totality_uniqueness`, L611-624) | PASS |
| Unit test | `registry_lookup_returns_expected_name_for_supported_version` (L416-422) | PASS |
| Table test | `registry_lookup_matrix_covers_all_version_classes` (L885-934) | PASS |

### R8: Runtime startup does not mutate old schema

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Proptest | B22 (`vb_aoah_runtime_open_never_invokes_cold_path`, L813-826) | PASS |
| Unit test | `runtime_open_never_invokes_cold_path` (L1157-1171) | PASS |
| Proptest | Invariant `proptest_detection_no_side_effects` (L866-876) | PASS |

### R9: Bounded accounting with checked arithmetic

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Proptest | B18-B20 (`vb_aoah_migration_accounting_overflow_returns_error`, L767-790) | PASS |
| Unit test | Multiple (L497-553) | PASS |
| Table test | `checked_add_matrix_covers_all_cases` (L942-994) | PASS |

### R10: All 17 error variants modeled

| Evidence Type | Artifact | Status |
|--------------|----------|--------|
| Enum definition | `MigErr` enum (L97-117, 17 variants) | PASS |
| Test coverage | 8 variants exercised via adapters, 9 await production wiring | PARTIAL (expected) |
| Kani harnesses | 7/7 VERIFIED against adapters (State 5) | VERIFIED_ADAPTER |

---

## Execution Evidence Summary

| Gate | Command | Result | Raw Evidence |
|------|---------|--------|-------------|
| nextest | `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_explicit_migration_skeleton_tests` | 51 passed, 0 skipped, 0 failed | Run 2026-05-27, 0.213s |
| clippy | `cargo clippy -p velvet-ballistics-workspace-tests --test restate_explicit_migration_skeleton_tests -- -D warnings` | No issues found | Run 2026-05-27 |
| proptest invariants | 19 proptest functions (included in 51 above) | All pass | Run 2026-05-27 |
| Kani harnesses | 7 harnesses in `verification/kani/` | VERIFIED_ADAPTER (State 5) | See formal-verification-report |
| fuzz targets | 4 targets in `fuzz/` | BUILT (campaigns not yet run) | See formal-verification-report |

---

## Artifact Inventory

| Artifact | Path | Lines | Status |
|----------|------|-------|--------|
| Test file | `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` | 1172 | ✅ Non-empty |
| Test registration | `crates/workspace_tests/Cargo.toml` | 157 | ✅ Registered |
| Black-hat review | `black-hat-review.md` | 309 | ✅ APPROVED |
| Verification ledger | `verification-ledger.jsonl` | 67 rows | ✅ Valid JSONL |
| State tracking | `STATE.md` | 108 | ✅ Current |
| Bead spec | `to-fix/09-restate-architecture-steal-plan.md` | 27 | ✅ Source |
| Production constants | `crates/vb_storage/src/constants.rs` | 89 | ✅ CURRENT_SCHEMA_VERSION = 1 |
| Production validation | `crates/vb_storage/src/codec/validation.rs` | 46 | ✅ MigrationRequired detection |
| Kani harnesses | `verification/kani/vb_aoah_*.rs` | 7 files | ✅ VERIFIED_ADAPTER |
| Fuzz targets | `verification/fuzz/` | 4 targets | ⚠️ BUILT, campaigns pending |

---

## Unresolved Gaps and Deferred Work

| ID | Description | Severity | Resolution |
|----|------------|----------|-----------|
| GAP-001 | Cleanup post-state emptiness not modeled | LOW | Add `assert_old_keyspace_is_empty_after_cleanup()` test after production wiring |
| GAP-002 | `test-writer-report.md` and `landing-report.md` contain stale cross-bead content | LOW | Will be overwritten in states 14-15 |
| DEFERRED-01 | Production `migrations.rs` does not exist | BLOCKING | Implement per STATE.md §State 12 Closure Requires |
| DEFERRED-02 | 9 of 17 error variants not yet exercised | EXPECTED | Awaiting production code |
| DEFERRED-03 | 4 fuzz campaigns not yet executed | EXPECTED | Awaiting production code |
| DEFERRED-04 | 7 Kani harnesses need production re-run | EXPECTED | After migrations.rs is written |

---

## Black-Hat Review Status

**Review:** `black-hat-review.md`  
**Verdict:** APPROVED (PENDING_PRODUCTION_WIRING)  
**Findings:** 0 critical, 3 non-blocking, 1 gap-tracked  
**GOD RULES:** All applicable rules pass (Verus excluded per reduced scope, GOD RULE 4 N/A — no production code)

---

## Formal Verification Status

**Report:** `formal-verification-report.md` (State 12)  
**Status:** PENDING_PRODUCTION_CLOSURE  
**Obligations:** 18 total (14 PASS_ADAPTER, 4 BUILT)  
**Kani:** 7/7 VERIFICATION SUCCESSFUL against adapters  
**Proptest:** All pass within 51-test suite  
**Fuzz:** 4 targets compiled, campaigns pending  

---

## Landing Gates

| Gate | Requirement | Status |
|------|------------|--------|
| nextest | 51/51 pass | ✅ |
| clippy | 0 warnings | ✅ |
| black-hat | APPROVED | ✅ |
| truth-serum | PASS (test-first scope) | ✅ |
| ledger | All 13 states appended | ✅ |
| production code | `migrations.rs` created | ❌ DEFERRED |
| moon ci | Canonical CI gate | ❌ DEFERRED |
| mutation testing | ≥95% kill rate | ❌ DEFERRED |

---

**Bundle prepared by:** evidence-packaging  
**Timestamp:** 2026-05-27T00:00:00Z  
**Schema version:** assurance-bundle/v1  
**Bundle status:** VALID (PENDING_PRODUCTION_CLOSURE)
