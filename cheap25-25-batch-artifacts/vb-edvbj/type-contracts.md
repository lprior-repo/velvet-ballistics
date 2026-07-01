# Type Contracts — vb-edvbj

## Existing types — preservation contract

The following types MUST remain unchanged by this bead. List documents the preserved shape, not the change.

| Type | Path | Preserved shape (relevant slice) | Reason preserved |
| ---- | ---- | -------------------------------- | ---------------- |
| `RuntimeJournalEvent` | `crates/vb_runtime/src/journal/chunk_001.rs:13-195` | `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] #[non_exhaustive] pub enum RuntimeJournalEvent`. 21 variants (`RunSubmitted`, `RunAdmission`, `RunFinished`, `RunFailed`, `RunCancelled`, `RunKilled`, `ActionScheduled`, `ActionCompleted`, `ActionScheduledTicket`, `ActionCompletedEnvelope`, `ActionFailed`, `ActionAbandoned`, `WaitScheduled`, `WaitResolved`, `AskScheduled`, `AskAnswered`, `AskTimedOut`, `SlotWritten`, `StepStarted`, `StepSucceeded`, `Resumed`). | Adding a variant is the upstream cause of the silent corruption bug; the fix is at the dispatcher, not the enum. `#[non_exhaustive]` blocks external exhaustive matches from forcing compile-time updates. |
| `JournalEvent` (vb_storage) | `crates/vb_storage/src/events.rs:23` | Includes `RunResumed { run, seq, timestamp: DateTime<Utc> }` (lines 290-297). | Receives layer-helper output; unchanged. |
| `RuntimeJournal::append_sequenced` (trait) | `crates/vb_runtime/src/journal/chunk_001.rs:228-247` | `fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()>;` | Signature unchanged; `?` propagation picks up the new error. |
| `VolatileRuntimeJournal::append` | `crates/vb_runtime/src/journal/chunk_001.rs:281-288` | Bypasses `storage_event` entirely; pushes into a `Vec`. | Unaffected by the bug or the fix. |

## New type — `RuntimeError::UnmappedRuntimeJournalEvent`

### Declaration

```rust
// crates/vb_runtime/src/error/mod.rs — appended after line 202
//
// Typed failure returned by `StorageRuntimeJournal::storage_event` when none of
// the per-layer helpers (`run_storage_event`, `action_storage_event`,
// `boundary_storage_event`) maps the incoming `RuntimeJournalEvent` to a
// `JournalEvent`. Today this fires for `RuntimeJournalEvent::Resumed { .. }`.
// This variant replaces the prior silent fallback that fabricated a
// `JournalEvent::RunFailedEvent`. See RE-019 / vb-edvbj.
UnmappedRuntimeJournalEvent {
    /// `&'static str` name of the unmapped variant, e.g. `"Resumed"`.
    /// Cheap to inspect, copy, format, and equal-compare without owning.
    event_kind: &'static str,
},
```

### Field-level contract

| Field | Type | Invariant |
| ----- | ---- | --------- |
| `event_kind` | `&'static str` | MUST equal the `Rust` identifier of the offending `RuntimeJournalEvent` variant — i.e. one of `"RunSubmitted"`, `"RunAdmission"`, `"RunFinished"`, `"RunFailed"`, `"RunCancelled"`, `"RunKilled"`, `"ActionScheduled"`, `"ActionCompleted"`, `"ActionScheduledTicket"`, `"ActionCompletedEnvelope"`, `"ActionFailed"`, `"ActionAbandoned"`, `"WaitScheduled"`, `"WaitResolved"`, `"AskScheduled"`, `"AskAnswered"`, `"AskTimedOut"`, `"SlotWritten"`, `"StepStarted"`, `"StepSucceeded"`, `"Resumed"`. No allocation; no locale-specific rendering; no run-id or seq attached (those live in the caller's local context and would only feed `tracing`, not the discriminant). |

### Equality

`runtime_error_field_eq` in `crates/vb_runtime/src/error/equality.rs` MUST be extended with a new tuple match arm:

```rust
(
    RuntimeError::UnmappedRuntimeJournalEvent { event_kind: a },
    RuntimeError::UnmappedRuntimeJournalEvent { event_kind: b },
) => a == b,
```

`runtime_error_unit_eq` does NOT need a new tag (this variant is field-bearing).

### Display

`runtime_error_static_message` in `crates/vb_runtime/src/error/display.rs` MUST be extended:

```rust
RuntimeError::UnmappedRuntimeJournalEvent { event_kind } => Some(
    "unmapped runtime journal event — dispatcher has no mapping for this variant",
),
```

