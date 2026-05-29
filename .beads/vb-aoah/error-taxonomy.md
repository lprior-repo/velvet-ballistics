# Error Taxonomy — vb-aoah Explicit Storage Migration Skeleton

## Existing Errors to Preserve

- `JournalError::MigrationRequired { from, to }`: runtime-open and old record decode rejection for supported older schema/version.
- `JournalError::UnsupportedSchemaVersion { version }`: future or unsupported schema/version where migration is not available.

## Required Migration Error Families

### Version and Registry Errors

- `UnsupportedMigrationSource { from, to }`: explicit migration requested from a version with no supported path.
- `MissingMigrationRegistryEntry { from, to }`: source version is marked supported but no named migration exists.
- `DuplicateMigrationRegistryEntry { from, to }`: registry contains ambiguous routes.

### Manifest Errors

- `MigrationManifestMissing`: no manifest exists where a manifest is required for old/current distinction.
- `MigrationManifestCorrupt { reason_code }`: manifest cannot be decoded or has impossible state.
- `MigrationManifestAdvanceRejected { from, to, phase }`: caller tried to advance before successful verification/cleanup.

### Copy/Rewrite Errors

- `MigrationReadFailed { keyspace }`: old records could not be read.
- `MigrationWriteFailed { keyspace }`: current records could not be written.
- `MigrationRecordDecodeFailed { record_kind }`: old record cannot be decoded by the named migration.
- `MigrationRecordEncodeFailed { record_kind }`: current record cannot be encoded.
- `MigrationBatchLimitExceeded { limit }`: bounded batch/count/bytes limit exceeded.

### Verification Errors

- `MigrationVerificationFailed { reason_code, checked_count }`: new records do not match migration contract.
- `MigrationMissingNewRecord { record_kind }`: expected migrated record absent.
- `MigrationUnexpectedNewRecord { record_kind }`: extra current-layout record appears outside migration contract.

### Cleanup Errors

- `MigrationCleanupFailed { keyspace }`: deletion failed or old keyspace still contains required-cleanup entries.
- `MigrationCleanupVerificationFailed { remaining_count }`: post-cleanup emptiness check failed.

## Error Semantics

- All errors are typed and carry stable diagnostic mappings if exposed publicly.
- Runtime open uses `MigrationRequired` for old supported versions and must not collapse migration failures into generic storage errors.
- Explicit migration failures are terminal for that attempt and must not advance manifest.
- Corrupt records fail closed; no best-effort partial replay is permitted.

## Railway Outcomes

```text
RuntimeOpen = OpenedCurrent | InitializedCurrent | Err(MigrationRequired | UnsupportedSchemaVersion | ManifestCorrupt | StorageIo)
ExplicitMigration = Migrated | NoOp | Err(Registry | Manifest | ReadWrite | Verification | Cleanup | StorageIo)
```
