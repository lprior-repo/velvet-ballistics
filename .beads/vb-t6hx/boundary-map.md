# Boundary Map — vb-t6hx

## Pure Functional Core Candidates

| Component | Inputs | Outputs | Forbidden effects |
|---|---|---|---|
| Doctor argv parser | argv tokens | `DoctorCommand` or `DoctorParseError` | storage open, filesystem writes |
| Keyspace parser | text | `StorageKeyspaceName` | dynamic storage lookup |
| Hex key parser | text | `HexKey` or typed parse error | storage access |
| Limit parser | text/default | `ScanLimit`, `PreviewLimit` | unchecked casts/arithmetic |
| Preview projector | bytes + `PreviewLimit` | `BoundedPreview` | unbounded allocation/rendering |
| Decode error mapper | `JournalError` | `DoctorDecodeError` | string parsing |
| Output model builder | outcomes | bounded render model | storage mutation |

## Imperative Shell

`vb_cli/src/app_impl.rs` and CLI command dispatch own:
- reading argv/env through existing main entry;
- selecting output format;
- opening storage through a read-only capability;
- invoking storage scan/get;
- writing stdout/stderr;
- mapping errors to `CliExitCode`.

Shell must not own:
- key byte layout;
- envelope decode order;
- hot runtime types;
- storage mutation for read-only inspection.

## Storage Boundary

`vb_storage` should expose a narrow read-only diagnostic API if required:
- open existing DB without mutating user data;
- address declared keyspaces by typed enum/name;
- bounded iteration and exact get;
- canonical envelope decode functions already exist.

Must not expose:
- raw Fjall internals directly to CLI;
- mutation methods on read-only capability;
- new runtime doctor types.

## Codec Boundary

Existing canonical functions:
- `decode_record_header`
- `decode_record`
- `decode_journal_event`
- `encode_record`

Contract:
- CLI doctor uses these functions or a storage-owned wrapper preserving validation order.
- CLI never reimplements unchecked envelope offsets.

## Test Boundary

Workspace tests belong under:
- `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs`

Tests should interact through public CLI and public storage fixture APIs. They should not depend on private Fjall fields.

## Runtime/Core Exclusion

No changes or types for this bead should enter:
- `vb_core` hot execution model;
- `vb_runtime` shard loop;
- `vb_ipc` binary command protocol;
- action ABI;
- workflow validation/compile hot path.

## Data Flow

```text
argv -> CLI parser -> typed DoctorCommand
     -> read-only storage open -> scan/get bounded bytes
     -> optional canonical envelope decode
     -> bounded render model -> stdout/stderr or structured cold output
```

There is no data flow from doctor scanner back into journal writer queues, runtime frames, action dispatch, or workflow admission.
