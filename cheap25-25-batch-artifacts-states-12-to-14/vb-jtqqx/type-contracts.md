# Type Contracts — vb-jtqqx

## Scope of These Contracts

The contracts below specify the malformed-payload shapes the decoder must
reject and the exact `KeyDecodeError` variant each shape must produce. They
are the normative specification that downstream `test-writer` /
`holzman-rust` must implement inside the three PO-008 proptest bodies. No
production type changes.

## Side-Index Length Envelopes (read-only constants)

| Side-index variant | Prefix byte | Expected length (B) | Layout |
| --- | --- | --- | --- |
| `IndexStatus` | `PREFIX_INDEX_STATUS = 0x30` | `INDEX_STATUS_KEY_BYTES = 18` | `[prefix][state_u8][timestamp_u64_be][run_u64_be]` |
| `IndexWorkflow` | `PREFIX_INDEX_WORKFLOW = 0x31` | `INDEX_WORKFLOW_KEY_BYTES = 13` | `[prefix][workflow_u32_be][run_u64_be]` |
| `IndexAction` | `PREFIX_INDEX_ACTION = 0x32` | `INDEX_ACTION_KEY_BYTES = 13` | `[prefix][action_u16_be][run_u64_be][step_u16_be]` |

Source: `crates/vb_storage/src/constants.rs:38-43`, `77-79`;
`crates/vb_storage/src/keys.rs:100-155` (encoders), `395-432` (decoders).

## Decoder Error Surface (read-only)

`vb_storage::KeyDecodeError` has five variants. Four are reachable from
side-index payloads; one (`ReservedSeqSentinel`) is not.

| Variant | Reachable from side-index payloads? | Trigger |
| --- | --- | --- |
| `EmptyKey` | Yes — through `try_key_prefix` with `bytes == []`. | Length-zero slice. |
| `UnknownPrefix { prefix: u8 }` | Yes — through `try_key_prefix` with a first byte not in the nine known prefixes. | First byte ∉ {`0x01`,`0x02`,`0x10`,`0x11`,`0x12`,`0x20`,`0x30`,`0x31`,`0x32`}. |
| `KeyLengthMismatch { prefix: u8, expected: usize, actual: usize }` | Yes — at the length check at `keys.rs:349-355`. | Length differs from `prefix.expected_key_len()`. `prefix` reflects the **actual** prefix byte. |
| `InvalidRunId` | Yes — at `keys.rs:400-402`, `412-414`, `423-425`. | Length is correct but `run_u64_be` field decodes to `0`. |
| `ReservedSeqSentinel` | **No.** Reserved for `RunEvent` / `RunSnapshot` payloads only. | `EventSeq == u64::MAX`. |

## Malformed Payload Contracts (the contract the tests must verify)

Each contract below names a malformed-byte shape, the exact error variant
the decoder must produce, and which of the three PO-008 tests must cover
it. "Optional" entries are encouraged for completeness but not strictly
required by the bead's primary acceptance; the **required** entries
collectively satisfy the bead scope.

### Contract M-001 — Empty slice rejects with `EmptyKey`

- **Shape**: `bytes == []`.
- **Function**: `vb_storage::keys::try_key_prefix(&[])`.
- **Expected error**: `Err(KeyDecodeError::EmptyKey)`.
- **Equivalence**: `decode_storage_key(&[])` also returns
  `Err(KeyDecodeError::EmptyKey)` because it delegates to `try_key_prefix`
  first (`keys.rs:347`).
- **Test assignment**: At least one of the three PO-008 tests must assert
  this. Recommended placement: `index_action_key_decode_error_on_short_input`
  (the "shortest possible input" framing fits the test name).

### Contract M-002 — Unknown prefix byte rejects with `UnknownPrefix { prefix }`

- **Shape**: First byte ∉ known prefixes; the rest of the slice may be any
  bytes. Suggested probe bytes: `0x00`, `0x40`, `0x7F`, `0xFF`. Recommended
  test shape: `vec![0xFF; 13]` (the same length as workflow / action
  variants) and `vec![0xFF; 18]` (the same length as status) so the
  unknown-prefix case is exercised at exactly the lengths a side-index
  entry would have had.
