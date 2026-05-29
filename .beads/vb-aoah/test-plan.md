# Test Plan — vb-aoah Explicit Migration Skeleton Tests

## Provenance

- **Test planner**: test-planner (via proof-reviewer agent)
- **Invocation ID**: test-planner-vb-aoah-state8-001
- **Bead**: vb-aoah (migration skeleton tests)
- **State**: 8
- **Target test file**: `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`
- **Contract**: `.beads/vb-aoah/contract.md` (R1-R11)
- **Workflow model**: `.beads/vb-aoah/workflow-model.md` (3 workflows + edge case)
- **Error taxonomy**: `.beads/vb-aoah/error-taxonomy.md` (5 error families, 17 error variants)
- **Hazard analysis**: `.beads/vb-aoah/hazard-analysis.md` (7 hazard categories)
- **Proof obligations**: `proof-obligations.planned.jsonl` (18 obligations: 7 Kani + 7 proptest + 4 fuzz)
- **Bridge input**: `proof-to-implementation-input.md` (domain claims and source targets)
- **Workspace**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-aoah`
- **Date**: 2026-05-27

## Summary

- **Behaviors identified**: 22 (7 domain claims × 2-4 scenarios each + edge cases)
- **Trophy allocation**: 5 unit / 12 integration / 2 E2E / 3 static + proptest
- **Proptest invariants**: 7 (existing, to be hardened)
- **Fuzz targets**: 4 (existing, to be wired to production)
- **Kani harnesses**: 7 (existing, to be re-run against production)
- **Existing test file**: 156 lines, 7 proptest functions using adapters — to be migrated to production APIs

## 1. Behavior Inventory

### B1: Runtime Open Returns MigrationRequired (contract:R6)
Domain claim: Runtime open of an old supported store returns `MigrationRequired { from, to }` and performs no migration side effects.

### B2: Runtime Open Creates New Store (contract:R6)
Domain claim: Runtime open with no existing store metadata initializes a current-version store.

### B3: Runtime Open Reads Current Store (contract:R6)
Domain claim: Runtime open of a current-version store opens all keyspaces without migration.

### B4: Runtime Open Rejects Unsupported Future Version (contract:R6)
Domain claim: Runtime open of a future-version store returns `UnsupportedSchemaVersion`.

### B5: Migration Registry Totality (contract:R3)
Domain claim: Every supported old storage version maps to exactly one named migration entry.

### B6: Migration Registry Rejects Missing Entry (contract:R3)
Domain claim: A supported version with no registry entry returns `MissingMigrationRegistryEntry`.

### B7: Migration Registry Rejects Duplicates (contract:R3)
Domain claim: Ambiguous registry routes return `DuplicateMigrationRegistryEntry`.

### B8: Verify-Before-Advance (contract:R4)
Domain claim: Manifest/version advancement is impossible before verification succeeds.

### B9: Manifest Advance Rejected Without Verification (contract:R4)
Domain claim: Calling advance on an unverified migration returns `MigrationManifestAdvanceRejected`.

### B10: Manifest Advance Succeeds After Verification (contract:R4)
Domain claim: After successful verification, manifest advancement succeeds.

### B11: Cleanup Postcondition — Success Requires Empty Old Keyspace (contract:R5)
Domain claim: Cleanup-required migration reports success only after the old keyspace is empty.

### B12: Cleanup Postcondition — Non-Empty Rejects Success (contract:R5)
Domain claim: Cleanup attempt with non-empty old keyspace returns `MigrationCleanupFailed`.

### B13: Cleanup Postcondition — No-Cleanup Required Migration (contract:R5)
Domain claim: Migration with no cleanup requirement skips cleanup and can advance directly.

### B14: Reopen After Migration Is Idempotent (contract:R7)
Domain claim: Reopen after successful migration reads current records without invoking migration hooks or counters.

### B15: Reopen After Migration Does Not Rerun Migration (contract:R7)
Domain claim: Migration run counter does not increment on reopen of a current-version store.

### B16: Empty Old Keyspace Returns Explicit NoOp (contract:R9)
Domain claim: Empty old-keyspace behavior is explicit no-op — either manifest unchanged or explicit no-op evidence recorded.

### B17: Empty Old Keyspace Cannot Claim Unverified Migration (contract:R9)
Domain claim: Empty old-keyspace must not silently advance manifest or claim verification.

### B18: Bounded Accounting — Checked Addition (contract:R11)
Domain claim: Record count and byte accounting use checked arithmetic that returns typed errors on overflow.

### B19: Bounded Accounting — Overflow Returns Error (contract:R11)
Domain claim: Counter overflow (u64::MAX) returns `MigrationBatchLimitExceeded`, not wrapped success.

### B20: Bounded Accounting — Batch Size Limits (contract:R11)
Domain claim: Migration batches respect named batch size and byte limits.

### B21: Manifest Version Update Gates All Paths (contract:R4,R5,R8)
Domain claim: Manifest version update is reachable only through verified, cleaned, committed paths — never through error or skip paths.

### B22: Runtime Open Never Invokes Migration Cold Path (contract:R6,hazard:Temporal)
Domain claim: Runtime open must never invoke copy, cleanup, verify, or manifest advance — it is a detection-only path.

## 2. Trophy Allocation

| Layer | Count | Behaviors | Rationale |
|---|---|---|---|
| Unit (Calc) | 5 | B5, B6, B7, B18, B19 | Pure logic: registry lookup, checked arithmetic. No I/O, no persistence. |
| Integration | 12 | B1, B2, B3, B4, B8, B9, B10, B11, B12, B14, B15, B16, B17, B21, B22 | Real FjallJournal + migrations.rs interactions. Test state, not interactions. |
| E2E (CLI) | 2 | B1, B3 | CLI-level `vb migrate status` / `vb open` commands that exercise the full stack. |
| Proptest | 7 | All domain claims | Property-based strategies: exhaustive combinatorial over version ranges, record counts, byte limits, phase states. |
| Fuzz | 4 | B1 (hostile manifest), B11 (corrupt keyspace), B16 (malformed empty fixture), B18 (boundary overflow) | Hostile byte-level inputs at codec/manifest boundaries. |
| Kani | 7 | B1, B5, B8, B11, B14, B16, B18 | Bounded model checking: panic-freedom, overflow-freedom, typestate correctness, idempotence. |

**Trophy ratio**: ~23% unit (5/22), ~55% integration (12/22), ~9% E2E (2/22), ~14% static/proptest (3 static categories). Integration layer is heaviest — this bead's primary risk is behavioral correctness at the storage component boundary.

## 3. BDD Scenarios

### Behavior B1: Runtime Open Returns MigrationRequired

```
### Scenario: Runtime open detects old supported store and returns MigrationRequired without side effects
Given: a store on disk with schema version 0 (supported old version)
  And: the store has old-keyspace records and manifest at version 0
