# Type Contracts — vb-uwxct

This bead is TEST-ONLY; no production types are modified. The contracts below
spell out the **acceptance shape** of the seven specimens after the repair so
that downstream proof planning, behavior tests, and Kani harnesses can bind to
them without ambiguity.

## 1. Production Encoder — `run_event_key` (reference only)

```rust
pub fn run_event_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>;
```

Source: `crates/vb_storage/src/keys.rs:81-83`.

**Preconditions**
- `run` is a `RunId` newtype constructed via `RunId::new(value)` with
  `value != 0` recommended for non-null runs.
- `seq` is an `EventSeq` newtype constructed via `EventSeq::new(value)` with
  `value` in `0..=u64::MAX`.

**Postconditions**
- If `seq.get() == u64::MAX` ⇒ returns `Err(JournalError::SequenceOverflow)`.
- Else ⇒ returns `Ok(key)` of shape `[PREFIX_RUN_EVENT][run_be_8][seq_be_8]`,
  length exactly `JOURNAL_KEY_BYTES` (17).
- All bytes written big-endian.

**Refinement sketch**
```
keys::run_event_key(run, seq) ==
    match sequenced_run_key(PREFIX_RUN_EVENT, run, seq) {
        Ok(k) => Ok(k),
        Err(JournalError::SequenceOverflow) =>
            assert!(seq.get() == u64::MAX) ∧ Ok(k_with_17_bytes_when_seq_lt_max)
        Err(other) => unreachable under current sequenced_run_key (KeyCapacity unreachable on 17-byte write)
    }
```

## 2. Production Helper — `sequenced_run_key` (private, reference only)

```rust
fn sequenced_run_key(
    prefix: u8,
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>;
```

Source: `crates/vb_storage/src/keys.rs:480-496`.

**Property (Rust-level, exact)**
```
∀ prefix ∈ {PREFIX_RUN_EVENT, PREFIX_RUN_SNAPSHOT},
  ∀ run: RunId, ∀ seq: EventSeq:
    sequenced_run_key(prefix, run, seq).is_err()
      ⇔ seq.get() == u64::MAX
    sequenced_run_key(prefix, run, seq).err() == Some(JournalError::SequenceOverflow)
    sequenced_run_key(prefix, run, seq).ok() == Some([prefix][run_be_8][seq_be_8])
```

**Behavioral consequence**: any test that calls this on a `seq_value != u64::MAX`
MUST receive `Ok`; any test on `seq_value == u64::MAX` MUST receive the typed
`Err(JournalError::SequenceOverflow)` and MUST NOT panic. This is precisely
the asymmetry the seven specimens must honor.

## 3. Error Variant — `JournalError::SequenceOverflow` (reference only)

```rust
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ...
    #[error("journal event sequence overflow")]
    SequenceOverflow,
    // ...
}
```

Source: `crates/vb_storage/src/error/mod.rs:69-70`.

**Acceptance**
- Unit variant, no inner data.
- Returned ONLY when `seq.get() == u64::MAX` is supplied to any of
  `journal_key`, `run_event_key`, `run_snapshot_key`.
- Distinct from `SequenceGap` and `TooManyEvents` (verified in
  `sequence_overflow_must_be_distinct_from_sequence_gap` at lines 1195-1215
  of the target file — OUT OF SCOPE for this bead, do not retouch).

## 4. Acceptance Type Contracts for the Repaired Test Specimens

The seven repaired specimens are typed **contracts** that downstream owners
must satisfy. The signatures below are the post-repair shape; the implementation
agents will edit source, but the contracts here are what test-planner and
proof-planner bind against.

### 4.1 Six proptests in `restate_journal_tail_scan_fallback_tests.rs:1326-1449`

Common structural contract for all six:

```rust
fn proptest_contract(
    run: u64,                                    // constrained to non-zero where applicable
    seq: u64,                                    // constrained to 0u64..u64::MAX OR
                                                 //   match-bound with Err(SequenceOverflow) → vacuous
) {
    prop_assume!(run != 0 || /* ok to skip if zero */);
    // Result is ALWAYS:
    //   Ok([u8; 17])   when seq ∈ 0..u64::MAX
    //   Err(JournalError::SequenceOverflow)   when seq == u64::MAX
    let result: Result<[u8; 17], JournalError> = run_event_key(RunId::new(run), EventSeq::new(seq));
    // Specimen asserts on the Ok shape; Err(SequenceOverflow) is contract-conformant.
}
```

