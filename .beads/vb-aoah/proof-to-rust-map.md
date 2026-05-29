# Proof-to-Rust Bridge Map — vb-aoah State 7

## Provenance

- **Bead**: vb-aoah (migration skeleton tests)
- **Pipeline state**: 7 (proof-to-implementation bridge)
- **Proof reviewer**: proof-reviewer-vb-aoah-state5-001 (APPROVED)
- **Input**: proof-obligations.planned.jsonl (18 reduced-scope obligations)
- **Date**: 2026-05-27

## Bridge Overview

18 proof obligations spanning 3 verifiers (Kani, proptest, cargo-fuzz) across 6 domain claim clusters. All production migration code is **pending** (test-first bead). This bridge maps proof claims to planned production source symbols, existing infrastructure, explicit behavior test refs, refinement harness refs, and evidence commands. Current mapping_status is `planned` for all rows; State 12 requires `materialized` or `verified` closure.

### Production Code Gap

The bead contract requires a production migration API at `crates/vb_storage/src/migrations.rs`. This file does not exist yet. Current Kani harnesses and proptest tests use adapter functions (`adapter_*`, `state7_*_adapter`) that model the expected contract. The bridge maps obligations to **planned** production symbols that must be implemented before State 12 closure.

## Obligation-to-Source Mapping

### Cluster 1: Runtime Open / MigrationRequired Detection (Seeds 001, PO-R01, PO-R08, PO-R15)

**Domain claim**: Runtime open of an old supported store returns MigrationRequired and performs no migration side effects.

| Obligation | Verifier | Production Source Ref | Status |
|---|---|---|---|
| PO-R01 | kani | `vb_storage::open_store` → planned `migrations::detect_old_store` in `crates/vb_storage/src/migrations.rs` | planned |
| PO-R08 | proptest | `vb_storage::open_store` → `FjallJournal::open` in `crates/vb_storage/src/journal/core.rs:71` | planned |
| PO-R15 | cargo-fuzz | `vb_storage::codec::decode_record_header` boundary in `crates/vb_storage/src/codec/`; hostile manifest bytes at runtime-open surface | planned |

**Existing infrastructure**:
- `CURRENT_SCHEMA_VERSION` (u16 = 1) at `crates/vb_storage/src/constants.rs:48`
- `JournalError::MIGRATION_REQUIRED_CODE` (0x400D) at `crates/vb_storage/src/error/codes.rs:32`
- `JournalError::UNSUPPORTED_SCHEMA_VERSION_CODE` (0x400C) at `crates/vb_storage/src/error/codes.rs:30`
- `FjallJournal::open` at `crates/vb_storage/src/journal/core.rs:71` — needs version-detection logic
- `open_store` convenience wrapper at `crates/vb_storage/src/lib.rs:192`

**Planned production symbols**:
- `migrations::detect_old_store(version: u16) -> Option<MigrationRequired>`
- `migrations::MigrationRequired { from_version, to_version }` (typed struct, not raw comparison)
- `JournalError::MigrationRequired { from, to }` variant (currently only diagnostic code exists)

### Cluster 2: Registry Totality / Uniqueness (Seed 002, PO-R02, PO-R09)

**Domain claim**: Every supported old storage version maps to exactly one named migration entry.

| Obligation | Verifier | Production Source Ref | Status |
|---|---|---|---|
| PO-R02 | kani | planned `migrations::MigrationRegistry` in `crates/vb_storage/src/migrations.rs` | planned |
| PO-R09 | proptest | planned `migrations::MigrationRegistry` in `crates/vb_storage/src/migrations.rs` | planned |

**Planned production symbols**:
- `migrations::MigrationRegistry` — typed registry struct
- `migrations::MigrationRegistryEntry { name: &'static str, from_version: u16, ... }`
- `migrations::MigrationRegistry::lookup(version: u16) -> Result<MigrationAction, MigrationError>`

### Cluster 3: Verify-Before-Advance (Seed 003, PO-R03, PO-R10)

**Domain claim**: Manifest/version advancement is impossible before verification succeeds.