The `&'static str` is intentionally NOT interpolated into the static message (keeps the message byte-stable for telemetry and avoids accidental dependency on the call-site's `Display` of the variant name).

`write_runtime_error_dynamic` MUST be extended to include `event_kind` in the alternate dynamic path for callers that want the variant name:

```rust
RuntimeError::UnmappedRuntimeJournalEvent { event_kind } => {
    write!(f, "unmapped runtime journal event: {event_kind}")
}
```

`std::error::Error::source` returns `None` (no preserved source — this is an internal invariant, not a propagated error).

### Diagnostic

`diagnostics.rs` MUST add a fresh `DiagnosticCode` constant and routes:

```rust
pub const UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE: DiagnosticCode = DiagnosticCode::new(0x2020);
```

`diagnostic_code` extension:

```rust
Self::UnmappedRuntimeJournalEvent { .. } => Self::UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE,
```

`runtime_code` MUST return `None` for this variant (no symbolic code analogue — the legacy code set has no equivalent category), to mirror the precedent set by `UnsupportedDurabilityProfile` and `IntrospectionEpochExhausted`.

`symbolic_code` therefore falls through `registered_symbolic_code` and the unrecognised branch returns `SymbolicCode::INTERNAL_INVARIANT`. This is intentional: it mirrors `UnsupportedDurabilityProfile` (which routes the same way) and signals "internal structural invariant, not a runtime data error".

### Conversions

`From<...>` impls in `conversions.rs` are NOT extended. No `From<vb_storage::JournalError>`, no `From<vb_core::errors::CoreError>`, no `From<ResumeError>` is changed. The new variant is reachable only from `storage_event` and propagates via `?` to existing callers.

### Diagnostic-code collision note (open Q for downstream)

`crate::diagnostics.rs:33` and `:44` already both register `0x201F` for `AdmissionCapabilityCountMismatch` and `IntrospectionEpochExhausted`. The bead reserves `0x2020` for `UnmappedRuntimeJournalEvent` to avoid collision with the existing `0x201F` duplicate; the duplicate is **out of scope** for vb-edvbj but MUST be flagged as a finding (see `hazard-analysis.md` H-2).

## Modified type — `StorageRuntimeJournal::storage_event` (signature unchanged, body replaced)

### Signature (preserved)

```rust
fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent>
```

### Body contract (replacement for `chunk_002.rs:270-303`)

```rust
fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent> {
    // 1. Try each per-layer helper exactly once. Cloning is performed by
    //    the per-layer path that consumes the variant (same shape as today).
    let result = match &event {
        // run-layer routes
        RuntimeJournalEvent::RunSubmitted { .. }
        | RuntimeJournalEvent::RunAdmission { .. }
        | RuntimeJournalEvent::RunFinished { .. }
        | RuntimeJournalEvent::RunFailed { .. }
        | RuntimeJournalEvent::RunCancelled { .. }
        | RuntimeJournalEvent::RunKilled { .. }
        | RuntimeJournalEvent::StepStarted { .. }
        | RuntimeJournalEvent::StepSucceeded { .. } => {
            Self::run_storage_event(clone_for_dispatch(&event), seq)
        }
        // action-layer routes
        RuntimeJournalEvent::ActionScheduled { .. }
        | RuntimeJournalEvent::ActionCompleted { .. }
        | RuntimeJournalEvent::ActionScheduledTicket { .. }
        | RuntimeJournalEvent::ActionCompletedEnvelope { .. }
        | RuntimeJournalEvent::ActionFailed { .. }
        | RuntimeJournalEvent::ActionAbandoned { .. } => {
            Self::action_storage_event(clone_for_dispatch(&event), seq)
        }
        // boundary-layer routes
        RuntimeJournalEvent::WaitScheduled { .. }
        | RuntimeJournalEvent::WaitResolved { .. }
        | RuntimeJournalEvent::AskScheduled { .. }
        | RuntimeJournalEvent::AskAnswered { .. }
        | RuntimeJournalEvent::AskTimedOut { .. }
        | RuntimeJournalEvent::SlotWritten { .. }
        | RuntimeJournalEvent::Resumed { .. } => {
            return Self::boundary_storage_event(clone_for_dispatch(&event), seq)?
                // ⟨FIX⟩ no synthetic RunFailed fabrication on None.
                .ok_or_else(|| {
                    let event_kind = RuntimeJournalEvent::Resumed::kind_of(&event);
                    RuntimeError::UnmappedRuntimeJournalEvent { event_kind }
                });
        }
    };
    // 2. Layer is run or action. Both return `Option<JournalEvent>`.
    //    Some(event) → emit it. None → unmapped.
    match result {
        Some(storage_event) => Ok(storage_event),
        None => {
            let event_kind = runtime_journal_event_kind(&event);
            Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind })
        }
    }
}

// Helper: map a `&RuntimeJournalEvent` to the `&'static str` name of its
// variant. Implementation is a non_exhaustive-respecting match that covers
// all 21 declared variants and rejects any future variant at runtime via
// `unreachable!`. (Unreachable-not-panic note: this fn is private; the
// runtime_error_field_eq invariant in §II covers the input space.)
fn runtime_journal_event_kind(event: &RuntimeJournalEvent) -> &'static str {
    match event {
        RuntimeJournalEvent::RunSubmitted { .. } => "RunSubmitted",
        // ... one arm per variant ...
        RuntimeJournalEvent::Resumed { .. } => "Resumed",
        _ => "Unknown",
    }
}
```

The exact location of the inner `match event` for the dispatcher-boundary-layer grouping is permitted to differ (e.g. preserve the original `_ => Self::boundary_storage_event(...)` arm and only swap the post-dispatch fallback). The contract requires: **the only path that returns `Ok(JournalEvent::RunFailedEvent { .. })` is the explicit `RuntimeJournalEvent::RunFailed { run }` arm in `run_storage_event`.**

## Ownership & mutability

No change to ownership / mutability invariants. `storage_event` continues to consume `RuntimeJournalEvent` by value (consumed-clone-into-helper pattern unchanged), takes `seq` by value, and returns a fresh `JournalEvent`.

## Time, async, unsafe, FFI

No change. `storage_event` remains synchronous, non-async, no `unsafe`, no FFI.

## Failure modes (typed)

| Failure | Origin | Returned via |
| ------- | ------ | ------------ |
| `EncodeFailed` | `encoded_slot_taint_extra` from `boundary_storage_event` `SlotWritten` arm | `?` from `boundary_storage_event` (already correct) |
| `UnmappedRuntimeJournalEvent { event_kind }` | All helpers return `None` (or boundary helper returns `None` for an unmapped variant) | New typed return from `storage_event` |
