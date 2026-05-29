# Error Taxonomy — vb-t6hx

## Top-Level Railway

```text
DoctorError = Parse | StorageOpen | StorageQuery | Decode | Output
```

Errors must be variants with stable categories/codes. Human text is rendering only.

## Parse Errors

| Variant | Trigger | Required behavior |
|---|---|---|
| `MissingStoragePath` | storage scan/get invoked without `--db` | fail before storage open |
| `MissingSubcommand` | `doctor storage` without `scan`/`get` | fail with usage category |
| `UnknownStorageCommand { found }` | non-scan/get storage command | fail with typed parse diagnostic |
| `UnknownKeyspace { found }` | keyspace not declared | fail before storage open |
| `MissingKey` | get requires key | fail before storage open |
| `EmptyHexKey` | empty key text | fail before storage open |
| `OddHexLength { len }` | hex input has odd byte nybbles | fail before storage open |
| `InvalidHexDigit { offset }` | non-hex byte | fail before storage open |
| `InvalidLimit { found }` | non-decimal limit | fail before storage open |
| `ZeroLimit` | limit == 0 | fail before storage open |
| `LimitTooLarge { found, max }` | scan limit exceeds cap | fail before storage open |
| `PreviewTooLarge { found, max }` | preview cap exceeds cap | fail before storage open |
| `ConflictingDecodeFlags` | mutually exclusive raw/decode flags | fail before storage open |

## Storage Open Errors

| Variant | Trigger | Required behavior |
|---|---|---|
| `ReadOnlyOpenFailed { source }` | Fjall read-only/open-existing failure | no mutation fallback |
| `ProcessLockHeld { path, holder_pid }` | existing lock conflict | preserve existing typed info |
| `KeyspaceUnavailable { keyspace }` | declared keyspace unavailable in DB | typed storage diagnostic |
| `ReadOnlyUnsupported` | implementation cannot provide read-only capability | fail closed; do not use mutating open |

## Storage Query Errors

| Variant | Trigger | Required behavior |
|---|---|---|
| `NotFound { keyspace, key }` | exact key absent | normal get outcome with non-success or typed diagnostic per CLI contract |
| `ScanFailed { keyspace, source }` | iterator error | typed storage diagnostic |
| `GetFailed { keyspace, source }` | exact lookup error | typed storage diagnostic |
| `PreviewAllocationFailed { requested }` | bounded preview allocation cannot reserve | typed output/storage diagnostic |
| `ValueLengthOverflow` | storage value length cannot fit display type | typed diagnostic |

## Decode Errors

Map `vb_storage::JournalError` categories without collapsing them:

| Storage error | Doctor decode category |
|---|---|
| `UnexpectedEof` | `DecodeUnexpectedEof` |
| `BadMagic { found }` | `DecodeBadMagic { found }` |
| `UnsupportedSchemaVersion { version }` | `DecodeUnsupportedSchemaVersion { version }` |
| `MigrationRequired { from, to }` | `DecodeMigrationRequired { from, to }` |
| `UnknownRecordKind { kind }` | `DecodeUnknownRecordKind { kind }` |
| `RecordKindFamilyMismatch { magic, kind }` | `DecodeFamilyMismatch { magic, kind }` |
| `HeaderLengthMismatch { found }` | `DecodeHeaderLengthMismatch { found }` |
| `PayloadTooLarge { len, max }` | `DecodePayloadTooLarge { len, max }` |
| `HeaderChecksumMismatch` | `DecodeHeaderChecksumMismatch` |
| `PayloadDigestMismatch` | `DecodePayloadDigestMismatch` |
| `PostcardDecodeFailed` | `DecodePostcardFailed` |
| `InvalidEvent` | `DecodeInvalidEvent` |

Contract:
- `PayloadTooLarge` must occur before allocation/Postcard decode.
- `PostcardDecodeFailed` must be impossible until all envelope validations pass.
- Decode errors do not mutate storage and do not retry with relaxed validation.

## Output Errors

| Variant | Trigger | Required behavior |
|---|---|---|
| `IoFailed` | stdout/stderr write fails | deterministic CLI exit |
| `FormattingLimitExceeded` | bounded output accumulator cap exceeded | typed diagnostic |
| `UnsupportedOutputMode` | invalid output format for storage command | parse/output diagnostic |

## Exit Code Mapping Seeds

- Parse errors: validation/usage exit.
- Storage open/query errors: storage error exit.
- Decode errors: storage/decode error exit, not success.
- Not-found: stable not-found diagnostic; downstream must decide final exit code explicitly in test plan.
