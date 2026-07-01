# Contract — vb-edvbj

## Preconditions

1. `RuntimeJournalEvent` (21 variants, `#[non_exhaustive]`) is the only event-kind source for `StorageRuntimeJournal::storage_event`. Source: `crates/vb_runtime/src/journal/chunk_001.rs:15-195`.
2. The three layer helpers — `run_storage_event` (`chunk_002.rs:41-103`), `action_storage_event` (`chunk_002.rs:105-191`), `boundary_storage_event` (`chunk_002.rs:193-268`) — are present and have not been modified.
3. The exact buggy fallback at `chunk_002.rs:295-302` synthesises `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }` whenever every helper returns `None`. The bug is bounded to this single function.
4. `RuntimeError` is `#[non_exhaustive]` and derives `Debug` + `Clone`. Adding `UnmappedRuntimeJournalEvent { event_kind: &'static str }` is permitted.

## Postconditions

1. The buggy fallback is **deleted**; `chunk_002.rs:295-302` no longer exists in the source.
2. `StorageRuntimeJournal::storage_event` returns `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind })` for any `RuntimeJournalEvent` for which every per-layer helper returns `None` (today: `Resumed { run, timestamp }`).
3. `storage_event` does **not** fabricate any `JournalEvent::RunFailedEvent { .. }` for any input other than `RuntimeJournalEvent::RunFailed { run }`.
4. `storage_event`'s return type is unchanged: `RuntimeResult<JournalEvent>` (precondition for `?` propagation at `chunk_002.rs:343` and `chunk_003.rs:12`).
5. Per-layer helper signatures are unchanged: `run_storage_event -> Option<JournalEvent>`, `action_storage_event -> Option<JournalEvent>`, `boundary_storage_event -> RuntimeResult<Option<JournalEvent>>`.
6. `RuntimeError::UnmappedRuntimeJournalEvent` is registered in:
   - `crates/vb_runtime/src/error/mod.rs` (variant declaration),
   - `crates/vb_runtime/src/error/equality.rs::runtime_error_field_eq` (PartialEq field-arm),
   - `crates/vb_runtime/src/error/display.rs::runtime_error_static_message` (Display static-message arm) and `write_runtime_error_dynamic` (Display dynamic-message arm),
   - `crates/vb_runtime/src/error/display.rs::Error::source` (returns `None`, no edit),
   - `crates/vb_runtime/src/error/diagnostics.rs::diagnostic_code` (new constant `UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE = 0x2020`) and `runtime_code` (returns `None`).
7. Existing behavior verified by `crates/vb_runtime/src/journal/tests/chunk_001.rs`, `chunk_002.rs`, `chunk_003.rs`, `chunk_004.rs` MUST continue to pass without modification (no existing test triggers the fallback path, per `codebase-map.md`).
8. A new regression test `re_019_resumed_does_not_fabricate_run_failed` is added (out of scope for rust-contract; the test-writer / test-planner owns it).

## Invariants

### Behaviour invariants

- **I-1 (No fabrication):** `Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })` is reachable from `storage_event` ONLY via the explicit `RuntimeJournalEvent::RunFailed { run }` arm in `run_storage_event`.
- **I-2 (Total variant coverage):** For every `RuntimeJournalEvent` variant, the dispatcher produces either `Ok(JournalEvent)` (mapped) or `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind })` (unmapped). The wildcard-fabrication fallback no longer exists.
- **I-3 (Propagation uniformity):** The new error propagates via `?` from `storage_event` → `append_sequenced` → `RuntimeJournal::append_sequenced` → `RuntimeShard::append_journal_event`. No caller-level rewrite is required.
- **I-4 (Strict gate preserved):** `QueuedStorageRuntimeJournal::append_sequenced` returns `Err(UnsupportedAsyncStrictAck)` for `DurabilityProfile::Strict` BEFORE reaching `storage_event`. Independent of the fix; preserved.

### Type invariants

- **I-5:** `event_kind: &'static str` — never `String`, never `Arc<str>`. Allocation-free, copy-free, comparison-free.
- **I-6:** `event_kind` is one of the 21 declared `RuntimeJournalEvent` variant name literals. (Future-variant behavior: per H-4 mitigation, dispatcher enumerates every variant.)
- **I-7:** `RuntimeError::UnmappedRuntimeJournalEvent { event_kind }` is `Clone` (auto-derived; `&'static str` is `Copy`).
- **I-8:** `PartialEq` is field-equality on `event_kind`. `Eq` is structural.

### Display / Diagnostic invariants