When: vb_storage::open_store(path) is called via runtime
Then: the result is Err(JournalError::MigrationRequired { from: 0, to: CURRENT_SCHEMA_VERSION })
  And: the old keyspace still contains its original records (no copy happened)
  And: the manifest is unchanged (still at version 0)
  And: no new migration records exist in the current keyspace
```

```
### Scenario: Runtime open on new path initializes current-version store
Given: a clean/empty directory with no store metadata
When: vb_storage::open_store(path) is called via runtime
Then: the result is Ok with a handle to a new current-version store
  And: the store manifest reports CURRENT_SCHEMA_VERSION
  And: declared keyspaces exist and are empty
```

```
### Scenario: Runtime open on current-version store returns Ok without migration
Given: a store at CURRENT_SCHEMA_VERSION with populated keyspaces
When: vb_storage::open_store(path) is called via runtime
Then: the result is Ok with a handle to the existing store
  And: migration run counter is unchanged
  And: all existing records are readable
```

```
### Error: Runtime open on future/unsupported version returns UnsupportedSchemaVersion
Given: a store with schema version > CURRENT_SCHEMA_VERSION (future/unsupported)
When: vb_storage::open_store(path) is called via runtime
Then: the result is Err(JournalError::UnsupportedSchemaVersion { version: >CURRENT_SCHEMA_VERSION })
  And: the store is not mutated
```

### Behavior B5: Migration Registry Totality

```
### Scenario: Lookup returns exactly one named migration per supported old version
Given: a MigrationRegistry containing entries for all supported old versions
When: MigrationRegistry::lookup(version) is called for each supported version
Then: each call returns Ok(MigrationAction) with a distinct, non-empty name
  And: no two supported versions map to the same lookup pathway name with the same semantics
```

```
### Error: Missing registry entry returns typed error
Given: a MigrationRegistry that is missing an entry for a supported old version V
When: MigrationRegistry::lookup(V) is called
Then: the result is Err(MigrationError::MissingMigrationRegistryEntry { from: V, to: CURRENT_SCHEMA_VERSION })
```

```
### Error: Duplicate registry entry returns typed error
Given: a MigrationRegistry with two entries mapping the same from-version
When: MigrationRegistry is constructed or validated
Then: the construction or validation returns Err(MigrationError::DuplicateMigrationRegistryEntry { from, to })
```

### Behavior B8: Verify-Before-Advance

```
### Scenario: Manifest advance is impossible before verification succeeds
Given: a migration in Copied phase (records copied but not verified)
When: advance_manifest(phase) is called
Then: the result is Err(MigrationError::MigrationManifestAdvanceRejected { phase: Copied })
  And: the manifest version remains at the old version
  And: the MigrationPhase remains Copied (not advanced)
```

```
### Scenario: Manifest advance succeeds after verification and cleanup
Given: a migration in Verified phase with cleanup completed
When: advance_manifest(phase) is called
Then: the result is Ok(Committed)
  And: the manifest version is updated to CURRENT_SCHEMA_VERSION
```

```
### Scenario: Manifest advance succeeds after verification when no cleanup is needed
Given: a migration in Verified phase with no cleanup required
When: advance_manifest(phase) is called
Then: the result is Ok(Committed)
  And: no cleanup operations were performed
```

```
### Error: Verifying already-committed migration is rejected
Given: a migration already in Committed phase
When: verify_records() or advance_manifest() is called
Then: the operation is rejected with an appropriate typed error
```

### Behavior B11: Cleanup Postcondition

```
### Scenario: Cleanup-required migration reports success after emptying old keyspace
Given: a migration with cleanup_required = true
  And: the old keyspace contains records