| Obligation | Verifier | Production Source Ref | Status |
|---|---|---|---|
| PO-R03 | kani | planned `migrations::MigrationPhase` typestate in `crates/vb_storage/src/migrations.rs` | planned |
| PO-R10 | proptest | planned `migrations::MigrationPhase` typestate in `crates/vb_storage/src/migrations.rs` | planned |

**Planned production symbols**:
- `migrations::MigrationPhase` — closed enum: `Planned | Copied | Verified | Cleaned | Committed`
- `migrations::advance_manifest(phase: MigrationPhase) -> Result<MigrationPhase, MigrationError>` — returns `VerificationFailed`, not `Committed`, when verification missing
- `migrations::MigrationError::VerificationFailed` variant

### Cluster 4: Cleanup Postcondition (Seed 004, PO-R04, PO-R11, PO-R16)

**Domain claim**: Cleanup-required migration reports success only after the old keyspace is empty.

| Obligation | Verifier | Production Source Ref | Status |
|---|---|---|---|
| PO-R04 | kani | planned `migrations::cleanup_old_keyspace` in `crates/vb_storage/src/migrations.rs` | planned |
| PO-R11 | proptest | planned `migrations::cleanup_old_keyspace` in `crates/vb_storage/src/migrations.rs` | planned |
| PO-R16 | cargo-fuzz | planned `migrations::cleanup_old_keyspace` codec boundary; corrupt old keyspace byte inputs | planned |

**Planned production symbols**:
- `migrations::cleanup_old_keyspace(journal: &FjallJournal, old_version: u16) -> Result<CleanupOutcome, MigrationError>`
- `migrations::CleanupOutcome` — enum: `Success | OldKeyspaceNotEmpty | NoCleanupNeeded`
- `migrations::MigrationError::CleanupOldKeyspaceNotEmpty` variant
- Keyspace deletion via `fjall::Keyspace::clear()` or equivalent persistence operations

### Cluster 5: Reopen Idempotence (Seed 005, PO-R05, PO-R12)

**Domain claim**: Reopen after successful migration reads current records without invoking migration.

| Obligation | Verifier | Production Source Ref | Status |
|---|---|---|---|
| PO-R05 | kani | planned `migrations::is_current_version` guard in `FjallJournal::open` at `crates/vb_storage/src/journal/core.rs:71` | planned |
| PO-R12 | proptest | planned `migrations::is_current_version` guard in `FjallJournal::open` at `crates/vb_storage/src/journal/core.rs:71` | planned |

**Planned production symbols**:
- `migrations::is_current_version(version: u16) -> bool` — check against `CURRENT_SCHEMA_VERSION`
- Version-guard branch in `FjallJournal::open` — when version == CURRENT_SCHEMA_VERSION, skip all migration hooks
- `migrations::migration_run_counter: AtomicU64` — must not increment on reopen

### Cluster 6: Empty Keyspace No-Op + Accounting Overflow (Seeds 006-007, PO-R06, PO-R07, PO-R13, PO-R14, PO-R17, PO-R18)

**Domain claim (006)**: Empty old-keyspace behavior is explicit no-op and cannot silently claim unverified migration.
**Domain claim (007)**: Migration counters and byte limits use checked bounded arithmetic and cannot overflow into success.

| Obligation | Verifier | Production Source Ref | Status |
|---|---|---|---|
| PO-R06 | kani | planned `migrations::migrate_from` empty-keyspace branch in `crates/vb_storage/src/migrations.rs` | planned |
| PO-R07 | kani | planned `migrations::checked_accounting` in `crates/vb_storage/src/migrations.rs` | planned |
| PO-R13 | proptest | planned `migrations::migrate_from` empty-keyspace branch | planned |
| PO-R14 | proptest | planned `migrations::checked_accounting` | planned |
| PO-R17 | cargo-fuzz | malformed empty-fixture byte inputs at Postcard codec boundary in `crates/vb_storage/src/codec/` | planned |
| PO-R18 | cargo-fuzz | boundary/overflow numeric inputs at Postcard codec boundary in `crates/vb_storage/src/codec/` | planned |

