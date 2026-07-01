# Contract — vb-jtqqx Side-Index Malformed-Key Test Repair

## Normative Clauses

| ID | Clause |
| --- | --- |
| SIDEX-MAL-001 | Every PO-008 proptest body must call `vb_storage::keys::decode_storage_key` (and/or `try_key_prefix`) against at least one crafted malformed byte slice and assert `prop_assert!(matches!(..., Err(KeyDecodeError::<variant>)))`. The decoder must be invoked; asserting on the valid key's length is not a substitute. |
| SIDEX-MAL-002 | The malformed-payload constructor must consume every strategy declared on the test signature. `truncate_len`, `_short_key`, `_extra_bytes`, and `truncate_len` analogues must drive at least one payload shape each. Discarding a strategy with `_` is forbidden. |
| SIDEX-MAL-003 | Across the three PO-008 tests, the decoder's reachable side-index error branches must each be exercised at least once: `EmptyKey`, `UnknownPrefix`, `KeyLengthMismatch` (multiple `actual` lengths), `InvalidRunId` (one per side-index variant). `ReservedSeqSentinel` is not exercised because it is unreachable from side-index payloads. |
| SIDEX-MAL-004 | The `JOURNAL_KEY_PROPTEST_CASES = 128` budget at `journal_side_index_contracts.rs:23` is preserved exactly. The repair does not raise or lower the case count. |
| SIDEX-MAL-005 | The file-level `#![forbid(unsafe_code)]` lint at line 14 is preserved. The repair adds no `unsafe`. |
| SIDEX-MAL-006 | Decoder results are matched under `prop_assert!`. No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg!`, or unchecked indexing of decoder results. The decoder is allowed to return `Err`; the test must not panic on that path. |
| SIDEX-MAL-007 | The PO-008 block must not call `FjallJournal::open`, `temp_journal()`, `has_*_index_entry`, `KeyspaceScanPolicy::*`, or any I/O / membership probe. The decoder is pure and is the only entry point used. |
| SIDEX-MAL-008 | The repair is bounded to one test file (`crates/workspace_tests/tests/journal_side_index_contracts.rs`). No edits to `Cargo.toml`, `crates/vb_storage/**`, `crates/workspace_tests/**` outside the named test file, or any dependency. |
| SIDEX-MAL-009 | The `KeyLengthMismatch { prefix, expected, actual }` assertion must specify the correct `prefix` byte per `error-taxonomy.md#Variant Field Assertions`. The prefix is the byte that was actually present at offset 0 of the payload, not the prefix the caller expected. |
| SIDEX-MAL-010 | Each of the three tests must include a `run == 0` payload of correct length for its variant, asserting on `Err(KeyDecodeError::InvalidRunId)`. The decoder's per-variant `InvalidRunId` branches (`keys.rs:400-402`, `412-414`, `423-425`) are exercised by the three tests respectively. |
| SIDEX-MAL-011 | Each of the three tests must include at least one within-family-prefix-mismatch payload (e.g. `vec![0x30; 13]` for the workflow and action tests, `vec![0x32; 18]` for the status test), asserting on `Err(KeyLengthMismatch { prefix: <actual_prefix>, expected: <actual_expected_len>, actual: <wrong_len> })`. |
| SIDEX-MAL-012 | At least one of the three tests must include the empty-slice payload `&[]`, asserting on `Err(KeyDecodeError::EmptyKey)` against `try_key_prefix` (or `decode_storage_key`). Recommended placement: `index_action_key_decode_error_on_short_input`. |
| SIDEX-MAL-013 | At least one of the three tests must include an unknown-prefix payload such as `vec![0xFF; L]` for some `L`, asserting on `Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF })`. Recommended placement: `index_workflow_key_decode_error_on_wrong_length`. |
| SIDEX-MAL-014 | Truncated-slice payloads must use the `truncate_len` strategy output to compute the truncated length. The truncated length must stay in `[1, 13)` for the action and workflow tests and `[1, 18)` for the status test so the assertion maps to `KeyLengthMismatch` and not `EmptyKey`. The existing `1u8..=12u8` bound for action satisfies this; status and workflow tests must use an analogous bound when truncating. |
| SIDEX-MAL-015 | Oversize-slice payloads (the existing `_extra_bytes in 0u8..=10u8` strategy in status and workflow) must wire `_extra_bytes` into the payload. Removing `_extra_bytes` is forbidden; unwired `_extra_bytes` is the original bug (H-MAL-001). |
| SIDEX-MAL-016 | `KeyDecodeError::ReservedSeqSentinel` must not be asserted. The three side-index variants do not carry an `EventSeq` field; the variant is unreachable from this code path (H-MAL-003). |
| SIDEX-MAL-017 | `JournalError::KeyCapacity` must not be asserted. `KeyCapacity` is the encoder-side error, surfaced by `ArrayVec::try_push` failures, not the decoder-side error (H-MAL-002). The PO-008 docstring may be updated to reflect the decoder contract. |
| SIDEX-MAL-018 | `KeyDecodeError` is imported via `vb_storage::KeyDecodeError` (the public re-export at `crates/vb_storage/src/lib.rs:202`) or via `vb_storage::error::KeyDecodeError`. No path-rewriting and no `use crate::...` from outside `vb_storage`. |

## Acceptance Invariants for Downstream States

1. **Each of the three PO-008 tests calls `decode_storage_key` at least once** with a malformed payload and asserts on a `KeyDecodeError` variant via `prop_assert!(matches!(...))`.
2. **Per-variant `InvalidRunId` coverage**: the action test exercises `IndexAction`'s `InvalidRunId` branch (`keys.rs:423-425`); the workflow test exercises `IndexWorkflow`'s (`keys.rs:412-414`); the status test exercises `IndexStatus`'s (`keys.rs:400-402`).
3. **Per-variant `KeyLengthMismatch` coverage** with both the correct-prefix-truncated shape and the within-family-prefix-mismatch shape.
4. **`EmptyKey` and `UnknownPrefix`** are each exercised by at least one test (placement flexible per SIDEX-MAL-012, SIDEX-MAL-013).
5. **No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg!`** in the PO-008 block.
6. **`proptest!` macro framing preserved**, `JOURNAL_KEY_PROPTEST_CASES = 128` preserved, `#![forbid(unsafe_code)]` preserved.
7. **No edits to `Cargo.toml`, `vb_storage/**`, `Cargo.lock`, or dependency manifests**.
8. **No new imports** beyond `vb_storage::keys::{decode_storage_key, try_key_prefix, index_*_key}` and `vb_storage::KeyDecodeError`. (`index_*_key` and `try_key_prefix` may already be in scope.)

## Open Domain Questions

- **Should `vb_storage::constants::PREFIX_INDEX_*` and `INDEX_*_KEY_BYTES` be `pub` instead of `pub(crate)` so the test can import them?** Out of scope for this P1. The repair uses literal byte / length constants with a comment citing `constants.rs`. A future bead may widen the visibility if needed.
- **Should the empty-slice and unknown-prefix cases be added to all three tests for symmetry, or distributed one-each?** Contract permits distribution. The recommended placement minimizes churn in the test signatures (action test gains no new strategy; workflow test gains the unknown-prefix case without adding a new strategy).
- **Should a future bead migrate the malformed-decode coverage into `crates/vb_storage/src/keys/tests.rs` as compact unit tests?** The proptest at workspace-test tier is the right place for property-driven coverage today. Per-variant unit tests in `keys/tests.rs` would complement but are out of scope here.
- **Should a future bead add a Kani harness for `decode_storage_key` covering all five `KeyDecodeError` variants with arbitrary byte sequences?** Out of scope here; flagged as a follow-up proof seed (PS-MAL-019 in `proof-seeds.jsonl`).
- **Should `FjallJournal::has_*_index_entry` be promoted to a decode-on-read probe that surfaces `MalformedKeyspaceRow`?** Out of scope; that is a production-side decision independent of this test-only repair.