When: cleanup_old_keyspace() is called and all old records are deleted
Then: the result is Ok(CleanupOutcome::Success)
  And: old_keyspace_is_empty() returns true
  And: cleanup count equals the original old record count
```

```
### Error: Cleanup with non-empty old keyspace returns typed error
Given: a migration with cleanup_required = true
  And: the old keyspace still contains records after a cleanup attempt
When: cleanup_old_keyspace() is called but deletion is incomplete
Then: the result is Err(MigrationError::MigrationCleanupFailed { remaining_count: >0 })
  And: old_keyspace_is_empty() returns false
```

```
### Scenario: No-cleanup-required migration skips cleanup and can advance
Given: a migration with cleanup_required = false
When: the migration workflow transitions from Verified to ManifestAdvanced
Then: no cleanup_old_keyspace() call is made
  And: advance succeeds
```

### Behavior B14: Reopen After Migration Is Idempotent

```
### Scenario: Reopen after successful migration reads current records without invoking migration
Given: a store that has been successfully migrated (version = CURRENT_SCHEMA_VERSION)
  And: the migration run counter is at N (N >= 1)
When: vb_storage::open_store(path) is called
Then: the result is Ok with access to all current records
  And: migration run counter is still N (not N+1)
  And: no migration hooks/registry were invoked
  And: is_current_version() returns true
```

```
### Scenario: Reopen after migration does not rerun migration cold path
Given: the store is at CURRENT_SCHEMA_VERSION with current records loaded
When: migration registry is inspected
Then: no migration entry was invoked during this open
  And: migration counters are untouched
```

### Behavior B16: Empty Old Keyspace Returns Explicit NoOp

```
### Scenario: Migration from empty old keyspace produces explicit NoOp
Given: a store where the old keyspace contains zero records
When: migrate_from(old_version, current_version) is called
Then: the result is Ok(MigrationOutcome::NoOp)
  And: the manifest is either unchanged or contains explicit no-op evidence
  And: no new records were written to current keyspace
```

```
### Scenario: Empty old keyspace cannot silently claim unverified migration
Given: an empty old keyspace and a migration that produced NoOp
When: the manifest is inspected
Then: it does NOT show Committed or Verified phase
  And: migration run counter is either 0 or explicitly reflects the no-op
```

### Behavior B18: Bounded Accounting — Checked Addition

```
### Scenario: Checked arithmetic succeeds within bounds
Given: current_record_count = 100, delta = 50, max_records = 200
When: checked_add_records(current_record_count, delta, max_records)
Then: the result is Ok(150)
```

```
### Error: Checked arithmetic returns error on overflow
Given: current_record_count = u64::MAX - 10, delta = 20, max_records = u64::MAX
When: checked_add_records(current_record_count, delta, max_records)
Then: the result is Err(MigrationError::MigrationBatchLimitExceeded { limit: max_records })
```

```
### Error: Checked arithmetic returns error on limit exceeded (no overflow)
Given: current_record_count = 199, delta = 2, max_records = 200
When: checked_add_records(current_record_count, delta, max_records)
Then: the result is Err(MigrationError::MigrationBatchLimitExceeded { limit: 200 })
```

### Behavior B21: Manifest Version Update Gates All Paths

```
### Scenario: Manifest cannot be updated via error path
Given: a migration that failed at any phase (copy, verify, cleanup)
When: any operation is called after the failure
Then: the manifest version remains at the old version
  And: the store is not opened as current-version
```

```
### Scenario: Manifest cannot be updated via skip path
Given: a migration that skipped verification
When: advance_manifest is called
Then: it returns MigrationManifestAdvanceRejected
```

### Behavior B22: Runtime Open Never Invokes Migration Cold Path

```
### Scenario: Runtime open is detection-only, never invokes copy/cleanup/verify/advance
Given: any supported old-version store
When: vb_storage::open_store(path) is called
Then: the result is Err(MigrationRequired) — never Ok
  And: copy, cleanup, verify, and advance_manifest functions were never called
  And: no migration side effects are observable in the store
```

## 4. Proptest Invariants

### 4.1 Existing Proptest Tests (to be hardened)

The file `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` contains 7 proptest tests currently using `state7_*_adapter` functions. These must be updated per the test plan below. Known weaknesses from bridge review finding BR-F-002:

| Test Function | Weakness | Hardening Required |
|---|---|---|
| `vb_aoah_verify_before_manifest_advance` | Only tests when `version == CURRENT_SCHEMA_VERSION` | Must also test old versions where verification fails |
| `vb_aoah_empty_old_keyspace_explicit_noop` | Tautology at line 142: `prop_assert!(f.old_records > 0)` | Replace with assertion on explicit NoOp outcome |
| `vb_aoah_migration_accounting_overflow_returns_error` | Uses u16 casts on u8 values — cannot overflow | Expand bounds to u64 and test actual limit-exceeded paths |

### 4.2 New Proptest Invariants

```
### Proptest: MigrationRegistry lookup idempotence
Invariant: lookup(v) returns the same result for the same version v across repeated calls.
Strategy: any version 0..CURRENT_SCHEMA_VERSION, any registry state with bounded entry count.
```

```
### Proptest: Cleanup outcome determinism
Invariant: cleanup_old_keyspace(old_records: u64, cleanup_result: bool) —
  if old_records == 0, cleanup_count == 0;
  if old_records > 0 and cleanup_result == true, cleanup_count == old_records.