- **Function**: `vb_storage::keys::try_key_prefix(&[0xFF, ...])`.
- **Expected error**: `Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF })`.
- **Equivalence**: `decode_storage_key(&[0xFF, ...])` returns
  `Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF })` for any length
  (prefix check precedes length check).
- **Test assignment**: At least one of the three PO-008 tests must assert
  this. Recommended placement: `index_workflow_key_decode_error_on_wrong_length`
  (the "wrong length" framing subsumes unknown-prefix-at-correct-length).

### Contract M-003 — Truncated side-index key rejects with `KeyLengthMismatch`

- **Shape**: A valid side-index key whose leading `truncate_len` bytes
  have been sliced off (the existing `truncate_len in 1u8..=12u8`
  strategy). All three side-index variants can be truncated.
- **Function**: `vb_storage::keys::decode_storage_key(&short_key)`.
- **Expected error**: `Err(KeyDecodeError::KeyLengthMismatch { prefix: <actual_prefix>, expected: <expected_len>, actual: <short_len> })`.
- **Test assignment**:
  - `index_action_key_decode_error_on_short_input`: must cover truncation
    of an `index_action_key` (the test's own framing).
  - `index_workflow_key_decode_error_on_wrong_length`: must cover truncation
    of an `index_workflow_key`.
  - `index_status_key_decode_error_on_wrong_length`: must cover truncation
    of an `index_status_key`.

### Contract M-004 — Oversize side-index key rejects with `KeyLengthMismatch`

- **Shape**: A valid side-index key with `_extra_bytes` appended (the
  existing `_extra_bytes in 0u8..=10u8` strategy in the status and
  workflow tests). The action test has no `extra_bytes` strategy and so
  only needs to cover truncation, but it may still cover oversize by
  appending zero bytes to a copied action key.
- **Function**: `vb_storage::keys::decode_storage_key(&oversize_key)`.
- **Expected error**: `Err(KeyDecodeError::KeyLengthMismatch { prefix: <side-index prefix>, expected: <13 or 18>, actual: <13|18 + extra> })`.
- **Test assignment**:
  - `index_status_key_decode_error_on_wrong_length`: must wire
    `_extra_bytes` into an oversize key.
  - `index_workflow_key_decode_error_on_wrong_length`: must wire
    `_extra_bytes` into an oversize key.
  - `index_action_key_decode_error_on_short_input`: optional — may
    pad with extra bytes if desired.

### Contract M-005 — Within-family prefix mismatch rejects with `KeyLengthMismatch { prefix: <actual> }`

- **Shape**: A buffer whose prefix byte is one side-index prefix but whose
  total length matches a *different* side-index variant's length envelope.
  Example pairs (length = 13 cases):
  - `[0x30, ...13B...]` — status prefix at workflow/action length.
    `expected: 18, actual: 13, prefix: 0x30`.
  - `[0x31, ...13B...]` — workflow prefix at correct length, but if built
    via `decode_storage_key` against an action-shaped payload this
    represents a wrong-prefix-within-index-family case.
- **Function**: `vb_storage::keys::decode_storage_key(&wrong_family_key)`.
- **Expected error**: `Err(KeyDecodeError::KeyLengthMismatch { prefix: <actual_prefix>, expected: <expected_len for actual_prefix>, actual: <actual_len> })`.
- **Test assignment**: Each of the three tests must exercise at least one
  within-family mismatch shape so that the decoder's "use the actual
  prefix" rule is verified across all three side-index prefixes.

### Contract M-006 — Valid-length side-index payload with `run == 0` rejects with `InvalidRunId`

- **Shape**: A buffer of the correct length envelope for a side-index
  variant, with the `run` field's big-endian bytes all zero. Concrete
  recipes:
  - Action: `vec![0x32, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
    — 13 bytes, prefix `0x32`, action=1, run=0, step=0.
  - Workflow: `vec![0x31, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
    — 13 bytes, prefix `0x31`, workflow=1, run=0.
  - Status: `vec![0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
    — 18 bytes, prefix `0x30`, state=0, timestamp=0, run=0.
- **Function**: `vb_storage::keys::decode_storage_key(&zero_run_key)`.
- **Expected error**: `Err(KeyDecodeError::InvalidRunId)`.
- **Test assignment**: Each of the three PO-008 tests must include a
  `run == 0` payload so that the per-variant `InvalidRunId` branch is
  exercised across all three side-index variants.

### Contract M-007 — The `try_key_prefix` ↔ `decode_storage_key` surface is consistent

- **Shape**: For every byte sequence, the prefix classification from
  `try_key_prefix` is what `decode_storage_key` would surface in a
  `KeyLengthMismatch { prefix, .. }`. The decoder must not re-classify
  the prefix.
- **Function**: `vb_storage::keys::decode_storage_key(...)`.
- **Test implication**: A test that builds a wrong-family-prefix payload
  must assert on `KeyLengthMismatch { prefix: <byte_that_was_actually_present>, .. }`,
  not on `KeyLengthMismatch { prefix: <expected_prefix> }`. This is the
  negative-space companion to Contract M-005.

## Required Test Body Shape (Holzman-Rust + proptest)

Every test must obey these structural rules:

1. The `proptest!` macro and `JOURNAL_KEY_PROPTEST_CASES = 128` budget
   are preserved verbatim.
2. The existing strategy signatures are preserved; nothing is removed.
   The repair **rewires** `_short_key`, `_extra_bytes`, and `truncate_len`
   into the malformed-payload constructor.
3. Every assertion under `prop_assert!` (not `assert!`) so failures
   surface as proptest-shaped.
4. No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg!`. The
   decoder results are matched; the `match` arms assert.
5. `#![forbid(unsafe_code)]` lint at file scope is preserved.
6. The valid-key invariant (`valid_key.len() == N`) remains in the body
   as a sanity check but is no longer the *primary* assertion. The
   primary assertion is `prop_assert!(matches!(decode_storage_key(&malformed), Err(KeyDecodeError::Variant { .. })))`.

## Smart-Constructor Skeleton (recommended, not mandated)

The repair may use a helper closure inside each test body to keep the
assertion list readable. Example skeleton for the action test:

```rust
// Inside the proptest body:
//   Build the malformed payloads from the (action, run, step, truncate_len)
//   strategy inputs. Then for each malformed payload p, assert:
//     matches!(
//         decode_storage_key(&p),
//         Err(KeyDecodeError::KeyLengthMismatch { prefix: 0x32, expected: 13, actual }) if actual < 13
//     ) || matches!(decode_storage_key(&p), Err(KeyDecodeError::EmptyKey))
//       || matches!(decode_storage_key(&p), Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF }))
//       || matches!(decode_storage_key(&p), Err(KeyDecodeError::InvalidRunId))
```

The actual `match` decomposition is up to `test-writer`; the contract
is only that *every* branch above is reached at least once across the
three tests.

## Public API Compatibility Notes

- The decoder and error type are part of the public `vb_storage` API
  (`vb_storage::keys::decode_storage_key`, `vb_storage::KeyDecodeError`).
  The test-only repair exercises this public surface and does not
  require any visibility widening.
- The constants `PREFIX_INDEX_*` and `INDEX_*_KEY_BYTES` are
  `pub(crate)`-visible at the `vb_storage` constant module
  (`crates/vb_storage/src/constants.rs:38-43`, `77-79`). The test file
  is *outside* `vb_storage` (in `velvet-ballistics-workspace-tests`).
  The repair has two paths to reach these constants:
  - Use the literal byte values (`0x30`, `0x31`, `0x32`) and length
    values (`18`, `13`) with a comment citing `constants.rs`. This is
    the cheapest path and is permitted because the test asserts
    against the *encoder/decoder contract*, not the constant module's
    visibility.
  - Re-import the constants by adding a `pub` re-export in
    `vb_storage/src/constants.rs` and importing from the test. This
    path requires a `vb_storage` source edit and is **forbidden** for
    this test-only repair; it is recommended only if a future bead
    wants to assert constant-equality directly.
- No `Cargo.toml` change. The `[[test]] journal_side_index_contracts`
  entry is already declared in `crates/workspace_tests/Cargo.toml:61-62`
  per the codebase map.