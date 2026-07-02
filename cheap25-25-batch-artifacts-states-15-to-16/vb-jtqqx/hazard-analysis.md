# Hazard Analysis — vb-jtqqx

| Hazard ID | Class | Hazard | Consequence | Contract control | Proof seed |
| --- | --- | --- | --- | --- | --- |
| H-MAL-001 | Test-only gap | The three PO-008 proptest bodies construct malformed byte sequences (`truncate_len`, `_extra_bytes`, `_short_key`) but discard them; the only assertions are on the *valid* key's length. The decoder is never invoked against a malformed payload, so the test gives zero behavioural coverage of the decoder's rejection contract. | A regression in `decode_storage_key` (e.g. dropping the length check at `keys.rs:349-355`) goes undetected. | Each PO-008 body must contain at least one `prop_assert!(matches!(decode_storage_key(malformed), Err(KeyDecodeError::Variant)))`. `truncate_len` and `_extra_bytes` must be wired into the malformed-payload constructor. | PS-MAL-001, PS-MAL-002, PS-MAL-003, PS-MAL-004 |
| H-MAL-002 | Stale vocabulary | The PO-008 proptest docstring at `journal_side_index_contracts.rs:184` and the per-test docstrings reference `JournalError::KeyCapacity`. `KeyCapacity` is the encoder-side error, not the decoder-side error. | Misleading documentation; future maintainers will assert on the wrong error type. | Body comments and assertions must reference `KeyDecodeError`. The docstring may be updated to reflect the decoder contract. | PS-MAL-005 |
| H-MAL-003 | Unreachable test | Asserting on `KeyDecodeError::ReservedSeqSentinel` from a side-index payload. The three side-index variants carry no `EventSeq` field; the variant is unreachable from this code path. | Vacuous test that always fails to exercise the variant; reviewer may mistakenly approve it as coverage. | Contract explicitly forbids asserting on `ReservedSeqSentinel`. Tests assert only on `EmptyKey`, `UnknownPrefix`, `KeyLengthMismatch`, `InvalidRunId`. | PS-MAL-006 |
| H-MAL-004 | Holzman violation | Decoder results are asserted via `unwrap()` or `expect()` instead of `match`/`prop_assert!(matches!(...))`. | Panic instead of proptest-shaped failure; violates AGENTS.md zero-`unwrap` rule. | All decoder results must be matched. The decoder is allowed to return `Err`; `unwrap` would panic on the `Err` path. | PS-MAL-007 |
| H-MAL-005 | Wrong-layer routing | Routing the malformed payload through `FjallJournal::has_*_index_entry` or any other membership-only probe. These accept `AsRef<[u8]>` and never decode. | Test passes even if the decoder is broken; the probe is independent of the decode contract. | The PO-008 block must call `decode_storage_key` (and/or `try_key_prefix`) directly. No `temp_journal()` inside PO-008. | PS-MAL-008 |
| H-MAL-006 | Dead proptest strategy | The strategies `truncate_len`, `_short_key`, and `_extra_bytes` exist on the test signatures but have no consumer in the body. | Proptest runs 128 cases with one proptest input being decorative; coverage budget is wasted. Black-hat reviewer will flag this. | Strategies must drive the malformed-payload constructor. No "discard with `_`" pattern. | PS-MAL-009 |
| H-MAL-007 | Constant drift | The repair hardcodes `0x30`, `0x31`, `0x32`, `18`, `13` instead of importing from `vb_storage::constants`. If the encoder/decoder constants change, the test goes silently stale. | The test would still pass against the *new* decoder (assuming the constants shifted in lockstep) but would be wrong against any historical decoder. | Repair may use literal constants with a comment citing `constants.rs:38-43, 77-79`. A future bead may add `pub` re-exports; that is out of scope here. | PS-MAL-010 |
| H-MAL-008 | Inconsistent variant surfacing | The decoder surfaces the **actual** prefix in `KeyLengthMismatch { prefix, .. }`. A test that asserts `prefix: 0x32` against `vec![0x30; 13]` will silently mismatch on the wrong variant field. | The test appears to pass (the `matches!` succeeds at the variant level) but the field-level check fails on closer inspection. Black-hat reviewer catches this. | All `KeyLengthMismatch` assertions must specify the correct `prefix` field per the actual first byte of the payload, per `error-taxonomy.md#Variant Field Assertions`. | PS-MAL-011 |
| H-MAL-009 | Test bound overflow | A truncated slice of a 13-byte action key using `valid[..(valid_len - truncate_len)]` produces a 0-byte slice when `truncate_len == 13`. The decoder returns `EmptyKey`, not `KeyLengthMismatch`. | Test reports `KeyLengthMismatch` mismatch but actually returns `EmptyKey`; the assertion must accept both. | The repair should bound `truncate_len` so the truncated length stays in `[1, 13)`, OR the assertion must match `EmptyKey` when `truncate_len == 13`. The existing `1u8..=12u8` already excludes `13`, so the bound is correct — but downstream must not widen the range. | PS-MAL-012 |
| H-MAL-010 | Coverage gap on per-variant `InvalidRunId` | A test that asserts `InvalidRunId` only on the action variant leaves the workflow and status branches unexercised. | The decoder's `InvalidRunId` branches for `IndexWorkflow` (`keys.rs:412-414`) and `IndexStatus` (`keys.rs:400-402`) are uncovered by proptest. | Each of the three PO-008 tests must include a `run == 0` payload. See `error-taxonomy.md#Per-test required shapes`. | PS-MAL-013 |
| H-MAL-011 | Within-family prefix mismatch gap | A test that only truncates / oversizes with the correct prefix never exercises the wrong-prefix-within-index-family case (e.g. `0x30` prefix at length 13). | The decoder's `KeyLengthMismatch { prefix: <actual> }` field is never field-checked. | Each of the three tests must include at least one within-family-prefix-mismatch payload. See `error-taxonomy.md#Per-test required shapes`. | PS-MAL-014 |
| H-MAL-012 | Empty-key branch gap | No test calls `decode_storage_key(&[])` or `try_key_prefix(&[])`. | The `try_key_prefix` empty branch (`keys.rs:282`) is uncovered by proptest. | At least one of the three tests must assert on `EmptyKey`. Recommended placement: action test. | PS-MAL-015 |
| H-MAL-013 | Unknown-prefix branch gap | No test calls the decoder with a first byte outside the nine known prefixes. | The `try_key_prefix` unknown-prefix branch (`keys.rs:293`) is uncovered by proptest. | At least one of the three tests must assert on `UnknownPrefix`. Recommended placement: workflow test. | PS-MAL-016 |