Strategy: any u64 record count bounded to [0, MAX_U64/8], any boolean cleanup result.
```

```
### Proptest: Manifest version monotonicity
Invariant: The manifest version must never decrease and never skip from old to current
  without passing through all required phase transitions.
Strategy: any MigrationPhase enum value, any valid/invalid transition attempt.
```

```
### Proptest: Record count consistency
Invariant: total_migrated_records + remaining_old_records == original_old_records
  after any migration that copies records.
Strategy: any u64 record counts with bounded totals.
```

```
### Proptest: No side effects from migration detection
Invariant: Calling detect_old_store(version) must not mutate any persisted state.
Strategy: any u16 version, with before/after state snapshots.
```

## 5. Fuzz Targets

### 5.1 Existing Fuzz Targets

| Fuzz Target | Boundary | Contract | Status |
|---|---|---|---|
| `vb_aoah_runtime_open_hostile_manifest` | Postcard codec → runtime open | R6 | Built, pending wire to production |
| `vb_aoah_cleanup_corrupt_old_keyspace` | Postcard codec → cleanup | R5 | Built, pending wire to production |
| `vb_aoah_empty_keyspace_malformed_input` | Postcard/manifest boundary → NoOp detection | R9 | Built, pending wire to production |
| `vb_aoah_migration_accounting_boundary_overflow` | Codec → checked arithmetic boundary | R11 | Built, pending wire to production |

All 4 fuzz targets must be wired to production migration code after `migrations.rs` is implemented. Each must:
- Exercise hostile byte sequences through the Postcard codec boundary
- Assert that all failures return typed errors (no panics, no unwrap, no expect)
- Include corpus seeds for: empty input, valid minimum, boundary+1, repeated delimiters, unicode confusion, version byte corruption

### 5.2 Fuzz Target Command Evidence

```bash
cargo fuzz run vb_aoah_runtime_open_hostile_manifest -- -max_total_time=60 -runs=10000
cargo fuzz run vb_aoah_cleanup_corrupt_old_keyspace -- -max_total_time=60 -runs=10000
cargo fuzz run vb_aoah_empty_keyspace_malformed_input -- -max_total_time=60 -runs=10000
cargo fuzz run vb_aoah_migration_accounting_boundary_overflow -- -max_total_time=60 -runs=10000
```

## 6. Kani Harnesses

### 6.1 Existing Kani Harnesses (to be re-run against production)

| Kani Harness | Obligation | Domain Claim | Status |
|---|---|---|---|
| `vb_aoah_runtime_open_no_side_effects` | PO-R01 | B1 — Runtime open returns MigrationRequired without side effects | VERIFIED (adapter), needs re-run |
| `vb_aoah_migration_registry_totality` | PO-R02 | B5 — Registry totality and uniqueness | VERIFIED (adapter), needs re-run |
| `vb_aoah_verify_before_manifest_advance` | PO-R03 | B8 — Manifest cannot advance before verification | VERIFIED (adapter), needs re-run |
| `vb_aoah_cleanup_success_requires_empty_old_keyspace` | PO-R04 | B11 — Cleanup success requires empty old keyspace | VERIFIED (adapter), needs re-run |
| `vb_aoah_reopen_after_migration_no_rerun` | PO-R05 | B14 — Reopen does not rerun migration | VERIFIED (adapter), needs re-run |
| `vb_aoah_empty_old_keyspace_noop` | PO-R06 | B16 — Empty keyspace produces explicit NoOp | VERIFIED (adapter), needs re-run |
| `vb_aoah_migration_accounting_checked_bounds` | PO-R07 | B18 — Bounded arithmetic cannot overflow into success | VERIFIED (adapter), needs re-run |

All 7 harnesses passed with `VERIFICATION:- SUCCESSFUL` and `0 failures` per raw evidence in `.beads/vb-aoah/raw-evidence/attempt8/kani-vb_aoah_all_harnesses.log`. After production `migrations.rs` is implemented, all harnesses must be re-run against production code and must continue to show `VERIFICATION:- SUCCESSFUL`.

### 6.2 Kani Harness Command Evidence

```bash
cargo kani -p vb_storage --harness vb_aoah_runtime_open_no_side_effects --output-format terse
cargo kani -p vb_storage --harness vb_aoah_migration_registry_totality --output-format terse
cargo kani -p vb_storage --harness vb_aoah_verify_before_manifest_advance --output-format terse
cargo kani -p vb_storage --harness vb_aoah_cleanup_success_requires_empty_old_keyspace --output-format terse
cargo kani -p vb_storage --harness vb_aoah_reopen_after_migration_no_rerun --output-format terse
cargo kani -p vb_storage --harness vb_aoah_empty_old_keyspace_noop --output-format terse
cargo kani -p vb_storage --harness vb_aoah_migration_accounting_checked_bounds --output-format terse
```

### 6.3 Kani Model Bounds

- Storage versions: u16 values, supported-old set explicitly enumerated
- Record counts: bounded to MAX_RECORDS=8 (skeleton), u64 in production
- Byte totals: bounded to MAX_BYTES=64 (skeleton), u64 in production
- Unwinding: `#[kani::unwind(3)]` for current bounded loops
- Arbitrary: All harnesses use `kani::Arbitrary` per GOD RULE
- These are test-first skeleton constraints. Production bounds must be reviewed at State 12.