**Planned production symbols**:
- `migrations::MIGRATE_FLAG_KEY: &str = "__migration_flag"` — explicit audit trail key
- `migrations::MigrationOutcome::NoOp` variant — explicit, not silent
- `migrations::checked_add_records(current: u64, delta: u64) -> Result<u64, MigrationError>` — checked bounded arithmetic
- `migrations::checked_add_bytes(current: u64, delta: u64) -> Result<u64, MigrationError>` — checked bounded arithmetic
- `migrations::MigrationError::RecordLimitExceeded` variant
- `migrations::MigrationError::ByteLimitExceeded` variant
- `RECORD_HEADER_LEN` at `crates/vb_storage/src/constants.rs:46` — existing header boundary
- `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` at `crates/vb_storage/src/constants.rs:78` — existing byte limit pattern

## Behavior Test Mapping

The primary behavior test surface is `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` (156 lines, 7 proptest functions). Currently uses `state7_*_adapter` functions as test doubles. Per the bridge standard, these are the **behavior tests** for each domain claim. They must be updated to call production APIs after State 7 implementation.

### Behavior Test Ref Table

| Obligation | Behavior Test Function (current) | Production API to Bind (planned) |
|---|---|---|
| PO-R01, PO-R08 | `vb_aoah_runtime_open_migration_required_no_side_effects` | `vb_storage::open_store` → migration detection |
| PO-R02, PO-R09 | `vb_aoah_migration_registry_totality_uniqueness` | `migrations::MigrationRegistry::lookup` |
| PO-R03, PO-R10 | `vb_aoah_verify_before_manifest_advance` | `migrations::advance_manifest` |
| PO-R04, PO-R11 | `vb_aoah_cleanup_empty_old_keyspace_postcondition` | `migrations::cleanup_old_keyspace` |
| PO-R05, PO-R12 | `vb_aoah_reopen_after_migration_idempotent` | `migrations::is_current_version` guard |
| PO-R06, PO-R13 | `vb_aoah_empty_old_keyspace_explicit_noop` | `migrations::migrate_from` empty branch |
| PO-R07, PO-R14 | `vb_aoah_migration_accounting_overflow_returns_error` | `migrations::checked_add_records/bytes` |

All 7 behavior tests would fail if the production migration behavior were deleted — they are independent executable tests. However, they currently test adapter functions. State 7 must:
1. Create `crates/vb_storage/src/migrations.rs` with production symbols
2. Replace adapter calls in the behavior test file with production API calls
3. Ensure behavior tests pass against production code

## Refinement Harness Mapping

Refinement harnesses are separate from behavior tests:

| Obligation | Kani Refinement Harness | Location |
|---|---|---|
| PO-R01 | `vb_aoah_runtime_open_no_side_effects` | `crates/vb_storage/src/vb_aoah_runtime_open_no_side_effects_kani.rs` |
| PO-R02 | `vb_aoah_migration_registry_totality` | `crates/vb_storage/src/vb_aoah_migration_registry_totality_kani.rs` |
| PO-R03 | `vb_aoah_verify_before_manifest_advance` | `crates/vb_storage/src/vb_aoah_verify_before_manifest_advance_kani.rs` |
| PO-R04 | `vb_aoah_cleanup_success_requires_empty_old_keyspace` | `crates/vb_storage/src/vb_aoah_cleanup_success_requires_empty_old_keyspace_kani.rs` |
| PO-R05 | `vb_aoah_reopen_after_migration_no_rerun` | `crates/vb_storage/src/vb_aoah_reopen_after_migration_no_rerun_kani.rs` |
| PO-R06 | `vb_aoah_empty_old_keyspace_noop` | `crates/vb_storage/src/vb_aoah_empty_old_keyspace_noop_kani.rs` |
| PO-R07 | `vb_aoah_migration_accounting_checked_bounds` | `crates/vb_storage/src/vb_aoah_migration_accounting_checked_bounds_kani.rs` |

