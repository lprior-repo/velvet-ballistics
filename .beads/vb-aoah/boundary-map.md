# Boundary Map — vb-aoah Explicit Storage Migration Skeleton

## Pure Core

- Version classification: old-supported/current/future-unsupported/corrupt.
- Migration registry lookup and duplicate detection.
- Typestate transition rules for plan phases.
- Verification decision over bounded record metadata/digests.
- Outcome lattice construction.

Pure core constraints:
- No filesystem, clocks, environment variables, JSON/YAML/HTTP.
- No arbitrary string identifiers for migration routing.
- No unchecked indexing/slicing/casts/arithmetic.

## Imperative Storage Shell

- Fjall database open and process locking.
- Keyspace open/create access.
- Record iteration, read, write, delete, and flush behavior.
- Manifest persistence.

Storage shell constraints:
- Runtime open must not call explicit migration execution.
- Manifest advancement occurs only after core returns verified/cleaned state.
- Batches/counts/bytes are bounded and checked.

## CLI / Operator Shell

- Parses explicit operator command/options for migration.
- Calls storage migration API.
- Renders typed result/errors for humans.

CLI constraints:
- Cold path only.
- No JSON/HTTP route in runtime core.
- Command shape is open, but it must be explicit and not hidden in `doctor` or normal runtime open.

## Test Boundary

- `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` seeds minimal old-version fixtures.
- Fixtures are test-only and must not copy Restate storage layout/wire format.
- Tests assert old keyspace emptiness, new records exactness, manifest ordering, and runtime-open rejection.

## External Boundaries

- Restate source/docs are inspiration only; no imports, copied names, module structure, wire formats, HTTP/JSON, or distributed semantics.
- Fjall API is the persistence mechanism; Postcard/envelope validation remains the record encoding boundary.

## Hot/Cold Split

- All migration activity is cold/tooling/storage maintenance path.
- Hot runtime execution must not perform migration, parse text formats, allocate unbounded buffers, or dynamically resolve strings.