## 7. Mutation Checkpoints

### Critical Mutations to Survive

| Mutation | Must Be Caught By | Rationale |
|---|---|---|
| `detect_old_store` returns `None` for supported version | `vb_aoah_runtime_open_migration_required_no_side_effects` (proptest) + Kani PO-R01 | Runtime open must always detect old stores |
| `advance_manifest` condition `if phase == Verified` changed to `if phase == Planned` | `vb_aoah_verify_before_manifest_advance` (proptest) + Kani PO-R03 | Must guard against premature advancement |
| `cleanup_old_keyspace` returns `Success` without checking emptiness | `vb_aoah_cleanup_empty_old_keyspace_postcondition` (proptest) + Kani PO-R04 | Cleanup must verify emptiness |
| `is_current_version` returns `true` for old version | `vb_aoah_reopen_after_migration_idempotent` (proptest) + Kani PO-R05 | Reopen must not trigger migration |
| `checked_add_records` wraps instead of returning error | `vb_aoah_migration_accounting_overflow_returns_error` (proptest) + Kani PO-R07 | Arithmetic must be checked |
| Runtime open invokes `migrate_from` | `vb_aoah_runtime_open_migration_required_no_side_effects` (proptest) + Kani PO-R01 | Runtime must be detection-only |
| Registry lookup returns `Ok` for unsupported version | `vb_aoah_migration_registry_totality_uniqueness` (proptest) + Kani PO-R02 | Registry must reject unknowns |
| Empty keyspace branch falls through to copy logic | `vb_aoah_empty_old_keyspace_explicit_noop` (proptest) + Kani PO-R06 | Empty must be explicit NoOp |

### Mutation Threshold Target

**95% kill rate** minimum on `crates/vb_storage/src/migrations.rs` and the migration integration points in `crates/vb_storage/src/journal/core.rs`.

## 8. Combinatorial Coverage Matrix

### Unit Tests — Migration Registry (B5, B6, B7)

| Scenario | Input Version | Expected Output | Test Layer |
|---|---|---|---|
| Happy: known supported version | 0 (supported old) | Ok(MigrationAction { name: "..." }) | unit |
| Happy: another supported version | 1 (if pre-current) | Ok(MigrationAction { name: "..." }) | unit |
| Error: missing entry for supported version | Supported version with no entry | Err(MissingMigrationRegistryEntry) | unit |
| Error: duplicate entry | Supported version with 2+ entries | Err(DuplicateMigrationRegistryEntry) | unit |
| Error: future version | CURRENT_SCHEMA_VERSION + 1 | Err(UnsupportedMigrationSource) | unit |
| Error: current version | CURRENT_SCHEMA_VERSION | Err(UnsupportedMigrationSource) | unit |
| Bound: u16::MAX version | u16::MAX | Err(UnsupportedMigrationSource) | unit |
| Bound: zero version | 0 | Ok or specific typed error | unit |

### Integration Tests — Runtime Open (B1, B2, B3, B4)

| Scenario | Store State | Expected Output | Test Layer |
|---|---|---|---|
| Old supported store | Version 0, populated | Err(MigrationRequired) | integration |
| New store init | No metadata | Ok, version = CURRENT | integration |
| Current store | Version = CURRENT | Ok, records readable | integration |
| Future store | Version > CURRENT | Err(UnsupportedSchemaVersion) | integration |
| Corrupt manifest | Garbled metadata | Err(ManifestCorrupt) | integration |
| Side-effect check | Old store, check keyspaces | Old keyspace unchanged | integration |
| Runtime never invokes cold path | Old store, instrumented | copy/cleanup/verify never called | integration |

### Integration Tests — Verify-Before-Advance (B8, B9, B10)

| Scenario | Phase | Expected Output | Test Layer |
|---|---|---|---|
| Advance from Copied | Copied | Err(MigrationManifestAdvanceRejected) | integration |
| Advance from Cleaned (unverified) | Cleaned (skipped verify) | Err(MigrationManifestAdvanceRejected) | integration |
| Advance from Verified (cleanup done) | Verified | Ok(Committed) | integration |
| Advance from Verified (no cleanup) | Verified | Ok(Committed) | integration |
| Advance from Committed | Committed | Ok(idempotent) or typed rejection | integration |
| Manifest version after rejection | Any failed advance | Stays at old version | integration |

### Integration Tests — Cleanup Postcondition (B11, B12, B13)

| Scenario | Old Keyspace State | Expected Output | Test Layer |
|---|---|---|---|
| Clean old keyspace with records | Old records > 0, deletion succeeds | Ok(Success), count matches | integration |
| Clean with remaining records | Old records > 0, deletion incomplete | Err(MigrationCleanupFailed) | integration |
| Clean already-empty keyspace | Old records = 0 | Ok(NoCleanupNeeded) or Success(0) | integration |
| No-cleanup migration skip | cleanup_required = false | Ok, skipped, advance allowed | integration |
| Clean then verify emptiness | After cleanup | old_keyspace_is_empty() = true | integration |

