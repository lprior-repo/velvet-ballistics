# Error Taxonomy — vb-jtqqx

## Decoder Error Surface (read-only, sourced from `crates/vb_storage/src/error/key_decode.rs:8-31`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyDecodeError {
    EmptyKey,
    UnknownPrefix { prefix: u8 },
    KeyLengthMismatch { prefix: u8, expected: usize, actual: usize },
    InvalidRunId,
    ReservedSeqSentinel,
}
```

For the PO-008 proptest block, the four reachable variants are
`EmptyKey`, `UnknownPrefix`, `KeyLengthMismatch`, and `InvalidRunId`. The
fifth, `ReservedSeqSentinel`, is unreachable from side-index payloads
and must not be asserted in the repaired tests.

## Malformed-Shape → Error-Variant Mapping (the contract table)

This is the normative mapping the proptest bodies must verify. Each row
identifies a malformed-byte shape, the decoder entry point to call, and
the exact `KeyDecodeError` variant the call must return. Rows are
grouped by the test that owns them per the workflow model.

### Universal shapes (any test may exercise)

| Shape | Decoder call | Expected error | Owner test |
| --- | --- | --- | --- |
| `[]` (empty slice) | `try_key_prefix(&[])` | `Err(KeyDecodeError::EmptyKey)` | required in ≥1 test (recommended: action) |
| `[]` (empty slice) | `decode_storage_key(&[])` | `Err(KeyDecodeError::EmptyKey)` (delegates to `try_key_prefix`) | acceptable substitute |
| `vec![0xFF; L]` for any `L` | `try_key_prefix(&[0xFF, ..])` | `Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF })` | required in ≥1 test (recommended: workflow) |
| `vec![0xFF; L]` for any `L` | `decode_storage_key(&[0xFF, ..])` | `Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF })` | acceptable substitute |

### Per-test required shapes

| Test | Shape | Decoder call | Expected error |
| --- | --- | --- | --- |
| `index_action_key_decode_error_on_short_input` | `valid[..13 - truncate_len]` | `decode_storage_key(&short)` | `Err(KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 13 - truncate_len })` |
| `index_action_key_decode_error_on_short_input` | 13-byte buffer with prefix `0x32` and `run == 0` | `decode_storage_key(&zero_run)` | `Err(KeyDecodeError::InvalidRunId)` |
| `index_action_key_decode_error_on_short_input` | `vec![0x30; 13]` (status prefix, action length) | `decode_storage_key(&mismatch)` | `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 })` |
| `index_status_key_decode_error_on_wrong_length` | `valid + vec![0u8; _extra_bytes]` | `decode_storage_key(&oversize)` | `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 18 + _extra_bytes })` |
| `index_status_key_decode_error_on_wrong_length` | `valid[..17]` | `decode_storage_key(&short)` | `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 17 })` |
| `index_status_key_decode_error_on_wrong_length` | 18-byte buffer with prefix `0x30` and `run == 0` | `decode_storage_key(&zero_run)` | `Err(KeyDecodeError::InvalidRunId)` |
| `index_status_key_decode_error_on_wrong_length` | `vec![0x32; 18]` (action prefix, status length) | `decode_storage_key(&mismatch)` | `Err(KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 18 })` |
| `index_workflow_key_decode_error_on_wrong_length` | `valid + vec![0u8; _extra_bytes]` | `decode_storage_key(&oversize)` | `Err(KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 13 + _extra_bytes })` |
| `index_workflow_key_decode_error_on_wrong_length` | `valid[..13 - truncate_len]` (recommended: introduce `truncate_len`) | `decode_storage_key(&short)` | `Err(KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 13 - truncate_len })` |
| `index_workflow_key_decode_error_on_wrong_length` | 13-byte buffer with prefix `0x31` and `run == 0` | `decode_storage_key(&zero_run)` | `Err(KeyDecodeError::InvalidRunId)` |
| `index_workflow_key_decode_error_on_wrong_length` | `vec![0x30; 13]` (status prefix, workflow length) | `decode_storage_key(&mismatch)` | `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 })` |

### Strongly recommended cross-coverage (not strictly required, but expected by black-hat review)

| Test | Shape | Decoder call | Expected error |
| --- | --- | --- | --- |
| any | `vec![0xFF; 13]` (action/workflow length, unknown prefix) | `try_key_prefix` or `decode_storage_key` | `Err(UnknownPrefix { prefix: 0xFF })` |
| any | `vec![0xFF; 18]` (status length, unknown prefix) | `try_key_prefix` or `decode_storage_key` | `Err(UnknownPrefix { prefix: 0xFF })` |
| any | `vec![0x00; L]` for any `L` | `decode_storage_key(&[0x00, ...])` | `Err(UnknownPrefix { prefix: 0x00 })` (because `0x00` is not a known prefix) |

## Variant Field Assertions (precise field-level checks)

When a test asserts on a `KeyLengthMismatch`, the `prefix`, `expected`,
and `actual` fields must be checked against the table above. Specifically:

- The `prefix` field is **the byte that was actually present** at offset 0
  of the malformed payload, not the prefix the caller expected. The
  decoder surfaces the actual prefix in the error (`keys.rs:351`).
- The `expected` field is `prefix.expected_key_len()` — i.e. the length
  envelope for the actual prefix.
- The `actual` field is the byte length of the input slice.

| Case | Actual `prefix` field | Actual `expected` field | Actual `actual` field |
| --- | --- | --- | --- |
| `vec![0x30; 13]` (status prefix, action length) | `0x30` | `18` | `13` |
| `vec![0x31; 18]` (workflow prefix, status length) | `0x31` | `13` | `18` |
| `vec![0x32; 18]` (action prefix, status length) | `0x32` | `13` | `18` |
| truncated action key of length `k < 13` | `0x32` | `13` | `k` |
| truncated status key of length `k < 18` | `0x30` | `18` | `k` |
| truncated workflow key of length `k < 13` | `0x31` | `13` | `k` |
| oversize action key of length `13 + e` | `0x32` | `13` | `13 + e` |
| oversize status key of length `18 + e` | `0x30` | `18` | `18 + e` |
| oversize workflow key of length `13 + e` | `0x31` | `13` | `13 + e` |

## Non-Reachable Variants (must NOT be asserted)

| Variant | Why unreachable from side-index payloads | Forbidden assertions |
| --- | --- | --- |
| `KeyDecodeError::ReservedSeqSentinel` | The three side-index variants do not carry an `EventSeq` field; only `RunEvent` and `RunSnapshot` do. | Any assertion that `decode_storage_key(&side_index_payload)` returns `Err(ReservedSeqSentinel)`. |

## Mapping to `JournalError::MalformedKeyspaceRow` (out of test scope)

`JournalError::MalformedKeyspaceRow { prefix, expected_len, actual_len }`
is the surfaced shape at the keyspace-iterator layer for **all** decoder
errors (see `decode_run_event_key` at `keys.rs:451-474`). The repaired
proptest tests do not exercise the keyspace iterator; they call
`decode_storage_key` directly and assert on `KeyDecodeError`. The
translation is one layer up and out of scope.

If a future bead wants to add proptest coverage that asserts on
`MalformedKeyspaceRow`, the conversion path is:

```
KeyDecodeError::EmptyKey              →  MalformedKeyspaceRow { prefix: 0x00, expected_len: 0, actual_len: 0 }   (no known-prefix path)
KeyDecodeError::UnknownPrefix { p }   →  MalformedKeyspaceRow { prefix: p, expected_len: 0, actual_len: bytes.len() }
KeyDecodeError::KeyLengthMismatch     →  MalformedKeyspaceRow { prefix, expected_len: expected, actual_len: actual }
KeyDecodeError::InvalidRunId          →  MalformedKeyspaceRow { prefix: <actual>, expected_len: <actual expected>, actual_len: bytes.len() }
KeyDecodeError::ReservedSeqSentinel   →  MalformedKeyspaceRow { prefix: 0x11|0x12, expected_len: 17, actual_len: bytes.len() }
```

(See `vb_storage/src/preview/tests.rs:111-180` for the live surfaced
shape.) This conversion is documented here for future-bead use only and
is **not part of this bead's contract**.