- **I-9:** `Display` static-message is `"unmapped runtime journal event — dispatcher has no mapping for this variant"` (suffix-free).
- **I-10:** `Display` dynamic-message includes the variant name: `unmapped runtime journal event: <event_kind>`.
- **I-11:** `diagnostic_code() == DiagnosticCode::new(0x2020)` (constant `UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE`).
- **I-12:** `runtime_code() == None`.
- **I-13:** `symbolic_code() == SymbolicCode::INTERNAL_INVARIANT` (via the unrecognised-code fallback in `registered_symbolic_code`).
- **I-14:** `Error::source() == None`.

## Signatures (preserved)

```rust
// crates/vb_runtime/src/error/mod.rs
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RuntimeError {
    // ... existing 51 variants ...
    UnmappedRuntimeJournalEvent {
        event_kind: &'static str,
    },
    // ...
}

// crates/vb_runtime/src/journal/chunk_002.rs — preserved
fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent>;
fn run_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent>;
fn action_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent>;
fn boundary_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<Option<JournalEvent>>;

impl RuntimeJournal for StorageRuntimeJournal {
    fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
        let storage_event = Self::storage_event(event, seq)?;
        self.append_storage_event(&storage_event)?;
        Ok(())
    }
    // ...
}
```

## Signatures (new)

```rust
// crates/vb_runtime/src/error/equality.rs
fn runtime_error_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        // ... existing arms ...
        (
            RuntimeError::UnmappedRuntimeJournalEvent { event_kind: a },
            RuntimeError::UnmappedRuntimeJournalEvent { event_kind: b },
        ) => a == b,
        // ... existing arms ...
        _ => false,
    }
}

// crates/vb_runtime/src/error/display.rs
fn runtime_error_static_message(error: &RuntimeError) -> Option<&'static str> {
    match error {
        // ... existing arms ...
        RuntimeError::UnmappedRuntimeJournalEvent { .. } => Some(
            "unmapped runtime journal event — dispatcher has no mapping for this variant",
        ),
        // ...
    }
}

fn write_runtime_error_dynamic(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match error {
        // ... existing arms ...
        RuntimeError::UnmappedRuntimeJournalEvent { event_kind } => {
            write!(f, "unmapped runtime journal event: {event_kind}")
        }
        // ...
        _ => Ok(()),
    }
}

// crates/vb_runtime/src/error/diagnostics.rs
impl RuntimeError {
    pub const UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE: DiagnosticCode = DiagnosticCode::new(0x2020);
    pub fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            // ... existing arms ...
            Self::UnmappedRuntimeJournalEvent { .. } => Self::UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE,
            // ...
        }
    }
    pub fn runtime_code(&self) -> Option<&'static str> {
        match self {
            // ... existing arms ...
            // no new arm — falls through to None
            // ...
        }
    }
}
```

## Error mapping (caller view)

| Caller site | Path | Existing error | After fix |
| ----------- | ---- | -------------- | --------- |
| `StorageRuntimeJournal::append_sequenced` (`chunk_002.rs:342-346`) | `Self::storage_event(event, seq)?` then `append_storage_event(...)` | `StorageJournalAppend` from Fjall append | Same `?` propagates `UnmappedRuntimeJournalEvent { event_kind }` |
| `QueuedStorageRuntimeJournal::append_sequenced` (`chunk_003.rs:8-16`) | `StorageRuntimeJournal::storage_event(event, seq)?` | Same as above + `UnsupportedAsyncStrictAck` (Strict profile) | Same; `UnsupportedAsyncStrictAck` precedence unchanged |
| `RuntimeShard::append_journal_event` (`shard/impl_parts/chunk_001.rs:194-199`) | `journal.append_sequenced(event, seq)?` | `?` propagates | `?` propagates the new error to whatever called `append_journal_event` (e.g. `handle_resume`) |

## Non-goals

- Do not modify `RuntimeJournalEvent` (preserved `#[non_exhaustive]`, 21 variants).
- Do not modify any of the three layer helpers' bodies or signatures.
- Do not implement Option B (explicit `Resumed -> RunResumed` mapping) in this bead.
- Do not modify `Verification::verus::extern_storage_kind_family.rs` (mirror is up-to-date for the existing `JournalEvent::RunResumed`).
- Do not introduce `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg!` macros.
- Do not change Fjall on-disk format.
- Do not add `String` to the new error variant — `&'static str` only.
- Do not register `From<...>` conversions for the new variant.
- Do not modify the diagnostic-code allocation range (use `0x2020`; do not touch the latent `0x201F` duplicate).
- Do not remove the `QueuedStorageRuntimeJournal::UnsupportedAsyncStrictAck` Strict-profile gate.

## Forbidden post-fix states

- `Ok(JournalEvent::RunFailedEvent)` produced from a non-`RunFailed` `RuntimeJournalEvent`.
- `Ok(JournalEvent::RunFailedEvent)` produced silently without operator-visible error logging when `storage_event` returns `Err`.
- A successful dispatch write to Fjall of any `JournalEvent` whose variant does not correspond to a mapped `RuntimeJournalEvent` arm.