### Integration Tests — Reopen Idempotence (B14, B15)

| Scenario | Post-Migration State | Expected Output | Test Layer |
|---|---|---|---|
| Reopen current store | Version = CURRENT, records | Ok, records readable | integration |
| Migration counter unchanged | Counter at N | Counter stays at N | integration |
| Migration hooks untouched | hooks instrumented | Zero invocations | integration |
| Reopen after failed migration | Version = old | Err(MigrationRequired) | integration |

### Integration Tests — Empty Keyspace NoOp (B16, B17)

| Scenario | Old Keyspace State | Expected Output | Test Layer |
|---|---|---|---|
| Empty old keyspace | Old records = 0 | Ok(NoOp) | integration |
| Manifest after NoOp | After NoOp outcome | Unchanged or no-op evidence | integration |
| NoOp cannot advance manifest | After NoOp | Manifest still old version | integration |
| NoOp cannot claim verified | After NoOp | Phase != Verified | integration |

### Integration Tests — Bounded Accounting (B18, B19, B20)

| Scenario | Current/Delta/Limit | Expected Output | Test Layer |
|---|---|---|---|
| Within limits | curr=100, delta=50, limit=200 | Ok(150) | unit |
| At limit | curr=200, delta=0, limit=200 | Ok(200) | unit |
| At limit + delta | curr=199, delta=2, limit=200 | Err(BatchLimitExceeded) | unit |
| Overflow: u64::MAX + 1 | u64::MAX, delta=1 | Err(BatchLimitExceeded) | unit |
| Overflow: u64::MAX + u64::MAX | u64::MAX, delta=u64::MAX | Err(BatchLimitExceeded) | unit |
| Zero delta | curr=100, delta=0 | Ok(100) | unit |

## 9. Test File Migration Plan

### Current State

`crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` (156 lines) contains:
- 7 proptest functions using `state7_*_adapter` test doubles
- 1 `Fixture` struct and `fixtures()` strategy
- Adapter functions modeling expected migration behavior

### Required Changes Before State 12

1. **Replace adapters with production API calls**:
   - `state7_supported_old_adapter` → `migrations::detect_old_store(version)`
   - `state7_registry_entry_adapter` → `MigrationRegistry::lookup(version)`
   - `state7_runtime_open_adapter` → `vb_storage::open_store(path)`
   - `state7_checked_accounting_adapter` → `checked_add_records/bytes()`
   - `state7_advance_manifest_adapter` → `advance_manifest(phase)`
   - `state7_cleanup_success_adapter` → `cleanup_old_keyspace(journal, old_version)`
   - `state7_reopen_runs_adapter` → migration counter inspection after reopen

2. **Harden weak assertions** (per BR-F-002):
   - `vb_aoah_verify_before_manifest_advance`: Add `version != CURRENT_SCHEMA_VERSION` path
   - `vb_aoah_empty_old_keyspace_explicit_noop`: Replace tautology with NoOp outcome assertion
   - `vb_aoah_migration_accounting_overflow_returns_error`: Expand bounds to u64 for overflow testing

3. **Add new test scenarios**:
   - `vb_aoah_runtime_open_future_version_rejected` (B4)
   - `vb_aoah_runtime_open_new_store_initializes` (B2)
   - `vb_aoah_registry_rejects_missing_entry` (B6)
   - `vb_aoah_registry_rejects_duplicate_entry` (B7)
   - `vb_aoah_advance_manifest_succeeds_after_verify_and_cleanup` (B10)
   - `vb_aoah_cleanup_no_cleanup_required_skips` (B13)
   - `vb_aoah_reopen_counter_unchanged` (B15)
   - `vb_aoah_empty_noop_cannot_claim_verified` (B17)
   - `vb_aoah_manifest_gates_all_paths` (B21)
   - `vb_aoah_runtime_open_never_invokes_cold_path` (B22)

### Required File Structure After State 12

```
crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs
├── Use statements: vb_storage::migrations, vb_storage::constants, proptest
├── Fixture struct + strategy (updated for production bounds)
├── Proptest 7: existing 7 (hardened) + up to 10 new tests
├── Unit tests (non-proptest): registry, checked arithmetic
└── Helper functions: create_test_store(), assert_no_side_effects()
```

## 10. Error Variant Coverage

Every error variant from `error-taxonomy.md` must have at least one explicit test scenario:

