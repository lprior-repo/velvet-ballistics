# Workflow Model — vb-aoah Explicit Storage Migration Skeleton

## Workflow 1: Runtime Store Open

States:

1. `Start`
2. `ReadStoreVersion`
3. `CurrentVersionOpen`
4. `NewStoreInitializedCurrent`
5. `RejectedMigrationRequired`
6. `RejectedUnsupportedFuture`
7. `RejectedCorruptManifest`

Transitions:

- `Start -> ReadStoreVersion`: open/inspect Fjall metadata with process lock rules.
- `ReadStoreVersion -> CurrentVersionOpen`: version is current; open declared keyspaces; no migration invoked.
- `ReadStoreVersion -> NewStoreInitializedCurrent`: no store metadata exists and path is valid for new store creation.
- `ReadStoreVersion -> RejectedMigrationRequired`: old supported version detected; return `MigrationRequired { from, to }` and perform no copy/cleanup/manifest mutation.
- `ReadStoreVersion -> RejectedUnsupportedFuture`: future version detected; return unsupported schema/version error.
- `ReadStoreVersion -> RejectedCorruptManifest`: unreadable/inconsistent manifest; return typed manifest error.

Terminal outcomes:

- Opened current store.
- Initialized new current store.
- Typed rejection without migration side effects.

## Workflow 2: Explicit Migration

States:

1. `Requested`
2. `SourceVersionRead`
3. `NamedMigrationSelected`
4. `RecordsCopied`
5. `RecordsVerified`
6. `OldKeyspaceCleaned`
7. `ManifestAdvanced`
8. `NoOpCompleted`
9. `Failed`

Transitions and guards:

- `Requested -> SourceVersionRead`: explicit operator/tool API only; not reachable from runtime open.
- `SourceVersionRead -> NamedMigrationSelected`: registry contains exactly one entry for `(from, current)`.
- `SourceVersionRead -> NoOpCompleted`: old keyspace empty and policy chooses no-op without mutation or explicit no-op evidence.
- `SourceVersionRead -> Failed`: current/future/corrupt/unsupported source detected.
- `NamedMigrationSelected -> RecordsCopied`: migration writes current-layout records with checked counts and bounded batches.
- `RecordsCopied -> RecordsVerified`: verification confirms every required new record and digest/content equality under the migration contract.
- `RecordsCopied -> Failed`: copy write error or bounded resource failure.
- `RecordsVerified -> OldKeyspaceCleaned`: cleanup required and deletion succeeds.
- `RecordsVerified -> ManifestAdvanced`: cleanup not required.
- `RecordsVerified -> Failed`: verification failed; manifest must remain old/not-current.
- `OldKeyspaceCleaned -> ManifestAdvanced`: old keyspace is empty and cleanup count is recorded.
- `OldKeyspaceCleaned -> Failed`: cleanup failure; manifest must remain not-current.

Critical ordering:

```text
copy/rewrite -> verify -> cleanup(if required) -> advance manifest
```

Forbidden ordering:

```text
advance manifest -> verify
runtime open -> copy/cleanup
cleanup -> verify migrated content
```

## Workflow 3: Reopen After Migration

States:

1. `Start`
2. `ReadCurrentManifest`
3. `OpenCurrentKeyspaces`
4. `ReadMigratedRecords`
5. `OpenedWithoutMigration`

Invariant:
- Reopen after successful migration must not invoke migration registry or mutate old schema; it reads current records as ordinary current-version data.

## Edge Case Workflow: Empty Old Keyspace

Allowed terminal shapes:

- `NoOpCompleted { manifest_unchanged }`, or
- `NoOpCompleted { explicit_noop_evidence_recorded }`.

Forbidden:

- Silent current-version claim without explicit no-op policy.
- Cleanup count underflow or unchecked count increments.
