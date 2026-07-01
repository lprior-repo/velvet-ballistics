# Boundary Map — vb-jtqqx

## Layered View

The repair crosses exactly two boundaries:

1. **Pure decoder layer** (`vb_storage::keys::decode_storage_key`,
   `vb_storage::keys::try_key_prefix`) — the contract surface under test.
2. **Workspace-test tier** (`crates/workspace_tests/tests/journal_side_index_contracts.rs`,
   PO-008 block) — the only mutable surface for this bead.

No imperative shell, async shell, or production boundary is crossed.

## Pure / Core Boundaries

| Boundary | File | Contract |
| --- | --- | --- |
| Pure parse | `crates/vb_storage/src/keys.rs:281-295` | `try_key_prefix(bytes) -> Result<KeyPrefix, KeyDecodeError>`. Empty slice → `EmptyKey`. Unknown first byte → `UnknownPrefix { prefix }`. Pure, total over byte slices. |
| Pure parse | `crates/vb_storage/src/keys.rs:346-434` | `decode_storage_key(bytes) -> Result<StorageKey, KeyDecodeError>`. Pure, total, panic-free. Errors: `EmptyKey`, `UnknownPrefix`, `KeyLengthMismatch`, `InvalidRunId`, `ReservedSeqSentinel`. |
| Encoder | `crates/vb_storage/src/keys.rs:100-155` | `index_action_key`, `index_status_key`, `index_workflow_key` produce valid-length arrays. Used by the test only to seed the malformed-payload construction; their correctness is already covered by `crates/vb_storage/src/keys/tests.rs`. |

## Test / Imperative Shell Boundary (the only mutable boundary)

| Boundary | File | Contract |
| --- | --- | --- |
| Proptest macro framing | `crates/workspace_tests/tests/journal_side_index_contracts.rs:187-257` | `proptest! { #![proptest_config(journal_proptest_config(JOURNAL_KEY_PROPTEST_CASES))] ... }`. Three tests inside. The `JOURNAL_KEY_PROPTEST_CASES = 128` budget knob is preserved. |
| Strategy declarations | same file, lines 195-256 | The signatures `(... truncate_len in 1u8..=12u8, ...)`, `(... _extra_bytes in 0u8..=10u8, ...)`, etc. are preserved verbatim. Repair rewires the strategy outputs into the malformed-payload constructor; it does not remove strategies. |
| Body assertions | same file, lines 206-256 (currently) | Body must call `vb_storage::keys::decode_storage_key` (and/or `try_key_prefix`) against crafted malformed payloads and assert `matches!(..., Err(KeyDecodeError::Variant))` under `prop_assert!`. No `unwrap`/`expect` on decoder results. |
| File-level lint | `crates/workspace_tests/tests/journal_side_index_contracts.rs:14` | `#![forbid(unsafe_code)]` — preserved. |

## Async / Concurrency Boundary

- The decoder is sync-only. The test bodies are sync-only.
- No spawned tasks, no channels, no `tokio` runtime needed.
- The test file imports `EventSeq`, `FjallJournal`, `JournalWriteBatch` for
  *other* PO tests in the same file. The PO-008 block does **not** use
  these imports and must not introduce new ones.

## Storage / Network / Time Boundary

- The PO-008 block must not open a Fjall journal, write to a tempdir,
  or perform any I/O. The decoder is pure; routing through storage
  would discard the malformed shape.
- The `temp_journal()` helper at line 38 is used by PO-002, PO-004,
  PO-009, etc. but is out of scope for PO-008.

## Parser / Serialization Boundary

- `decode_storage_key` parses bytes directly. There is no
  postcard / YAML / JSON / IPC layer between the malformed payload
  and the decoder. The tests must call `decode_storage_key(&[u8])`
  with the literal byte slice, not a deserialized wrapper.

## Trusted / Untrusted Data Classification

| Data | Trusted after | Notes |
| --- | --- | --- |
| The malformed payload (`Vec<u8>` / `&[u8]`) | A typed `KeyDecodeError` from `decode_storage_key` | The payload is constructed by the test as untrusted bytes; the decoder is the verifier. |
| The valid key returned by `index_*_key(...)` | Same call — round-trip is internal to the test | The encoder's correctness is not the contract; it is only the seed for the truncated/oversize/wrong-family payloads. |
| The `KeyDecodeError` returned by `decode_storage_key` | `match`-arm of the test assertion | The error is the *expected* shape, not a failure. Proptest should not "panic" on the error return. |

## Forbidden Boundary Shortcuts

- Do not route through `FjallJournal::has_*_index_entry` to test the
  decoder — these are membership-only probes and never decode.
- Do not call `KeyspaceScanPolicy::*` helpers — the policy lives at the
  keyspace-iterator layer; the PO-008 contract is at the pure-decoder
  layer.
- Do not import `vb_storage::keys::KeyDecodeError` via a private path
  (e.g. `crate::error::key_decode::KeyDecodeError`). The re-export is
  `vb_storage::KeyDecodeError` (`crates/vb_storage/src/lib.rs:202`) and
  is the only import path.
- Do not introduce a new `vb_storage` re-export or change visibility
  in `constants.rs`. The repair is test-only; widening constants is
  forbidden unless `vb_storage` source is opened as a separate
  bead (it is not).
- Do not remove the existing `_extra_bytes` / `truncate_len` strategies
  to "clean up". They are proptest inputs and the contract explicitly
  requires them to be wired into the malformed-payload constructor.
- Do not change the `JOURNAL_KEY_PROPTEST_CASES = 128` value. The
  proptest budget is fixed.
- Do not introduce a new `[[test]]` stanza in
  `crates/workspace_tests/Cargo.toml`. The existing entry at lines
  61-62 is sufficient.

## Cross-Boundary Failure Modes (mapped to hazard IDs)

| Failure | Boundary crossed | Hazard ID |
| --- | --- | --- |
| Test body silently ignores `_short_key` (the original bug) | Test → no decoder call | H-MAL-001 |
| Test body silently ignores `_extra_bytes` (the original bug) | Test → no decoder call | H-MAL-001 |
| Test asserts on `JournalError::KeyCapacity` (stale docstring) | Test → wrong error vocabulary | H-MAL-002 |
| Test asserts on `ReservedSeqSentinel` (unreachable) | Test → unreachable-test shamming | H-MAL-003 |
| Test uses `unwrap`/`expect` on a decoder result that should be `Err` | Test → Holzman violation | H-MAL-004 |
| Test routes the malformed payload through `FjallJournal::has_*` | Test → membership probe (never decodes) | H-MAL-005 |