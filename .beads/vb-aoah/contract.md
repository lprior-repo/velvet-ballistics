# Contract — vb-aoah Explicit Storage Migration Skeleton

## Contract Statement

`velvet-ballistics` storage migration must be explicit, named, typed, bounded, and fail-closed. Runtime store open may initialize a new current-version store or open an already-current store, but when it detects an old supported store/schema it must return typed `MigrationRequired { from, to }` and must not copy, cleanup, verify, or advance manifests.

## Acceptance-Relevant Requirements

R1. A workspace integration test surface exists at `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`.

R2. A supported old-version fixture is test-only, minimal, and does not copy Restate code/layout/wire formats.

R3. Explicit migration maps every supported old version to exactly one named migration.

R4. Explicit migration writes current-layout records and verifies migrated records before any manifest/current-version advancement.

R5. When cleanup is required, explicit migration verifies the old keyspace is empty before reporting success.

R6. Runtime open rejects old schema/version with typed `MigrationRequired { from, to }` and has no migration side effects.

R7. Reopening after successful migration reads current records without invoking migration.

R8. Missing verification or cleanup failure returns typed migration error and prevents manifest advancement.

R9. Empty old-keyspace migration has explicit no-op semantics: either manifest unchanged or explicit no-op evidence, not silent success.

R10. New migration errors are typed and diagnostic-code mapped when publicly exposed.

## Non-Functional Constraints

- First-party Rust remains no-unsafe/no-panic and forbids unwrap/expect/todo/unimplemented/dbg.
- No unchecked indexing, slicing, casts, arithmetic, ignored `Result`, or unbounded resources.
- Runtime core/storage path has no JSON/YAML/HTTP interpretation.
- Fjall and Postcard remain required storage/record mechanisms.
- No speed/performance claims without benchmark evidence.

## Recommended Type-Level Enforcement

- Use `StorageVersion` and `MigrationRegistryEntry` instead of raw version comparisons.
- Use typestate or phase-specific structs for planned/copied/verified/cleaned/committed migration states.
- Use closed enums for cleanup requirement, verification result, and migration outcome.
- Keep explicit migration API cold-path and inaccessible from normal runtime-open execution.

## Open Decisions for Downstream State

1. Choose manifest storage location.
2. Choose CLI command syntax.
3. Choose empty-old-keyspace no-op manifest behavior.
