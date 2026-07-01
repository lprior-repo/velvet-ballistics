# Type Contracts: vb-09aaz — Abort Batch on All Index Key Construction Failures

## Required Types

These are domain/type contracts, not production Rust implementations.
They specify the post-conditions and field invariants that the G8
fix must preserve or add.

### `JournalWriteBatch<'j>` (existing, augmented by G8 post-condition)

Already defined at `crates/vb_storage/src/batch/types.rs:21-30`.
No fields are added or changed by this contract. The G8 fix
modifies only the post-condition of `append_event`.

Field invariants that the G8 fix must preserve:

- `aborted: bool` — `true` iff a fallible step in the current
  batch has set the abort flag. Initialized to `false` in
  `new()` (types.rs:39). Inspectable via `is_aborted()`
  (types.rs:67-70).
- `inner: fjall::OwnedWriteBatch` — staged Fjall operations.
  On an aborted batch, `len()` returns 0 (types.rs:47-50), so
  the public `len()` accessor already hides the partial stage.
- `staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>` —
  same-batch duplicate guard. Inserts happen at append_event.rs:119,
  AFTER the G8 `?`. See Open Domain Decisions in `domain-model.md`
  for the recommended reorder.
- `staged_bytes: u64`, `byte_limit: Option<u64>` — unchanged.

### `JournalError::KeyCapacity` (existing, surfaced by G8)

Already defined at `crates/vb_storage/src/error/mod.rs:28-29` with
diagnostic code `KEY_CAPACITY_EXCEEDED` at
`crates/vb_storage/src/error/codes.rs:103, 196`. The variant has no
fields (a unit variant).

The G8 fix does not modify `KeyCapacity`. It only changes the
post-condition under which `KeyCapacity` is returned from
`append_event`: previously returned via `?` without setting
`aborted`; after the fix, returned via `?` AFTER setting
`aborted = true`.

### `JournalError::BatchAborted` (existing, returned by `commit`)

Already defined at `crates/vb_storage/src/error/mod.rs:42-43` with
diagnostic code at codes.rs. Returned by `commit()` at
`crates/vb_storage/src/batch/commit.rs:20-23` when
`self.aborted == true`. The G8 fix relies on this existing
short-circuit to refuse persistence of the partial batch.

### `IndexActionKey` (existing, fixed-length byte array)

`[u8; INDEX_ACTION_KEY_BYTES]` where `INDEX_ACTION_KEY_BYTES = 13`
(constants.rs:79). Built by `keys::index_action_key`
(keys.rs:139-155). The 13-byte layout is
`[0x32 prefix][action u16 be][run u64 be][step u16 be]`
(constants.rs:43, keys.rs:139-155). No type changes.

### `JournalEvent` variant reachability (existing, narrowed by G8)

The G8 path is reachable only for `JournalEvent` variants whose
action-lifecycle class implies a pending-action-index mutation
(`batch/action_index.rs:55-90`):

- Insert-side: `ActionScheduled`, `ActionScheduledTicket`.
- Remove-side: `ActionCompletedEvent`, `ActionFailedEvent`,
  `ActionCompletedEnvelope`, `ActionAbandoned`.
- No-op: all other variants. `stage_pending_action_index_op`
  returns `Ok(())` without staging any batch operation, so G8
  cannot fire for these events.

### `SpecJournalError::KeyCapacity` (mirror, in Verus spec)

Existing variant in
`verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:152`
and `_PS_009_production.rs:185`. Currently declared "unreachable
in this mirror" because `run_event_key` is abstracted out and
`key` is supplied as an exec arg.

With G8 added, the contract must decide one of:

  (a) Keep `KeyCapacity` unreachable in the mirror for the G1
      run_event_key path (the journal key is still supplied as
      an arg), but add a NEW mirror input (e.g. `index_key_ok: bool`)
      representing the success/failure of `index_action_key` in the
      G8 step. The mirror exec body for G8 becomes:
      ```
      if !index_key_ok {
          self.aborted = true;
          return Err(SpecJournalError::KeyCapacity);
      }
      ```
      The `assume_specification` contract adds a `KeyCapacity`
      postcondition arm requiring `spec_state_preserved_except_aborted`
      and a witness `!index_key_ok`.

  (b) Document `KeyCapacity` as reachable from the G1 path with a
      new mirror input `key_ok: bool` and a corresponding `Ok`-and-
      `KeyCapacity`-pair for the G1 path. This is the simpler but
      less faithful option.

This contract recommends option (a) because it preserves the
production mirror's "key is supplied" abstraction and adds G8 as a
new mirror step. The decision belongs to the proof-writer; this
contract only flags the requirement.

## API Contract

### `JournalWriteBatch::append_event` (modified by G8 fix)

Signature unchanged:
```rust
pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>
```

