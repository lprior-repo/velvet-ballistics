# Workflow Model — vb-t6hx

## Workflow: `doctor storage scan`

### States

1. `ArgvReceived`
2. `ParsedStorageScan`
3. `RejectedParseError` terminal
4. `ReadOnlyOpenAttempted`
5. `ReadOnlyOpenFailed` terminal
6. `KeyspaceSelected`
7. `ScanIterating`
8. `RowProjected`
9. `DecodeAttempted` optional substate
10. `OutputRendered` terminal success
11. `StorageFailed` terminal
12. `DecodeFailed` terminal or row-level diagnostic according to command contract

### Transitions

| From | Event | Guard | To | Outcome |
|---|---|---|---|---|
| `ArgvReceived` | parse scan args | keyspace declared, limit valid, preview valid, filters valid | `ParsedStorageScan` | typed request |
| `ArgvReceived` | parse scan args | any invalid parse input | `RejectedParseError` | typed CLI parse diagnostic |
| `ParsedStorageScan` | open storage | existing path and read-only capability available | `ReadOnlyOpenAttempted` | read-only handle |
| `ParsedStorageScan` | open storage | open/lock/config failure | `ReadOnlyOpenFailed` | typed storage diagnostic |
| `ReadOnlyOpenAttempted` | select keyspace | enum maps to storage keyspace | `KeyspaceSelected` | no string lookup in core |
| `KeyspaceSelected` | start scan | limit > 0 | `ScanIterating` | bounded iterator |
| `ScanIterating` | next row | emitted < limit | `RowProjected` | bounded preview row |
| `RowProjected` | decode requested | decode mode != skip | `DecodeAttempted` | canonical envelope validation |
| `RowProjected` | decode skipped | decode mode == skip | `ScanIterating` or `OutputRendered` | projection-only row |
| `DecodeAttempted` | decode ok | header/digest/payload valid | `ScanIterating` or `OutputRendered` | decode summary |
| `DecodeAttempted` | decode error | typed `JournalError` | `DecodeFailed` | typed decode diagnostic |
| `ScanIterating` | limit reached/end | rows <= limit | `OutputRendered` | stable text/structured output |

### Invariants

- No transition calls append, persist, create run ID, or write test event.
- Iteration stops at `ScanLimit` even if more rows exist.
- Every row preview is capped by `PreviewLimit`.
- Skip-decode path does not Postcard-decode payloads.

## Workflow: `doctor storage get`

### States

1. `ArgvReceived`
2. `ParsedStorageGet`
3. `RejectedParseError` terminal
4. `ReadOnlyOpenAttempted`
5. `GetIssued`
6. `Found`
7. `NotFound` terminal
8. `DecodeAttempted` optional
9. `OutputRendered` terminal success
10. `StorageFailed` terminal
11. `DecodeFailed` terminal

### Transitions

| From | Event | Guard | To | Outcome |
|---|---|---|---|---|
| `ArgvReceived` | parse get args | keyspace declared and key is valid `HexKey` | `ParsedStorageGet` | typed request |
| `ArgvReceived` | parse get args | bad hex/keyspace/limit | `RejectedParseError` | no storage open |
| `ParsedStorageGet` | open storage | read-only open succeeds | `ReadOnlyOpenAttempted` | read-only handle |
| `ReadOnlyOpenAttempted` | query exact key | key exists | `Found` | row with bounded preview |
| `ReadOnlyOpenAttempted` | query exact key | key absent | `NotFound` | typed not-found diagnostic |
| `Found` | decode requested | value exists | `DecodeAttempted` | canonical decode |
| `Found` | decode skipped | value exists | `OutputRendered` | bounded raw preview |
| `DecodeAttempted` | decode ok | envelope valid | `OutputRendered` | decode summary |
| `DecodeAttempted` | decode error | envelope invalid | `DecodeFailed` | typed decode diagnostic |

### Invariants

- Missing key is never represented as `None` crossing multiple layers; it becomes `GetOutcome::NotFound` immediately.
- Invalid hex never reaches storage.
- Large value output is a truncated preview with hint to use raw get or configured preview expansion.

## Workflow: Envelope Decode

### Mandatory Order

1. Check record has at least `RECORD_HEADER_BYTES`.
2. Decode header fields with checked offsets.
3. Validate `magic_u32` family.
4. Validate supported schema version.
5. Validate record kind and family.
6. Validate `header_len == 60`.
7. Validate `payload_len <= configured max` before allocation or Postcard decode.
8. Validate header checksum.
9. Ensure exactly declared payload bytes are available.
10. Validate payload BLAKE3 digest.
11. Only then Postcard-decode when payload decode mode requires it.

Terminal decode failures are typed and preserve the failing stage.