| Error Variant | Test Scenario | Layer |
|---|---|---|
| `MigrationRequired { from, to }` | B1 scenario 1 | integration + proptest |
| `UnsupportedSchemaVersion { version }` | B4 error | integration + proptest |
| `UnsupportedMigrationSource { from, to }` | B5 error (future version lookup) | unit + proptest |
| `MissingMigrationRegistryEntry { from, to }` | B6 error | unit + proptest |
| `DuplicateMigrationRegistryEntry { from, to }` | B7 error | unit + proptest |
| `MigrationManifestMissing` | Runtime open corrupt store | integration |
| `MigrationManifestCorrupt { reason_code }` | B1 corrupt manifest path | integration |
| `MigrationManifestAdvanceRejected { from, to, phase }` | B9 error | integration + proptest |
| `MigrationReadFailed { keyspace }` | Copy path failure | integration |
| `MigrationWriteFailed { keyspace }` | Rewrite path failure | integration |
| `MigrationRecordDecodeFailed { record_kind }` | Old record decode failure | integration |
| `MigrationRecordEncodeFailed { record_kind }` | New record encode failure | integration |
| `MigrationBatchLimitExceeded { limit }` | B19, B20 error | unit + proptest + Kani |
| `MigrationVerificationFailed { reason_code, checked_count }` | Verify path failure | integration |
| `MigrationMissingNewRecord { record_kind }` | Post-copy verification | integration |
| `MigrationUnexpectedNewRecord { record_kind }` | Extra record in current keyspace | integration |
| `MigrationCleanupFailed { keyspace }` | B12 error | integration + proptest |
| `MigrationCleanupVerificationFailed { remaining_count }` | Post-cleanup emptiness check | integration |

## 11. Hazard Mitigation Verification

| Hazard Category | Test Coverage | How Verified |
|---|---|---|
| Temporal: manifest advanced before verification | B8, B9, B21 | Kani PO-R03 + proptest PO-R10 — proves manifest cannot advance before verify |
| Temporal: runtime open performs implicit migration | B1, B22 | Kani PO-R01 + proptest PO-R08 — proves runtime is detection-only |
| Temporal: cleanup before verification | B11, B12 | Kani PO-R04 + proptest PO-R11 — cleanup only after verified state |
| Temporal: reopen accidentally reruns migration | B14, B15 | Kani PO-R05 + proptest PO-R12 — proves idempotence |
| Rust-core: primitive u16 version comparisons | B5, B6, B7 | Typed Registry + lookup enforces version safety |
| Rust-core: boolean lifecycle flags | B8, B10 | Typestate MigrationPhase enum — illegal states unrepresentable |
| Rust-core: unchecked counts overflow | B18, B19, B20 | Kani PO-R07 + proptest PO-R14 — checked arithmetic only |
| Bounded state: unbounded iteration | B20 | Batch size/bound checks in Kani PO-R07 |
| Persistence: keyspace manifest collision | B1 | Test verifies current keyspace count/names unchanged |
| Hostile input: corrupt fixture accepted | Fuzz PO-R15, PO-R16, PO-R17 | 3 fuzz targets exercise hostile inputs |
| Hostile input: future schema mistaken for old | B4 | fuzz target exercises boundary versions |
| Verification: count-only verification | B8 | Kani PO-R03 covers manifest ordering; content verification is future work |

## 12. Execution Gates

### Pre-State-12 (Current)
- `cargo kani -p vb_storage --harness vb_aoah_*` — must compile and pass with adapter functions
- `cargo fuzz build --target x86_64-unknown-linux-gnu` — all 4 targets must compile
- `rustfmt --check crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` — must pass

### State 12 (Post-Implementation)
- `cargo kani -p vb_storage` — all 7 harnesses must pass against production code
- `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_explicit_migration_skeleton_tests` — all proptest tests must pass
- `cargo fuzz run vb_aoah_* -- -max_total_time=60 -runs=10000` — all 4 fuzz campaigns must complete clean
- `cargo mutants -p vb_storage -- --test restate_explicit_migration_skeleton_tests` — ≥95% kill rate
- `moon ci` — canonical CI gate must pass

## 13. Test Execution Commands

```bash
# Proptest (all 7+ migration skeleton tests)
cargo nextest run -p velvet-ballistics-workspace-tests \
  --test restate_explicit_migration_skeleton_tests

# Proptest (individual)
cargo nextest run -p velvet-ballistics-workspace-tests \
  --test restate_explicit_migration_skeleton_tests \
  -- vb_aoah_runtime_open_migration_required_no_side_effects

# Kani (all 7 harnesses)
for harness in \
  vb_aoah_runtime_open_no_side_effects \
  vb_aoah_migration_registry_totality \
  vb_aoah_verify_before_manifest_advance \
  vb_aoah_cleanup_success_requires_empty_old_keyspace \
  vb_aoah_reopen_after_migration_no_rerun \
  vb_aoah_empty_old_keyspace_noop \
  vb_aoah_migration_accounting_checked_bounds; do
  cargo kani -p vb_storage --harness "$harness" --output-format terse
done

# Fuzz (all 4 campaigns)
for target in \
  vb_aoah_runtime_open_hostile_manifest \
  vb_aoah_cleanup_corrupt_old_keyspace \
  vb_aoah_empty_keyspace_malformed_input \
  vb_aoah_migration_accounting_boundary_overflow; do
  cargo fuzz run "$target" -- -max_total_time=60 -runs=10000
done

# Mutation testing
cargo mutants -p vb_storage -- --test restate_explicit_migration_skeleton_tests

# Canonical CI
moon ci
```