**Per-proptest contracts**

#### 4.1.1 `run_event_key_lexicographic_ordering`

- Inputs: `r1: u64, s1: u64, r2: u64, s2: u64`
- Assumes: `r1 != 0`, `r2 != 0`
- Tighten so that for each pair `(s1, s2)` ∈ `0u64..u64::MAX` independently:
  - `run_event_key(...).ok().unwrap()` ⇒ key1, key2
  - `run_event_key(...).err()` MUST equal `Some(JournalError::SequenceOverflow)` ⇒
    skip the pair via `prop_assume!(s1 != u64::MAX && s2 != u64::MAX)` OR
    `match`-bind with `Err(SequenceOverflow) => prop_assert!(true)` (vacuous).
- Property under test (only when both keys are `Ok`):
  `key1 < key2 ⇔ r1 < r2 ∨ (r1 == r2 ∧ s1 < s2)`.

#### 4.1.2 `sequence_bytes_roundtrip_through_key_encoding`

- Inputs: `run_val: u64, seq_val: u64`
- Assume: `run_val != 0`
- Tighten with `seq_val in 0u64..u64::MAX` (canonical pattern) OR
  `prop_assume!(seq_val != u64::MAX)` OR `match` arm that treats
  `Err(SequenceOverflow)` as the contract-correct rejection (vacuous).
- Property under test (only on `Ok`): bytes `[9..17]` of the key big-endian-decode
  back to the original `seq_val`.

#### 4.1.3 `run_event_key_always_17_bytes`

- Inputs: `run_val: u64, seq_val: u64`
- Assume: `run_val != 0`
- Tighten with `seq_val in 0u64..u64::MAX` OR `prop_assume!(seq_val != u64::MAX)`.
- Property under test (only on `Ok`): `key.len() == JOURNAL_KEY_BYTES`.

#### 4.1.4 `run_event_key_always_has_correct_prefix`

- Inputs: `run_val: u64, seq_val: u64`
- Assume: `run_val != 0`
- Tighten with `seq_val in 0u64..u64::MAX` OR `prop_assume!(seq_val != u64::MAX)`.
- Property under test (only on `Ok`): `key[0] == PREFIX_RUN_EVENT`.

#### 4.1.5 `different_runs_have_different_event_key_prefixes`

- Inputs: `r1: u64, r2: u64, s1: u64, s2: u64`
- Assumes: `r1 != 0`, `r2 != 0`, `r1 != r2`
- Tighten so each `s1`, `s2` is constrained to `0u64..u64::MAX` OR has a per-seq
  `prop_assume!` / `match` clause that explicitly accepts
  `Err(JournalError::SequenceOverflow)` on the sentinel.
- Property under test (only on `Ok`s): `&key1[..9] != &key2[..9]`.

#### 4.1.6 `same_run_different_seq_keys_differ_in_seq_bytes`

- Inputs: `run_val: u64, s1: u64, s2: u64`
- Assumes: `run_val != 0`, `s1 != s2`
- Tighten so each `s1`, `s2` is constrained to `0u64..u64::MAX` OR has a per-seq
  `prop_assume!` / `match` clause accepting `Err(JournalError::SequenceOverflow)`.
- Property under test (only on `Ok`s): `&key1[..9] == &key2[..9]` ∧
  `&key1[9..17] != &key2[9..17]` ∧ `key1 != key2`.

### 4.2 Kani harness `vb_eepg_typed_partitioned_ids` → `assert_key_contracts`

Source: `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-115`.

**Symbolic inputs**
```rust
#[derive(Clone, Copy, kani::Arbitrary)]
struct SymbolicKeyInputs {
    run_hi:  u16,
    run_lo:  u16,
    seq_hi:  u16,
    seq_lo:  u16,
    // ... unchanged ...
}
```