## Current Bug Sites (H-MAL-001 specifics)

The original `index_action_key_decode_error_on_short_input` body:

```rust
let truncate_len = truncate_len as usize;
if truncate_len < valid_len {
    let _short_key = &valid_key[..(valid_len - truncate_len)];
    // In production decode, short keys are rejected before field extraction.
    // We verify the encoding is correct by confirming valid_key is full-length.
    prop_assert_eq!(valid_len, 13, "valid index_action_key must be 13 bytes");
}
```

The decoded hazards:

- `_short_key` is built and immediately dropped.
- The comment "In production decode, short keys are rejected" is
  aspirational — the test does not actually exercise that rejection.
- The `if truncate_len < valid_len` branch is unreachable when
  `truncate_len in 1u8..=12u8` and `valid_len == 13` (always true);
  the branch is therefore dead-weight defensive code on top of the
  dead-weight payload.

The original `index_status_key_decode_error_on_wrong_length` body:

```rust
prop_assert_eq!(valid_key.len(), 18, "valid index_status_key must be 18 bytes");
prop_assert!(valid_key.len() >= 18, "valid key must be at least 18 bytes");
```

The decoded hazards:

- `_extra_bytes` is a strategy input but has zero consumers.
- The `>= 18` assertion is tautological (encoder always produces 18).
- No decode call.

The original `index_workflow_key_decode_error_on_wrong_length` body:

```rust
prop_assert_eq!(valid_key.len(), 13, "valid index_workflow_key must be 13 bytes");
prop_assert!(valid_key.len() == 13, "index_workflow_key is exactly 13 bytes for any valid input");
```

The decoded hazards:

- `_extra_bytes` is a strategy input but has zero consumers.
- The `== 13` assertion is redundant with the line above.
- No decode call.

## Out-of-Scope Hazards (flagged for future beads)

| Hazard | Why out of scope |
| --- | --- |
| `decode_storage_key` panic on adversarial input | The decoder is a `match`-based pure function with no indexing / slicing that can panic. Future Kani harness can prove this. |
| `decode_storage_key` does not validate `EventSeq::MAX` for side-index payloads | Side-index payloads do not carry `EventSeq`; this hazard is unreachable from this code path. |
| `KeyDecodeError` is `#[non_exhaustive]` — adding variants later could break test `match`es | The test must use `match` with explicit arms, not exhaustive `match`. Pattern: `matches!(..., Err(KeyDecodeError::KeyLengthMismatch { .. }))` allows future variants without breaking the test. |
| `JournalError::MalformedKeyspaceRow` surfacing differs from `KeyDecodeError` field set | The translation happens at the keyspace-iterator layer; not the decoder layer. |
| `vb_storage` constant visibility | Out of scope for this test-only repair; flagged for a future vb-* bead. |