# Boundary Map: vb-09aaz — Abort Batch on All Index Key Construction Failures

## Pure Core Boundary

The fix is small enough that a pure-core extraction is optional.
The relevant pure admission logic is the abort-on-fallible-step
pattern itself:

```text
fallible_step(batch)
  -> Accepted { batch.aborted == false (preserved) }
  -> Rejected { batch.aborted == true (set BEFORE error propagation), error: JournalError }
```

This helper is implicitly tested by every `put_*` method in
`putters.rs` (28 occurrences). The fix applies the same pattern
to the G8 step in `append_event`. No new pure helper is required.

If a follow-up bead wants to factor out the abort-on-error pattern
into a generic helper such as:

```text
fn try_or_abort<T>(batch: &mut JournalWriteBatch, result: Result<T, JournalError>)
  -> Result<T, JournalError>
```

that is a refactoring opportunity, not a correctness requirement.
The contract recommends deferring this refactoring until at least
3 distinct call sites use it (currently 1, after the fix).

## Imperative Storage Shell

`JournalWriteBatch::append_event` remains the imperative shell
(append_event.rs:42-121). The fix changes one line at L114-115:

```rust
// BEFORE (the bug):
self.journal
    .stage_pending_action_index_op(&mut self.inner, event)?;

// AFTER (the fix, mirroring putters.rs:188-200, 212-223, 235-247):
self.journal
    .stage_pending_action_index_op(&mut self.inner, event)
    .map_err(|e| {
        self.aborted = true;
        e
    })?;
```

Or equivalently a `match` block. The post-condition is identical:
on `Err`, the abort flag is set BEFORE the typed error propagates.

The fix is a 1-line code change (plus doc-comment updates at
append_event.rs:18-26 and L33-41). No new methods, no new fields,
no new error variants.

## Parser/Codec Boundary

The `index_action_key` constructor at `keys.rs:139-155` is the
boundary that translates an `(ActionId, RunId, StepIdx)` triple
into the fixed-length 13-byte index key. The fix does not modify
this constructor. Its behavior (return `Err(KeyCapacity)` on
`ArrayVec` overflow) is preserved.

The 13-byte fixed-length contract is:

```text
INDEX_ACTION_KEY_BYTES = 13 (constants.rs:79)
PREFIX_INDEX_ACTION   = 0x32 (constants.rs:43)
layout                = [0x32][action u16 be][run u64 be][step u16 be]
                       = 1 + 2 + 8 + 2 = 13 bytes
```

For nominal `ActionId(u16) × RunId(u64) × StepIdx(u16)` inputs,
the encoding always fits in 13 bytes and `into_inner()` succeeds.
KeyCapacity is therefore unreachable under normal operation, but
the contract treats it as DEFENSIVELY REACHABLE.

## Core/Storage Policy Boundary

No core/storage boundary change is required by the fix. The
`IndexActionKey` constructor is purely a storage-layer concern;
the `ActionId`, `RunId`, `StepIdx` newtypes come from `vb_core`
as opaque values, and the storage layer encodes them to bytes.

## Async/Concurrency Boundary

`JournalWriteBatch` is `!Send + !Sync` via
`PhantomData<*mut FjallJournal>` (types.rs:18-21). The
abort-on-fallible-step invariant is local to one batch handle;
no cross-thread aliasing is possible. No loom or scheduling
proof is required for this fix.

The queued-writer path (`JournalWriterQueue::flush_batch` at
queue/writer.rs:152-231 and `stage_queued_event` at
queue/writer/stage.rs:31-74) is single-threaded by construction:
the `OwnedWriteBatch` is owned by `stage_queued_event`'s stack
frame and is either committed or dropped on the same thread.

## Persistence Boundary

Fjall `OwnedWriteBatch` is the durable mutation boundary. The fix
preserves the existing invariant:

- The journal event insert at append_event.rs:104 and the
  pending-action-index insert/remove at
  `stage_pending_action_index_op` land in the SAME
  `OwnedWriteBatch`. A successful `commit()` makes them durable
  together; an aborted batch's `commit()` returns
  `Err(BatchAborted)` without persistence.

The fix's abort-on-error ensures that if the index mutation fails
to stage, the event insert at append_event.rs:104 is ALSO
discarded (because the batch aborts and `commit()` short-circuits).
This is the master §49 Crash-Consistency guarantee.

## Unsafe/FFI Boundary

No unsafe or FFI is required or introduced by this fix. Miri/unsafe
proof seeds are not primary. The existing repo-wide
`#![forbid(unsafe_code)]` lint at append_event.rs:1 applies.

## Public API Boundary

The public API surface is unchanged:

- `pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>` — signature unchanged.
- `pub fn is_aborted(&self) -> bool` — accessor unchanged.
- `pub fn commit(self) -> Result<(), JournalError>` — signature unchanged; short-circuit behavior unchanged.

The post-condition of `append_event` gains a new clause:

```text
On Err(JournalError::KeyCapacity) from stage_pending_action_index_op (G8):
  batch.is_aborted() == true
  batch.commit() returns Err(JournalError::BatchAborted)
  No durable writes occur for this batch.
```

This post-condition is DOCUMENTED in the doc-comment
(append_event.rs:33-41) and PROVED via the regression test
proposed at `batch/t_append_event.rs`
(`batch_append_event_index_key_error_aborts_commit`, mirroring
`batch_index_key_error_aborts_commit` at `batch/t_putters_b.rs:177-209`).