Guard order contract — the 8-guard precedence (C6):

  1. **G1 KeyConstruction** — `run_event_key(event.run_id(), event.seq())?`
     at append_event.rs:43. Returns `JournalError::KeyCapacity` on
     key-build overflow. Non-aborting (this is the only
     `JournalError::KeyCapacity` path that does not set
     `aborted = true`, because it fires BEFORE any state mutation;
     the batch cannot be partial at this point).

  2. **G2 SameBatchDuplicate** — `staged_event_keys.contains(&key)` at
     append_event.rs:55. Returns `JournalError::DuplicateStagedKey`.
     Non-aborting.

  3. **G3 DurableDuplicate** — `journal.events.contains_key(key)?` at
     append_event.rs:61. Returns `JournalError::DuplicateEvent`.
     ABORTING (`aborted = true` set at append_event.rs:62).
     Pre-existing behavior; unchanged by G8 fix.

  4. **G4 BatchCount** — `inner.len() >= MAX_BATCH_COUNT` at
     append_event.rs:68. Returns `JournalError::QueueFull`.
     Non-aborting.

  5. **G5 PerRecordEncoding** — `encode_record(MAGIC_JOURNAL_EVENT, ...)?`
     at append_event.rs:71-77. Returns `JournalError::Encode`,
     `JournalError::PostcardEncodeFailed`, or
     `JournalError::PayloadTooLarge { len, max }`. Non-aborting.

  6. **G6 AccumulatedByteAdmission** — `byte_limit.checked_add(encoded_len)`
     at append_event.rs:86-102. Returns
     `JournalError::JournalBatchBytesExceeded { attempted, limit }`
     or `JournalError::SequenceOverflow`. Non-aborting.

  7. **G7 Mutation** — `inner.insert(...)` at append_event.rs:104.
     Side-effect only; infallible.

  8. **G8 IndexKeyConstruction** (NEW abort contract) —
     `journal.stage_pending_action_index_op(&mut inner, event)?` at
     append_event.rs:114-115. Returns `JournalError::KeyCapacity`
     on `index_action_key` failure (the only fallible path inside
     `stage_pending_action_index_op`). ABORTING (the fix sets
     `self.aborted = true` before returning the typed error).

After G8 returns Ok, the function performs `staged_event_keys.insert(key)`
(append_event.rs:119) and returns `Ok(())`. These two steps are
infallible and do not affect the abort flag.

The fix pattern matches `putters.rs:188-204` (and 27 other locations):
```rust
self.journal.stage_pending_action_index_op(&mut self.inner, event)
    .map_err(|e| {
        self.aborted = true;
        e
    })?;
```

Or equivalently a `match` block. The post-condition is identical:
on `Err`, the abort flag is set BEFORE the typed error propagates.

### `JournalWriteBatch::is_aborted` (existing, contract preserved)

Returns `self.aborted` (types.rs:67-70). The fix relies on this
accessor for the abort-flag observability contract. No changes.

### `JournalWriteBatch::commit` (existing, contract preserved)

Short-circuits with `Err(JournalError::BatchAborted)` when
`self.aborted == true` (commit.rs:20-23). The fix relies on this
short-circuit; no changes.

### `FjallJournal::stage_pending_action_index_op` (existing, contract preserved)

Helper at `batch/action_index.rs:106-126`. Returns
`Err(JournalError::KeyCapacity)` from `index_action_key(...)` on
key-build failure. The fix does not modify this helper. Its
post-condition contract is unchanged.

## Illegal States That Must Become Unrepresentable

After the G8 fix:

- `append_event` returns `Err(KeyCapacity)` while `batch.is_aborted() == false` (the bug).
- `batch.commit()` returns `Ok(())` after an `append_event(KeyCapacity)` (the partial-persistence variant).
- A `JournalWriteBatch` whose `inner` has `len() > 0` AND whose `aborted == false` AND whose commit subsequently persists state without the corresponding index update (master §49 violation).

## Rust Reliability Constraints

- No `unsafe`.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or unchecked arithmetic.
- No behavior flags such as `abort_on_key_error: bool`; use the unconditional `aborted = true` pattern.
- No new error variant; reuse `JournalError::KeyCapacity` which already exists.
- No new public method; the abort invariant is documented as a post-condition of the existing `append_event` signature.
- No production-code change to `putters.rs`; it already follows the abort-on-fallible-step contract (28 occurrences) and serves as the canonical reference template.

## C6 Guard Precedence Update

The existing doc-comment at append_event.rs:18-26 lists 8 guards
but only enumerates 7. The fix MUST update this doc-comment to
enumerate G8 alongside G1..G7:

```text
# Guard Precedence (C6)
  1. Key construction (G1) — run_event_key -> Err(KeyCapacity), non-aborting
  2. Semantic event validation (G2)
  3. Same-batch duplicate check (G3) — HashSet::contains -> DuplicateStagedKey
  4. Durable duplicate check (G4) — events.contains_key -> DuplicateEvent, aborts
  5. Count capacity check (G5) — inner.len() >= MAX_BATCH_COUNT -> QueueFull
  6. Per-record encoding / payload size check (G6) — encode_record -> Encode/PayloadTooLarge
  7. Accumulated byte admission check (G7) — byte_limit.checked_add -> JournalBatchBytesExceeded
  8. Index-key construction (G8) — stage_pending_action_index_op -> KeyCapacity, ABORTS
```

Note: the doc-comment's existing enumeration (lines 18-26) uses a
slightly different guard numbering (it labels G1 as "Key construction"
and folds validation into G2); the canonical guard numbering used in
the Verus spec PS-008/PS-009 is consistent with the table above.
The doc-comment update is a documentation-only change; the
implementation is line 114-115 only.

The Postconditions (ensures) section at append_event.rs:33-41 lists
"On DuplicateEvent: batch is aborted" but does not list the
KeyCapacity case. The fix MUST add a new bullet:

```text
- On `KeyCapacity` (G8): batch is aborted, no partial persistence.
```