Kani harnesses currently use adapter functions. After State 7 implementation, these harnesses must be updated to call production migration functions. The fuzz targets (PO-R15 through PO-R18) serve as hostile-input refinement harnesses.

## State 12 Closure Obligations

For each bridge row to reach `materialized` or `verified` at State 12:

1. **Production migration file** must exist: `crates/vb_storage/src/migrations.rs`
2. **All planned symbols** must be implemented with production code (no adapters)
3. **Behavior tests** must call production APIs, not adapters
4. **Kani harnesses** must be re-run against production code, not adapters
5. **Proptest tests** must pass execution against production code
6. **Fuzz targets** must pass bounded runtime campaigns
7. **New error variants** must be added to `JournalError` or a new `MigrationError` type
8. **New diagnostic codes** must be registered in `error/codes.rs`

## Trusted Boundaries (Unchanged)

- Fjall persistence and Postcard codec remain trusted external dependencies
- Kani model bounds (u8/u16, MAX_RECORDS=8, MAX_BYTES=64) are test-first skeleton constraints; production bounds review required at State 12
- `CURRENT_SCHEMA_VERSION = 1` — must be revisited if schema evolves before production migration lands

## Mapping Summary

| Category | Count | Status |
|---|---|---|
| Kani obligations | 7 | Production source planned, refinement harness exists (adapter), behavior test exists (adapter) |
| Proptest obligations | 7 | Production source planned, behavior test exists (adapter) |
| Fuzz obligations | 4 | Source boundaries exist, fuzz targets built, runtime pending |
| **Total** | **18** | **All `mapping_status: planned` for State 7** |
| Production source symbols planned | 20+ | All target `crates/vb_storage/src/migrations.rs` |
| Behavior tests (existing) | 7 | `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` |
| Refinement harnesses | 11 (7 Kani + 4 fuzz) | Separate from behavior tests |
| Production error infrastructure | 2 | `MIGRATION_REQUIRED_CODE`, `UNSUPPORTED_SCHEMA_VERSION_CODE` in `error/codes.rs` |
| Production constants | 12+ | `CURRENT_SCHEMA_VERSION`, `RECORD_HEADER_LEN`, `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`, etc. in `constants.rs` |

---

## Proof/Refinement Coverage Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|--------------------|------------------|---------------------|------------------------|----------|------------------|------------|
| PO-R01 | runtime_open_result no side effects | Yes | `validation.rs:10-17` | `runtime_open_result` (L6) | `vb_aoah_runtime_open_no_side_effects.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_runtime_open_no_side_effects` | State 5 |
| PO-R02 | MigrationRegistry lookup totality | Yes | `migrations.rs` (planned) | `lookup_migration` (L1) | `vb_aoah_migration_registry_totality.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_registry_totality` | State 5 |
| PO-R03 | verify_before_manifest_advance | Yes | `migrations.rs` (planned) | `validate_advance` (L6) | `vb_aoah_verify_before_manifest_advance.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_verify_before_manifest_advance` | State 5 |
| PO-R04 | cleanup requires empty old keyspace | Yes | `migrations.rs` (planned) | `try_cleanup` (L6) | `vb_aoah_cleanup_success_requires_empty_old_keyspace.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_cleanup_success_requires_empty_old_keyspace` | State 5 |
| PO-R05 | reopen after migration no rerun | Yes | `migrations.rs` (planned) | `reopen_runs` (L6) | `vb_aoah_reopen_after_migration_no_rerun.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_reopen_after_migration_no_rerun` | State 5 |
| PO-R06 | empty old keyspace noop | Yes | `migrations.rs` (planned) | `migrate_empty_keyspace` (L6) | `vb_aoah_empty_old_keyspace_noop.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_empty_old_keyspace_noop` | State 5 |
| PO-R07 | migration accounting checked bounds | Yes | `migrations.rs` (planned) | `checked_add_bounded` (L1) | `vb_aoah_migration_accounting_checked_bounds.rs` | kani | `cargo kani -p vb_storage --harness vb_aoah_migration_accounting_checked_bounds` | State 5 |
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