**Refined contract for `assert_key_contracts`**

```rust
fn assert_key_contracts(inputs: SymbolicKeyInputs) {
    let run_value   = run_raw(inputs);
    let seq_value   = seq_raw(inputs);
    // ... existing setup unchanged ...

    // run_header_key is unaffected by sequence overflow; behavior unchanged.
    match keys::run_header_key(run) {
        Ok(key)  => { /* prefix + run-id bytes assertions */ }
        Err(_)   => assert!(false),
    }

    // run_event_key MUST distinguish the sentinel rejection.
    match keys::run_event_key(run, seq) {
        Ok(key) => {
            assert!(key[0] == PREFIX_RUN_EVENT);
            assert!(key[1..9] == run_value.to_be_bytes());
            assert!(key[9..17] == seq_value.to_be_bytes());
        }
        Err(JournalError::SequenceOverflow) => {
            // Contractually expected only when seq_value == u64::MAX.
            assert!(seq_value == u64::MAX);
        }
        Err(_) => assert!(false),
    }

    // Other match arms unchanged.
}
```

**Alternative acceptable shape**: a single `kani::assume(seq_value != u64::MAX)`
at the top of the harness, combined with a tightened original `match`.
Either shape satisfies this contract.

## 5. Forbidden Specimen Shapes

These shapes are explicitly forbidden after the bead:

- `run_event_key(...).expect("...")` over a full `u64` range. ❌
- `match keys::run_event_key(...) { Ok(_) => ..., Err(_) => assert!(false) }`
  on a harness that allows `seq == u64::MAX`. ❌
- Re-binding the rejection to a different variant
  (`SequenceGap`, `TooManyEvents`). ❌
- Silently changing the property under test so that `seq == u64::MAX`
  becomes a "success" case. ❌

## 6. Checked-Replacement Acceptance Forms

Each repaired specimen must take one of these forms (and these are NOT
interchangeable after the bead):

| Form | When acceptable |
|---|---|
| `seq in 0u64..u64::MAX` (proptest range) | Preferred; canonical pattern in `fjall_keyspace_manifest_tests.rs:129,131` |
| `prop_assume!(seq != u64::MAX)` | Acceptable when the surrounding property relies on knowing which value was rejected |
| `match run_event_key(...) { Ok(k) => prop_assert!..., Err(JournalError::SequenceOverflow) => prop_assert!(true), Err(_) => prop_assert!(false, "unexpected") }` | Acceptable when the property is meaningful only under `Ok`, but care needed: this **must not** silently accept `Err(SequenceOverflow)` for non-sentinel seq — the `Err(_)` arm must remain |
| `kani::assume(seq_value != u64::MAX)` at harness top | Acceptable when combined with tightened body |
| `Err(JournalError::SequenceOverflow) => { assert!(seq_value == u64::MAX); }` | Acceptable when `Err(_)` with another variant still fails |

Cross-cutting invariants:

- Specimens MUST still pass on `seq ∈ 0..u64::MAX`.
- Specimens MUST NOT silently accept `Err(_)` for any input other than
  `seq == u64::MAX`.
- Specimens MUST NOT introduce a new panic / unwrap / expect / `assert!(false)`
  on the success path.

## 7. Out-of-Type-Contract (for clarity)

- `crates/vb_storage/src/keys/tests.rs` (unit tests at 497-505 already
  contract-correct) — NOT in scope; do not modify.
- `crates/vb_storage/src/codec/tests.rs` (`encode_decode_with_max_sequence`
  uses `u64::MAX - 1`, contract-correct) — NOT in scope.
- `crates/vb_runtime/src/journal/tests/chunk_004.rs:964-973` —
  `JournalEvent::is_valid()` is a separate invariant on `EventSeq` and is NOT
  an encoder contract — NOT in scope.
- `crates/vb_storage/src/proptests.rs:27-40` — already uses
  `0u64..1000u64`, contract-correct — NOT in scope.
- `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs:123-146` —
  the canonical positive reference — NOT in scope.
- `crates/vb_storage/src/kani_record_kind.rs` — record-kind contracts, NOT
  sequence overflow — NOT in scope.
