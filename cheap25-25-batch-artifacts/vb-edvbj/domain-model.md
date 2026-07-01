# Domain Model — vb-edvbj

## Ubiquitous Language

| Term | Definition | File / Symbol |
| ---- | ---------- | ------------- |
| **Runtime journal event** | A `RuntimeJournalEvent` value — one of 21 `#[non_exhaustive]` variants representing a runtime-side lifecycle fact that the runtime may wish to durably persist. | `crates/vb_runtime/src/journal/chunk_001.rs:15-195` |
| **Storage runtime journal** | A `StorageRuntimeJournal` adapter that persists `RuntimeJournalEvent` into a `vb_storage::FjallJournal`. Sister adapter `QueuedStorageRuntimeJournal` stages through `JournalWriterQueue`. | `crates/vb_runtime/src/journal/chunk_002.rs:1-355`, `chunk_003.rs` |
| **Layer helper** | One of three pure functions — `run_storage_event`, `action_storage_event`, `boundary_storage_event` — that map a per-layer subset of `RuntimeJournalEvent` variants to `Option<JournalEvent>`. Returning `None` is the canonical "this variant is not owned by this layer" signal. | `chunk_002.rs:41-268` |
| **Storage-event dispatcher** | `StorageRuntimeJournal::storage_event` — the top-level dispatcher that routes the variant into exactly one layer helper and finally produces a `JournalEvent` for persistence. | `chunk_002.rs:270-303` |
| **Storage journal event** | A `vb_storage::JournalEvent` value — the on-disk record format persisted by `FjallJournal`. Carries `seq` (per-run sequence number) and the run identifier. | `crates/vb_storage/src/events.rs:23` |
| **Unmapped variant** | Any `RuntimeJournalEvent` for which every layer helper returns `None`. Today, this is **only** `RuntimeJournalEvent::Resumed { run, timestamp }`. Future additions to the enum that do not update all three helpers will also be unmapped. | inferred from `chunk_002.rs:41-268` |
| **Unmapped-journal-event error** | The typed runtime error `RuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str }` that replaces the silent fabricated `JournalEvent::RunFailedEvent`. | new variant; see `error-taxonomy.md` |
| **Resume** | A successful transition from a suspended state back to active for a `Run`. Persisted on storage as `JournalEvent::RunResumed { run, seq, timestamp: DateTime<Utc> }`. | `crates/vb_storage/src/events.rs:289-297`, `crates/vb_storage/src/journal/incident.rs:203` |
| **Fabricated run failure (BUG)** | The current behaviour in `storage_event` (lines 295-302) that synthesises `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }` whenever every helper returns `None`. Identified as RE-019 / vb-2gxqo. | `chunk_002.rs:295-302` |
| **DiagnosticCode** | A `u32` registered with `vb_core::HasSymbolicCode` for telemetry and stable error identity. New variants MUST allocate a fresh code from the `0x20xx` range. | `vb_core::DiagnosticCode`, `cr

ates/vb_runtime/src/error/diagnostics.rs` |

## Value Objects (new)

| Newtype / variant | Description | Validator / contract |
| ----------------- | ----------- | -------------------- |
| `RuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str }` | The unique, typed failure returned by `storage_event` when every layer helper returns `None` for an unmapped variant. `event_kind` is the `&'static str` name of the offending `RuntimeJournalEvent` variant (e.g. `"Resumed"`). | Equality: payload field matches (`event_kind: a == event_kind: b`). Display: static `"unmapped runtime journal event: <event_kind>"`. Diagnostic code: `0x2020` (`UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE`). No symbolic code (forces `INTERNAL_INVARIANT`). |

## Aggregates

- **Storage-journal dispatcher** (the only aggregate touched by this bead): owns `StorageRuntimeJournal`, the three layer helpers, the `RuntimeResult<JournalEvent>` return shape of `storage_event`, and the typed error path on `None` from all helpers.

## Commands

- **`dispatch_storage_event(event: RuntimeJournalEvent, seq: EventSeq)`** — internally dispatched via `StorageRuntimeJournal::storage_event`. Now returns `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind })` instead of `Ok(JournalEvent::RunFailedEvent { .. })` whenever every layer helper returns `None`.

## Events (semantic)

| Event | Originator | Persistence target | Status |
| ----- | ---------- | ------------------ | ------ |
| `RuntimeJournalEvent::Resumed { run, timestamp }` | `RuntimeShard::handle_resume` (caller chain: `RuntimeShard::append_journal_event → RuntimeJournal::append_sequenced → StorageRuntimeJournal::storage_event`) | Today: silently mis-persisted as `JournalEvent::RunFailedEvent`. **After fix:** raises `RuntimeError::UnmappedRuntimeJournalEvent { event_kind: "Resumed" }` and propagates to the caller. Operator-visible: run-time failure of `append_sequenced`. | BUG → FIX |

## Policies (invariants that MUST hold after the fix)

1. **No fabricated failures.** `storage_event` MUST never fabricate a `JournalEvent::RunFailedEvent`. A run-failed storage record is only produced when the incoming `RuntimeJournalEvent` is `RuntimeJournalEvent::RunFailed { run }`, which routes through `run_storage_event`'s explicit arm.
2. **Total variant coverage on the dispatcher.** For every variant of `RuntimeJournalEvent`, the dispatcher must produce either `Ok(JournalEvent)` for valid mappings or `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind: <variant_name> })` for unmapped variants. `Ok(JournalEvent::RunFailedEvent { .. })` is reachable ONLY via the `RuntimeJournalEvent::RunFailed { run }` arm.
3. **`#[non_exhaustive]` preserved.** `RuntimeJournalEvent` remains `#[non_exhaustive]` so external exhaustive matches force recompilation when a variant is added. The fix MUST NOT remove this attribute.
4. **Strict profile gate stays.** `QueuedStorageRuntimeJournal::append_sequenced`'s `Err(RuntimeError::UnsupportedAsyncStrictAck)` gate is a separate concern and remains unchanged.
5. **Caller-agnostic propagation.** The new typed error propagates via `?` to `append_sequenced` → `RuntimeJournal::append_sequenced` → `RuntimeShard::append_journal_event` → caller. No new error-handling site is required at any caller.

## Forbidden states

- **`Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })` produced when the incoming event was a successful `RuntimeJournalEvent::Resumed { .. }`** — the canonical RE-019 state-machine corruption this bead forbids.
- **`Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })` produced when the incoming event is any non-`RunFailed` `RuntimeJournalEvent` whose layer helpers all return `None`** — the generalised forward-port hazard for any future variant that the dispatcher's helpers do not anticipate.
