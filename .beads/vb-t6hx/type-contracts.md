# Type Contracts — vb-t6hx

## Contract Shape

These are domain/type contracts for downstream implementation. They are not production code.

## Doctor Command Types

```rust
enum DoctorCommand {
    HealthCheck(DoctorHealthRequest),
    Storage(StorageDoctorCommand),
}

enum StorageDoctorCommand {
    Scan(ScanRequest),
    Get(GetRequest),
}
```

Contract:
- Existing stateless `doctor` health behavior remains separate from storage scan/get.
- `doctor storage scan` and `doctor storage get` must not reuse the current mutating `cmd_doctor --db` workflow.
- No boolean behavior flags decide scan vs get; the command variant carries the workflow.

## Value Object Contracts

### `StorageKeyspaceName`

Closed enum mapped one-to-one to `FjallJournal::declared_keyspaces()`:

```text
workflow_source | compiled_ir | run_header | run_event | run_snapshot | blob | index_status | index_workflow | index_action
```

Constructor/parser:
- accepts only exact declared names;
- returns `DoctorParseError::UnknownKeyspace { found }` for anything else;
- exposes `as_str()` for storage lookup without allocation requirement.

### `HexKey`

Smart constructor:
- input is boundary text only;
- trims no semantic bytes except optional accepted `0x` prefix if downstream parser explicitly allows it;
- rejects empty key, odd hex length, and non-hex characters;
- stores bytes as bounded owned bytes or fixed-capacity/small vector according to repository conventions;
- exposes `as_bytes()`.

Errors:
- `EmptyHexKey`
- `OddHexLength { len }`
- `InvalidHexDigit { offset }`
- `HexKeyTooLong { len, max }` if a max is introduced.

### `ScanLimit`

Smart constructor:
- accepts decimal integer text or typed `usize` from parser;
- rejects zero;
- rejects values above `DOCTOR_MAX_SCAN_LIMIT`;
- no unchecked casts from CLI text.

### `PreviewLimit`

Smart constructor:
- accepts decimal integer text or default constant;
- rejects zero;
- rejects values above `DOCTOR_MAX_PREVIEW_BYTES`;
- every value preview must carry `truncated: bool` and `omitted_bytes` when truncated.

### `DecodeMode`

```rust
enum DecodeMode {
    SkipDecode,
    EnvelopeHeader,
    EnvelopePayload,
}
```

Contract:
- scan defaults to `SkipDecode` for bounded projection;
- `EnvelopeHeader` validates envelope metadata and payload length/digest without typed Postcard payload rendering unless implementation has a safe typed mapping;
- `EnvelopePayload` must call the canonical `vb_storage` decode path and preserve `JournalError` categories.

### `ReadOnlyStorage`

Capability contract:
```rust
struct ReadOnlyStorage { /* opaque */ }
```

Allowed methods only:
- `scan_bounded(keyspace: StorageKeyspaceName, prefix: Option<&[u8]>, limit: ScanLimit, preview: PreviewLimit) -> Result<BoundedRows, DoctorStorageError>`
- `get_exact(keyspace: StorageKeyspaceName, key: &HexKey, preview: PreviewLimit) -> Result<GetOutcome, DoctorStorageError>`

Forbidden on this capability:
- append journal event;
- persist/flush as a diagnostic write;
- delete/compact/migrate;
- create synthetic doctor run IDs or test records.

## Result Types

```rust
enum ScanOutcome {
    Rows { rows: BoundedRows, limit: ScanLimit, exhausted: bool },
    Empty,
}

enum GetOutcome {
    Found(DoctorRow),
    NotFound { keyspace: StorageKeyspaceName, key: HexKey },
}

struct DoctorRow {
    key_hex: BoundedHex,
    value_len: u64,
    preview: BoundedPreview,
    decode: Option<EnvelopeDecodeSummary>,
}
```

Contracts:
- `Rows.rows.len() <= limit`.
- `value_len` is checked-converted from storage byte length; no unchecked casts.
- `BoundedPreview.bytes.len() <= PreviewLimit`.
- `BoundedHex` creation is bounded and fallible if formatting could exceed a cap.
- `NotFound` is a normal typed outcome, not a generic storage failure.

## Error Type Contracts

```rust
enum DoctorParseError { /* typed variants */ }
enum DoctorStorageError { /* typed variants */ }
enum DoctorDecodeError { /* maps JournalError decode categories */ }
enum DoctorOutputError { /* bounded formatting/io errors */ }
enum DoctorError { Parse(DoctorParseError), Storage(DoctorStorageError), Decode(DoctorDecodeError), Output(DoctorOutputError) }
```

Contract:
- parse errors occur before storage open;
- storage open/query failures remain distinct from decode failures;
- `JournalError::BadMagic`, `UnexpectedEof`, `PayloadTooLarge`, `PayloadDigestMismatch`, `PostcardDecodeFailed`, `UnknownRecordKind`, and `RecordKindFamilyMismatch` map to stable doctor decode categories;
- every CLI exit path maps to a deterministic `CliExitCode`.

## Repository Rule Binding

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`.
- No unchecked indexing/slicing/casts/arithmetic.
- No JSON/YAML/HTTP in runtime core; CLI structured output remains cold path.
- No doctor-specific type in `vb_core`, `vb_runtime`, or hot execution structs.
