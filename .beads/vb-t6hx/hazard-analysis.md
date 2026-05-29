# Hazard Analysis — vb-t6hx

## Persistence Hazards

| Hazard | Consequence | Contract mitigation |
|---|---|---|
| Existing `cmd_doctor --db` mutates by `persist_strict` and `append_journaled` | Doctor inspection changes user DB and test assertion fails | Storage scan/get must use `ReadOnlyStorage` capability with no write methods |
| Read-only unavailable, fallback to mutating open | Silent noncompliance with bead invariant | Fail closed with `ReadOnlyUnsupported` |
| Opening creates missing keyspaces or DB files | Doctor scan may mutate filesystem metadata | Downstream must define open-existing/read-only semantics and test no user key writes; if engine cannot avoid metadata, contract must document and restrict to no record/key mutation |

## Parser/Hostile Input Hazards

| Hazard | Consequence | Contract mitigation |
|---|---|---|
| Bad hex reaches storage | Wrong key lookup or panic via slicing | `HexKey` smart constructor before storage open |
| Unknown keyspace as string | accidental access to wrong keyspace or injection | closed `StorageKeyspaceName` enum |
| Huge scan limit | unbounded iteration/output | `ScanLimit` max cap, non-zero bound |
| Huge preview limit | unbounded allocation/output | `PreviewLimit` max cap |
| Numeric filters parsed with unchecked casts | overflow/wrap | checked parse and conversions only |

## Decode Hazards

| Hazard | Consequence | Contract mitigation |
|---|---|---|
| Postcard decode before length validation | allocation DoS or invalid proof claim | mandatory envelope order from master lines 856-858 |
| Decode errors collapsed | tests cannot prove exact stage | typed `DoctorDecodeError` mapping |
| Reimplemented envelope offsets in CLI | unchecked indexing/slicing drift | use `vb_storage` canonical codec/wrapper |
| Projection accidentally decodes every row | scan becomes expensive/failure-prone | explicit `DecodeMode::SkipDecode` default |

## Bounded Resource Hazards

| Hazard | Consequence | Contract mitigation |
|---|---|---|
| Collect all scan rows then truncate | memory blowup | iterator stops at `ScanLimit`; accumulator reserves bounded capacity |
| Format full large values | terminal flood/memory use | bounded preview with omitted-byte count and raw-get hint |
| Hex-format arbitrarily large keys/values | output blowup | key/value render caps and fallible formatting |

## Boundary Drift Hazards

| Hazard | Consequence | Contract mitigation |
|---|---|---|
| Doctor types introduced into runtime/core | violates master hot/cold contract | keep types in CLI/storage diagnostic modules only |
| CLI duplicates storage key layout | future key layout drift | use `vb_storage::keys` and declared keyspace APIs |
| Inactive `vb_cli/src/bench.rs` doctor diverges | confusing public exports | downstream must inspect collision before sharing implementations |

## Concurrency/Lock Hazards

| Hazard | Consequence | Contract mitigation |
|---|---|---|
| Scanner blocks writer lock or requires exclusive write lock | operator tool disrupts running engine | read-only/open-existing mode should avoid writer-only mutation path where supported; lock errors remain typed |
| Concurrent compaction/scan changes row visibility | nondeterministic output | tests should assert bounds and categories, not global ordering unless storage guarantees it |

## Remaining Representable Risks

- Exact Fjall read-only/open-existing capability is not present in inspected source. Until implemented, the illegal state “doctor storage inspection owns a mutating `FjallJournal`” remains representable.
- Final CLI grammar is not yet implemented; parser may still represent doctor storage subcommands as raw positionals unless downstream adds typed variants.
- Not-found exit code semantics are not specified by existing docs; downstream must pin behavior in tests before implementation.
