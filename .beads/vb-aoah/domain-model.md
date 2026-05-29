# Domain Model — vb-aoah Explicit Storage Migration Skeleton

## Scope

Bead `vb-aoah` covers the storage migration domain only: add an explicit migration skeleton and behavior surface proving old storage shapes are not silently accepted or mutated by runtime open, and that an operator/tool migration verifies records, cleans old keyspace entries when required, and advances storage version state only after verification.

Out of scope: production implementation, tests, verifier artifacts, proof plans, Restate API/layout copying, distributed storage semantics, HTTP/JSON/YAML runtime behavior.

## Ubiquitous Language

| Term | Meaning |
|---|---|
| Store | A Fjall database directory containing `velvet-ballistics` keyspaces and binary Postcard/envelope records. |
| Runtime Open | Normal storage open through `FjallJournal::open`, `open_store`, or `init_keyspaces`. It may initialize a new current-version store but must not migrate an old store. |
| Explicit Migration | Cold-path operator/tool command or storage API invocation that intentionally transforms a supported old store to the current schema. |
| Storage Version | Version describing the store layout/manifest, distinct from each record envelope `schema_version`; current record schema is `CURRENT_SCHEMA_VERSION = 1`. |
| Migration Manifest | Durable metadata recording store version and migration evidence. Its storage location is unresolved by current code and must be chosen deliberately. |
| Named Migration | A supported old-version-to-current transformation with a stable internal name, e.g. `v0_to_v1_storage_manifest`, not an anonymous closure. |
| Old Keyspace | Legacy/test fixture keyspace or old record location used only to seed supported old layout in tests. |
| New Keyspace | Current canonical keyspace from the nine declared storage keyspaces or an approved manifest metadata location. |
| Verification | Pure/cold inspection after copy/rewrite that proves new records match expected migrated content before manifest advancement. |
| Cleanup | Deletion of obsolete old-keyspace records after verification, when the named migration declares cleanup required. |
| No-op Migration | Explicit migration result for an empty supported old store where no records require movement; manifest behavior must be explicit. |

## Entities and Value Objects

### Entities

- `StorageStore`: aggregate root over a Fjall database directory, keyspace handles, process lock, and manifest state.
- `MigrationRegistry`: immutable list of named supported migrations from old storage versions to target current version.
- `MigrationRun`: one explicit migration attempt with source version, target version, named plan, verification result, cleanup result, and terminal outcome.
- `MigrationManifest`: durable metadata for current store version and migration evidence.

### Value Objects

- `StorageVersion`: bounded nonzero/current-aware version newtype over `u16`; must reject unsupported/future values through typed errors.
- `RecordSchemaVersion`: existing envelope schema `u16`; old values route to migration-required, future values to unsupported.
- `MigrationName`: static ASCII identifier for an internal migration function; no user-controlled strings in core routing.
- `KeyspaceName`: constrained enum/newtype for approved keyspace names, preventing arbitrary string keyspace access in core migration logic.
- `MigrationEvidenceDigest`: BLAKE3 digest of verified migrated content or bounded evidence record.
- `MigratedRecordCount`: checked bounded count over `u64` with checked arithmetic.
- `CleanupRequired`: enum (`Required`, `NotRequired`) instead of boolean flags.
- `VerificationStatus`: enum (`NotRun`, `Passed`, `Failed`) making manifest advancement impossible before `Passed`.

## Aggregates

### StorageStore Aggregate

Invariants:
- Runtime open of a store detected as old returns typed `MigrationRequired { from, to }` and performs no migration cleanup/copy side effects.
- Current-version stores reopen without invoking migration logic.
- New stores are initialized as current version only through the approved initialization path.
- Store metadata must not be represented by arbitrary strings or untyped JSON/YAML/HTTP.

### MigrationRun Aggregate

Invariants:
- A migration run has exactly one source version and exactly one target version.
- A supported old version maps to exactly one named migration.
- Manifest advancement requires verification success and, when cleanup is required, cleanup success.
- Partial migration must terminate in a typed failure state and must not claim current version.

## Commands

- `OpenRuntimeStore(path)`: opens a current store or initializes a new current store; rejects old/future/corrupt state.
- `PlanExplicitMigration(path)`: reads version/manifest and selects a named migration or typed error.
- `RunExplicitMigration(path, options)`: executes copy/rewrite, verification, cleanup, and manifest advancement as a cold operator action.
- `VerifyMigratedRecords(run)`: compares old/new record content under bounded fixture-independent rules.
- `CleanupOldKeyspace(run)`: removes obsolete entries only after verification passes.

## Domain Events

- `RuntimeOpenRejectedOldStore { from, to }`
- `MigrationPlanned { name, from, to }`
- `MigrationCopiedRecords { count }`
- `MigrationVerified { count, evidence_digest }`
- `MigrationCleanupCompleted { deleted_count }`
- `MigrationManifestAdvanced { from, to }`
- `MigrationFailed { phase, error_kind }`

## Policies

- Runtime-open policy: fail closed on old schema; never silently mutate.
- Registry policy: every supported old version names a migration function.
- Verification-before-commit policy: manifest/version advances only after migrated records are verified.
- Cleanup policy: old keyspace is empty after migration when cleanup is required.
- No-copy policy: Restate may inspire failure modes only; no Restate code, layout, API, module names, wire formats, or distributed assumptions.

## Open Domain Questions

1. Exact durable manifest location is not present in current code. Choose a storage location that does not accidentally break `declared_keyspaces()` exact-nine tests unless that test is deliberately updated.
2. CLI command shape is undecided. Contract permits a cold explicit command/API but does not prescribe final syntax.
3. Empty old-keyspace behavior must be chosen: either manifest remains unchanged with no-op outcome or manifest records an explicit no-op migration; bead text allows either but requires explicitness.