## Verifier Boundary (Verus Spec Production Binding)

The Verus spec at `verification/verus/vb-vzcuf-PS-008.rs` and the
companion spec at `verification/verus/vb-vzcuf-PS-009.rs` model
the 7-guard order and declare `JournalError::KeyCapacity` as
unreachable in the mirror (PS-008 line 174, PS-009 line 168-171).

After the fix, the production path becomes:

```text
G1 KeyConstruction      -> run_event_key -> Err(KeyCapacity) [unreachable in mirror; key supplied]
G2 SameBatchDuplicate   -> DuplicateStagedKey
G3 DurableDuplicate     -> DuplicateEvent (aborts)
G4 BatchCount           -> QueueFull
G5 PerRecordEncoding    -> Encode / PayloadTooLarge
G6 AccumulatedByteAdmission -> JournalBatchBytesExceeded
G7 Mutation             -> inner.insert
G8 IndexKeyConstruction -> stage_pending_action_index_op -> Err(KeyCapacity) [NEW, ABORTS]
```

The Verus mirrors at
`verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:78-95`
and `_PS_009_production.rs:67-93` enumerate the 7-guard order and
must be regenerated to include G8. The fix is documented in the
DRIFT POLICY header of each mirror file (L5-14 for PS-008, L5-32
for PS-009): any change to `append_event.rs` (production G1..G8
ordering) forces a regeneration of the corresponding mirror.

The recommended mirror regeneration approach is:

1. Add a new `index_key_ok: bool` exec arg to the mirror's
   `append_event` (analogous to `encode_ok: bool` for G5).
2. Add a new mirror step at the end of the body:
   ```rust
   // Guard G8: index-key construction.
   if !index_key_ok {
       self.aborted = true;
       return Err(SpecJournalError::KeyCapacity);
   }
   ```
3. Update the `assume_specification` contract to add a new match
   arm for `Err(KeyCapacity)` requiring
   `spec_state_preserved_except_aborted` and witness
   `!index_key_ok`.
4. Add a new exec wrapper `wrapper_append_event_index_key_error`
   that exercises G8 from `verus!` context.

This is the proof-writer's responsibility. The contract only flags
the requirement and the recommended approach.

## Cross-Crate Boundary

The fix is entirely within `crates/vb_storage`. No changes to:

- `crates/vb_core` — `ActionId`, `RunId`, `StepIdx` newtypes are
  opaque to storage.
- `crates/vb_runtime` — consumes the storage API downstream; no
  batch append path.
- `crates/vb_cli` — CLI entry points do not touch the batch API.
- `crates/vb_queue_semantics` — orchestration, not storage.

The fix's blast radius is the single `append_event` function in
`crates/vb_storage/src/batch/append_event.rs:42-121`. The Verus
spec extension is the secondary blast radius.

## Test Boundary

The regression test for the G8 path belongs in
`crates/vb_storage/src/batch/t_append_event.rs`. It mirrors the
existing test at `batch/t_putters_b.rs:177-209`
(`batch_index_key_error_aborts_commit`) but exercises the G8 path
specifically. The test MUST assert:

1. `append_event` returns the typed error
   `Err(JournalError::KeyCapacity)`.
2. `batch.is_aborted() == true`.
3. `batch.commit()` returns `Err(JournalError::BatchAborted)`.
4. No events are durable for the run (`events_for_run(run).is_empty()`).

The trigger for G8 is `JournalError::KeyCapacity` from
`stage_pending_action_index_op`. Since `index_action_key` cannot
realistically fail for nominal inputs, the test must use a
controlled scenario:

- **Option A (preferred for unit test)**: Construct a synthetic
  `JournalEvent` variant whose action-lifecycle class implies an
  index mutation (e.g., `ActionScheduled`) and use
  `vb_core::ActionId::new(u16::MAX)` plus
  `vb_core::RunId::new(u64::MAX)` plus
  `vb_core::StepIdx::new(u16::MAX)` — but verify in the test
  that this actually triggers KeyCapacity. If the production
  `index_action_key` does not fail (because the encoding always
  fits), the test cannot directly trigger G8.

- **Option B (recommended for proptest)**: Add a
  `proptest_vb_vzcuf_PS_010.rs` (or extend
  `proptest_vb_vzcuf_PS_004.rs`) that hammers the G8 path with
  arbitrary `ActionId/RunId/StepIdx` triples and asserts the
  abort invariant under all inputs (degenerate or not). The
  proptest can also include a manual mutation of the internal
  `staged_event_keys` HashSet or a mock `FjallJournal` to force
  the failure path.

- **Option C (compromise)**: Add a unit test that exercises the
  abort-on-error pattern directly by:
    1. Staging a `JournalEvent::ActionScheduled` with nominal
       inputs (which would succeed).
    2. Manually flipping `self.aborted = true` after a successful
       `append_event` call (this is a no-op test of the commit
       short-circuit, not of the G8 fix).
    3. Asserting `batch.is_aborted() == true` and
       `batch.commit() == Err(BatchAborted)`.

Option C is the weakest (it doesn't actually test G8) but is
the only one that works under the current production
`index_action_key` behavior. The contract RECOMMENDS that the
test-planner consider options A and B in priority order, with C
as a fallback. The proof-planner can drive Option B (proptest)
if Option A is not feasible.

This is a TEST-PLANNING decision, not a CONTRACT decision.
The contract only requires that the abort invariant be tested
under all paths. The implementation choice is downstream.