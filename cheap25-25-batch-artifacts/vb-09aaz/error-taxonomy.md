# Error Taxonomy: vb-09aaz — Abort Batch on All Index Key Construction Failures

## Error Families

The fix touches only the abort-on-fallible-step invariant for one
existing error path (`JournalError::KeyCapacity` at G8). No new
error variants are introduced.

| Family | Existing error | Reachable from `append_event`? | Mutates? | Aborts? | Changed by vb-09aaz? |
| --- | --- | --- | --- | --- | --- |
| Key construction (event key, G1) | `JournalError::KeyCapacity` | Yes | no | no | No — fires before any state mutation, abort invariant is trivially satisfied |
| Semantic validation | `JournalError::InvalidEvent` | Yes | no | no | No |
| Same-batch duplicate (G2) | `JournalError::DuplicateStagedKey { run, seq }` | Yes | no | no | No |
| Durable duplicate (G3) | `JournalError::DuplicateEvent { run, seq }` | Yes | no | **yes** (existing, append_event.rs:62) | No — already aborting |
| Count capacity (G4) | `JournalError::QueueFull` | Yes | no | no | No |
| Per-record encoding (G5) | `JournalError::Encode`, `JournalError::PostcardEncodeFailed`, `JournalError::PayloadTooLarge { len, max }` | Yes | no | no | No |
| Accumulated byte admission (G6) | `JournalError::JournalBatchBytesExceeded { attempted, limit }`, `JournalError::SequenceOverflow` | Yes | no | no | No |
| Index key construction (G8) | `JournalError::KeyCapacity` | Yes (defensively; unreachable for nominal inputs) | no | **yes (NEW)** | **YES — must set `aborted = true` on Err** |
| Fjall I/O | `JournalError::Fjall(fjall::Error)` | Yes (durable-duplicate lookup, `contains_key?`) | no | no | No — production swallows via `?` and propagates; the abort invariant is trivially satisfied at this site because no state mutation has occurred yet |
| Batch commit | `JournalError::BatchAborted` | Yes (only from `commit()`) | n/a (no append path) | terminal | No — unchanged; relied upon by the fix |

## G8 IndexKeyConstruction Error — Detailed

The fix does not introduce a new variant; it changes the abort
behavior of the EXISTING `JournalError::KeyCapacity` when returned
from `JournalWriteBatch::append_event` via
`stage_pending_action_index_op`. After the fix:

- `Err(JournalError::KeyCapacity)` from `stage_pending_action_index_op`
  at append_event.rs:114-115 sets `self.aborted = true` before
  propagating the typed error.
- A subsequent `batch.commit()` on this aborted batch returns
  `Err(JournalError::BatchAborted)` (commit.rs:20-23).
- No partial persistence: the journal event for this batch is
  not committed; the index_action mutation is not committed; the
  Fjall database state is unchanged.

Before the fix, the same `Err(JournalError::KeyCapacity)` returned
from `stage_pending_action_index_op` propagated WITHOUT setting
`self.aborted = true`. The journal event WAS staged into
`self.inner` (append_event.rs:104), but the index mutation was
NOT. A subsequent `commit()` then persisted the event WITHOUT the
index update — master §49 Crash-Consistency violation.

## Error Variant Distinguishability

The fix does NOT change `JournalError::KeyCapacity`'s shape. It
remains a unit variant (`error/mod.rs:28-29`) with diagnostic
code `KEY_CAPACITY_EXCEEDED` (`error/codes.rs:103, 196`). The
display string `"journal key capacity exceeded"` is preserved.

Two production call sites return `JournalError::KeyCapacity` from
`append_event`:

- **G1 (`run_event_key`)**: append_event.rs:43 — fires before
  any state mutation. Non-aborting (the batch cannot be partial
  at this point). The abort-on-fallible-step invariant is
  trivially satisfied because `is_aborted() == false` is the
  correct post-state (nothing has been staged).
- **G8 (`stage_pending_action_index_op` -> `index_action_key`)**:
  append_event.rs:114-115 — fires AFTER the journal event is
  staged into `inner` (append_event.rs:104). ABORTING (the fix).

Both paths return the same `JournalError::KeyCapacity` variant.
The caller inspects `batch.is_aborted()` to determine which path
fired:

- `KeyCapacity` + `is_aborted() == false` -> G1 fired (event key
  construction failed before staging).
- `KeyCapacity` + `is_aborted() == true` -> G8 fired (index key
  construction failed after staging the event).

The runtime caller (`vb_runtime`) does not currently distinguish
these two cases; it surfaces the typed error and refuses to
commit the batch. After the fix, the runtime can rely on
`is_aborted() == true` as the post-condition of any G8 KeyCapacity
failure, which is the correct behavior regardless.

## Field-Level Diagnostics

The fix preserves the existing fieldless unit variant
`JournalError::KeyCapacity`. No new fields are added. The
diagnostic code `KEY_CAPACITY_EXCEEDED` is sufficient because:

- For G1 (event key): the failure is a non-recoverable contract
  violation (the encoding of `(run_id, seq)` to a 17-byte journal
  key has no realistic overflow path; the
  `JOURNAL_KEY_BYTES = 17` buffer holds `(u64 + u32) = 12` bytes
  with slack).
- For G8 (index key): the failure is also a non-recoverable
  contract violation (the encoding of
  `(action, run, step)` to a 13-byte fixed-length buffer has no
  realistic overflow path).

The diagnostic code is generic across both paths. The post-state
(`is_aborted()`) is the disambiguator.

## API Compatibility Notes

`JournalError::KeyCapacity` already exists and is unchanged. The
public method `append_event` retains its signature. The
post-condition adds a new clause ("on G8 Err, batch is aborted")
but no signature change. Existing callers that match on
`JournalError::KeyCapacity` continue to compile.

The doc-comment at append_event.rs:33-41 lists existing
postconditions. The fix MUST add a new bullet for the G8 abort
clause. This is a documentation-only change.

## Queued-Writer Path Error Taxonomy (Review Only)

`JournalWriterQueue::stage_queued_event` (queue/writer/stage.rs:31-74)
calls `stage_pending_action_index_op` at L72 with the same `?`
propagation pattern. The batch is single-shot (the
`OwnedWriteBatch` is dropped on `Err`, never committed). There is
no partial-write hazard at the queued-writer site. The typed error
propagates to `flush_batch` at queue/writer.rs:197-203 and then
to `drain_all` at the runtime boundary.

The contract explicitly does NOT require fixing the queued-writer
path. If a follow-up bead wants to differentiate "permanent" vs
"transient" failures at the queued-writer boundary, that is a
separate contract. The current behavior — drop the OwnedWriteBatch
on Err, surface the typed error to the runtime — is correct under
the no-partial-write policy.

## Direct Path (`append_unfsynced`) Error Taxonomy (Review Only)

`FjallJournal::append_unfsynced` (journal/internal.rs:50-79) builds
a fresh `OwnedWriteBatch` at L74 and commits it at L77. On
index-op failure at L76, the batch is dropped, no commit. There
is no partial-write hazard at this site. The contract does not
require fixing this path.