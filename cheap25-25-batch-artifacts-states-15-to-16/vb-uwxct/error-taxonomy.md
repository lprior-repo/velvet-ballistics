# Error Taxonomy — vb-uwxct

This bead is TEST-ONLY. There is no new error type to introduce. The taxonomy
below enumerates the **error states a specimen may observe** and the legal
specimen responses per state. Specimens are not allowed to **fabricate** errors
that are not in this taxonomy, and specimens are not allowed to **mask** an
error in this taxonomy with a panic.

## Observer: the Production Encoder

```
run_event_key(run, seq)
  --> Result<[u8; 17], JournalError>
```

Possible `JournalError` outcomes (the only `Err` variant the encoder can
return on `sequenced_run_key`):

| Variant | When | Documented at |
|---|---|---|
| `JournalError::SequenceOverflow` | `seq.get() == u64::MAX` | `crates/vb_storage/src/keys.rs:485-487` produces this exact variant |
| (no other variant) | All other inputs are `Ok` | The encoder never returns `KeyCapacity` because `try_push`/`try_extend_from_slice` into a 17-byte `ArrayVec` succeeds by construction |

So the encoder has **exactly one** observable `Err` outcome, and that outcome
is the same variant every time it fails.

## Specimen Error Taxonomy (the responses the seven specimens must give)

For each repair-form choice from `type-contracts.md` §6, the specimen must
classify the encoder result as one of:

### E0 — Encoder Result: `Ok([u8; 17])`

| Specimen | Required response |
|---|---|
| `run_event_key_lexicographic_ordering` | Continue with the property: `key1 < key2 ⇔ r1 < r2 ∨ (r1 == r2 ∧ s1 < s2)`. The contract-honored condition is that the property holds on this `Ok` case. |
| `sequence_bytes_roundtrip_through_key_encoding` | Decode `key[9..17]` as `u64::from_be_bytes(...)` and assert `== seq_val`. |
| `run_event_key_always_17_bytes` | Assert `key.len() == JOURNAL_KEY_BYTES`. |
| `run_event_key_always_has_correct_prefix` | Assert `key[0] == PREFIX_RUN_EVENT`. |
| `different_runs_have_different_event_key_prefixes` | Assert `&key1[..9] != &key2[..9]`. |
| `same_run_different_seq_keys_differ_in_seq_bytes` | Assert `key1[..9] == key2[..9]` ∧ `key1[9..17] != key2[9..17]` ∧ `key1 != key2`. |
| Kani `assert_key_contracts` | Assert `key[0] == PREFIX_RUN_EVENT` ∧ `key[1..9] == run_value.to_be_bytes()` ∧ `key[9..17] == seq_value.to_be_bytes()`. |

### E1 — Encoder Result: `Err(JournalError::SequenceOverflow)`

| Specimen | Required response (canonical, preferred) | Required response (alternative) |
|---|---|---|
| All six proptests | `prop_assume!(seq != u64::MAX)` OR `seq in 0u64..u64::MAX` (skip) | `match Err(JournalError::SequenceOverflow) => prop_assert!(true)` (vacuous acceptance) |
| Kani harness | `Err(JournalError::SequenceOverflow) => { assert!(seq_value == u64::MAX); }` (explicit, contract-honored) | `kani::assume(seq_value != u64::MAX)` at top of harness, `Err(_) => assert!(false)` body retained |

### E2 — Encoder Result: `Err(_)` not `SequenceOverflow`

| Specimen | Required response |
|---|---|
| All six proptests | This state is **unreachable** under the current encoder. A `match` arm that explicitly fails (`prop_assert!(false, "unexpected Err variant")`) is acceptable defensive code but not required when the `assume` / range form is chosen. |
| Kani harness | `Err(_) => assert!(false)` — retain this. E2 is the legitimate vacuous-rejection shape because the encoder has no other variant on this code path. |

### EF — Fabricated Specimen States

These are forbidden specimen error responses:

- `prop_assert!(false, "u64::MAX should succeed")` — illegal: the encoder
  correctly rejects `u64::MAX`.
- Re-binding `Err(JournalError::SequenceOverflow)` to `Err(JournalError::SequenceGap)`
  — illegal: variants are distinct (`sequence_overflow_must_be_distinct_from_sequence_gap`
  pins this).
- Panicking via `.expect("...")` on a full-range input — illegal: that is
  the original defect.

## Mapping Failure Path → Error Variant

| Path | Returned | Specimen response |
|---|---|---|
| Test samples `seq_value ∈ 0..u64::MAX` | `Ok([u8; 17])` | E0 — property held |
| Test samples `seq_value == u64::MAX` | `Err(JournalError::SequenceOverflow)` | E1 — skipped or vacuously accepted |
| (Unreachable) Production bug introduces another `Err` variant | `Err(other)` | E2 — defensive fail-closed via `prop_assert!(false, "unexpected variant")` |

## Sequencing Errors in the Specimen Post-Bead

The specimen must NOT treat `Ok(_)` as failure. The specimen must NOT treat
`Err(JournalError::SequenceOverflow)` as success unless it has verified that
`seq_value == u64::MAX`. The specimen must NOT use `.expect()` or `.unwrap()`
on a result whose source set is the full `u64` range.

## Variant Identity

The contract relies on `JournalError::SequenceOverflow` being distinct from
`JournalError::SequenceGap { ... }` and `JournalError::DuplicateEvent { ... }`.
This is **already enforced** in the source file at
`sequence_overflow_must_be_distinct_from_sequence_gap` (lines 1195-1215)
which is out-of-scope but referenced. After the bead, no test must rename or
collapse any of these variants.