## Proof/Refinement Coverage Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|--------------------|------------------|---------------------|------------------------|----------|------------------|------------|
| PO-R01 | runtime_open_result no side effects | Yes | `validation.rs:10-17` | B1-B4 (L2/L6) | `vb_aoah_runtime_open_no_side_effects.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_runtime_open_no_side_effects` | State 5 |
| PO-R02 | MigrationRegistry lookup totality | Yes | `migrations.rs` (planned) | B5-B7 (L1/L2) | `vb_aoah_migration_registry_totality.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_registry_totality` | State 5 |
| PO-R03 | verify_before_manifest_advance | Yes | `migrations.rs` (planned) | B8-B10 (L2) | `vb_aoah_verify_before_manifest_advance.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_verify_before_manifest_advance` | State 5 |
| PO-R04 | cleanup requires empty old keyspace | Yes | `migrations.rs` (planned) | B11-B13 (L2) | `vb_aoah_cleanup_success_requires_empty_old_keyspace.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_cleanup_success_requires_empty_old_keyspace` | State 5 |
| PO-R05 | reopen after migration no rerun | Yes | `migrations.rs` (planned) | B14-B15 (L2/L3) | `vb_aoah_reopen_after_migration_no_rerun.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_reopen_after_migration_no_rerun` | State 5 |
| PO-R06 | empty old keyspace noop | Yes | `migrations.rs` (planned) | B16-B17 (L2) | `vb_aoah_empty_old_keyspace_noop.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_empty_old_keyspace_noop` | State 5 |
| PO-R07 | migration accounting checked bounds | Yes | `migrations.rs` (planned) | B18-B20 (L1) | `vb_aoah_migration_accounting_checked_bounds.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_accounting_checked_bounds` | State 5 |
| PO-R08 | proptest runtime open no side effects | Yes | `migrations.rs` (planned) | B1-B4 (L2) | N/A | proptest | `cargo test vb_aoah_runtime_open_migration_required_no_side_effects` | State 9 |
| PO-R09 | proptest registry totality uniqueness | Yes | `migrations.rs` (planned) | B5-B7 (L1/L2) | N/A | proptest | `cargo test vb_aoah_migration_registry_totality_uniqueness` | State 9 |
| PO-R10 | proptest verify before advance | Yes | `migrations.rs` (planned) | B8-B10 (L2) | N/A | proptest | `cargo test vb_aoah_verify_before_manifest_advance` | State 9 |
| PO-R11 | proptest cleanup postcondition | Yes | `migrations.rs` (planned) | B11-B13 (L2) | N/A | proptest | `cargo test vb_aoah_cleanup_empty_old_keyspace_postcondition` | State 9 |
| PO-R12 | proptest reopen idempotent | Yes | `migrations.rs` (planned) | B14-B15 (L2) | N/A | proptest | `cargo test vb_aoah_reopen_after_migration_idempotent` | State 9 |
| PO-R13 | proptest empty keyspace explicit noop | Yes | `migrations.rs` (planned) | B16-B17 (L2) | N/A | proptest | `cargo test vb_aoah_empty_old_keyspace_explicit_noop` | State 9 |
| PO-R14 | proptest overflow returns error | Yes | `migrations.rs` (planned) | B18-B20 (L1/L2) | N/A | proptest | `cargo test vb_aoah_migration_accounting_overflow_returns_error` | State 9 |
| PO-R15 | fuzz hostile manifest | No (defense-in-depth) | `migrations.rs` (planned) | `fuzz_targets/vb_aoah_runtime_open_hostile_manifest.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_runtime_open_hostile_manifest` | State 5 |
| PO-R16 | fuzz corrupt old keyspace | No (defense-in-depth) | `migrations.rs` (planned) | `fuzz_targets/vb_aoah_cleanup_corrupt_old_keyspace.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_cleanup_corrupt_old_keyspace` | State 5 |
| PO-R17 | fuzz malformed input | No (defense-in-depth) | `migrations.rs` (planned) | `fuzz_targets/vb_aoah_empty_keyspace_malformed_input.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_empty_keyspace_malformed_input` | State 5 |
| PO-R18 | fuzz boundary overflow | No (defense-in-depth) | `migrations.rs` (planned) | `fuzz_targets/vb_aoah_migration_accounting_boundary_overflow.rs` | N/A | cargo-fuzz | `cargo fuzz run vb_aoah_migration_accounting_boundary_overflow` | State 5 |


## Open Questions

1. **Manifest storage location**: Where is the manifest persisted? This affects test fixture setup for integration tests that need to seed old-version manifests.
2. **CLI command syntax**: What is the CLI surface for explicit migration? Affects E2E test design.
3. **Empty no-op policy**: Should empty old keyspace leave manifest unchanged or record explicit no-op evidence? Affects B16/ B17 assertions.
4. **Content verification scope**: Current verification plan covers record counts and ordering. Content/digest-level verification is future work — needs explicit scope.
5. **Production model bounds**: Current Kani bounds (MAX_RECORDS=8, MAX_BYTES=64, u8) are skeleton. What are production bounds? Affects Kani unwinding and proptest strategy generation.
6. **Error diagnostic codes**: New migration error variants need diagnostic code assignments in `error/codes.rs`. What code range is allocated?
7. **Manifest keyspace collision**: Current tests assert exactly 9 declared keyspaces. Adding a manifest keyspace will break `restate_fjall_keyspace_manifest_tests.rs`. Coordination required.
