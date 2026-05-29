# Hazard Analysis — vb-aoah Explicit Storage Migration Skeleton

## Temporal Hazards

- Manifest advanced before verification: store appears current while records are missing/corrupt.
- Runtime open performs implicit migration: startup mutates old stores without operator intent.
- Cleanup before verification: old recovery source is destroyed before new records are proven.
- Reopen after migration accidentally reruns migration: non-idempotent cleanup/copy hazards.

## Rust-Core Invariant Hazards

- Primitive `u16` versions compared ad hoc create inconsistent old/future behavior.
- Boolean lifecycle flags allow `verified=false, committed=true` illegal state.
- Unchecked counts can overflow when counting migrated/deleted records.
- Raw strings for keyspace names permit typo-created shadow keyspaces.

## Bounded State Hazards

- Unbounded iteration/batching over old keyspace can violate resource rules.
- Payload allocation before envelope length validation reintroduces decode hazards.
- Cleanup count/deleted count mismatch may silently hide remaining old records.

## Persistence Hazards

- Fjall keyspace manifest collision with existing exact-nine declared keyspace tests.
- Partial write/flush failure leaves mixed old/new records.
- Missing diagnostic code for new errors weakens operator triage.

## Hostile Input / Corruption Hazards

- Corrupt old fixture/record accepted as migratable.
- Future schema mistaken for old supported schema.
- Malformed manifest causes fallback to new-store initialization instead of typed rejection.

## Verification Hazards

- Verification only checks record counts, not content/digest equivalence.
- Test-only fixture hardcodes a happy path and misses cleanup failure / missing verification path.
- Kani/Verus artifacts, if later added, may prove toy models disconnected from production code; this contract only emits seeds.

## API / Release Hazards

- CLI command spelling/package typo in bead command (`velvet-ballastics`) must be corrected in evidence to actual package `velvet-ballistics-workspace-tests`.
- Public migration API may become a stability promise; keep surface minimal and typed.
- Doctor command must not become implicit migration.

## Residual Illegal-State Risks

- Manifest storage location is unresolved, so exact type boundary for `MigrationManifest` remains open.
- Empty-old-keyspace no-op semantics are not yet chosen.
- CLI command shape is undecided.
