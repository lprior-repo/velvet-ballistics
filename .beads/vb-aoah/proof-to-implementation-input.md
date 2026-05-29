# Proof-to-Implementation Input (Reduced Scope) — vb-aoah

## Source targets expected downstream

- `crates/vb_storage/src/migrations.rs`: named migration registry, outcome lattice, phase transitions, cleanup/verification logic.
- `crates/vb_storage/src/lib.rs`: explicit cold-path API export only if needed; runtime open must not call migration execution.
- `crates/vb_storage/src/journal/core.rs`: runtime open/store-version detection and current-store reopen behavior.
- `crates/vb_storage/src/error/mod.rs` and `crates/vb_storage/src/error/codes.rs`: typed migration/cleanup/verification errors and diagnostic mapping.
- `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`: behavior/proptest surface for old fixture, explicit migration, cleanup, runtime rejection, reopen, empty no-op, and failure paths.
- `fuzz/fuzz_targets/*`: hostile old manifest/record/keyspace fixture inputs for seeds with codec boundaries (001, 004, 006, 007).

## Proof claims to bridge (reduced to 3 verifiers, 18 obligations)

### Kani (bounded verification — 7 claims)
| Obligation | Seed | Claim |
|---|---|---|
| PO-R01 | 001 | Runtime-open old supported store returns MigrationRequired, no panics/overflows/index violations in detection path |
| PO-R02 | 002 | Registry lookup: each supported version maps to exactly one entry, no panics on generated version sets |
| PO-R03 | 003 | Verif-before-advance: manifest remains old after copy/verification failure, no unchecked phase transitions |
| PO-R04 | 004 | Cleanup success unreachable while old keyspace non-empty; typed error returned, no panics |
| PO-R05 | 005 | Reopen after migration: reads current records without invoking migration hooks, no panics |
| PO-R06 | 006 | Empty keyspace: explicit NoOp outcome, manifest cannot advance to verified, no panics |
| PO-R07 | 007 | Checked arithmetic: overflow at u64 max returns typed limit error, not wrapped success |

### Proptest (behavior/integration — 7 claims)
| Obligation | Seed | Claim |
|---|---|---|
| PO-R08 | 001 | Old store open returns MigrationRequired, no migration side effects, keyspace/manifest unchanged |
| PO-R09 | 002 | Registry totality/uniqueness: duplicates/gaps rejected by typed errors |
| PO-R10 | 003 | Verify-before-advance: missing verification returns error, manifest unchanged |
| PO-R11 | 004 | Cleanup postcondition: success impossible with non-empty old keyspace, typed error returned |
| PO-R12 | 005 | Reopen idempotence: current records load, migration counters/hooks untouched |
| PO-R13 | 006 | Empty no-op: explicit NoOp outcome, explicit manifest behavior, no silent success |
| PO-R14 | 007 | Overflow error: max record/byte counts return typed limit error, never success |

### Cargo-fuzz (hostile input — 4 claims)
| Obligation | Seed | Claim |
|---|---|---|
| PO-R15 | 001 | Fuzz hostile manifest/version/codec inputs at runtime-open boundary; no crashes/panics |
| PO-R16 | 004 | Fuzz corrupt/truncated old keyspace inputs at cleanup codec boundary; typed errors only |
| PO-R17 | 006 | Fuzz malformed empty-fixture inputs at codec/manifest boundary; no panic, typed error or NoOp |
| PO-R18 | 007 | Fuzz boundary/overflow record count and byte-size inputs at codec boundary; typed limit errors |

## Obligation input

Use `proof-obligations.planned.jsonl` as the machine-readable source. The bridge must map each accepted proof artifact to Rust source refs, behavior test refs, refinement harness refs, and final evidence commands; planned mapping is insufficient for State 12 closure.

## Excluded lanes (not bridged)

- **TLA+**: not applicable (test-first bead). Bridge to revisit in production-migration bead.
- **Verus**: not applicable (test-first bead). Bridge to revisit post-implementation.
- **Flux**: not applicable (test-first bead). Bridge to revisit post-implementation.
- **Loom**: not applicable (no concurrency scope).
- **Miri**: not applicable (no unsafe/FFI scope).

## Non-vacuity constraints (for bridge and State 5/6)

- Kani harnesses must use `kani::Arbitrary` or bounded generators, never hardcoded shapes.
- Kani harnesses must exercise actual production/minimal-infrastructure functions, not proof-only local adapters.
- Proptest properties must exercise `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` against `vb_storage` migration infrastructure.
- Fuzz targets must exercise hostile inputs at manifest/codec/record boundaries.
