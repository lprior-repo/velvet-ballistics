# Error Taxonomy — vb-edvbj

## New variant: `RuntimeError::UnmappedRuntimeJournalEvent`

### Shape

```rust
UnmappedRuntimeJournalEvent {
    event_kind: &'static str,
}
```

### Properties

| Property | Value | Precedent |
| -------- | ----- | --------- |
| Variant kind | tuple-struct variant with one `&'static str` payload | `UnsupportedDurabilityProfile { profile_debug: String }` (`mod.rs:193-196`) |
| `Debug` | auto-derived, prints `UnmappedRuntimeJournalEvent { event_kind: "<name>" }` | established `#[derive(Debug)]` |
| `Clone` | auto-derived, copies the `&'static str` reference | established `#[derive(Clone)]` |
| `PartialEq` | field-equality via `runtime_error_field_eq` | new arm required in `equality.rs` |
| `Eq` | unconditional via `impl Eq for RuntimeError {}` | inherited |
| `Display` (static-message path) | `"unmapped runtime journal event — dispatcher has no mapping for this variant"` | new arm required in `display.rs::runtime_error_static_message` |
| `Display` (dynamic path) | `format!("unmapped runtime journal event: {event_kind}")` | new arm required in `display.rs::write_runtime_error_dynamic` |
| `std::error::Error::source` | `None` | inherited default |
| `DiagnosticCode` | `0x2020` (`UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE`) | new constant in `diagnostics.rs` |
| `runtime_code` (legacy string) | `None` | precedent `UnsupportedDurabilityProfile` (`diagnostics.rs:162`) |
| `symbolic_code` | `INTERNAL_INVARIANT` (via `registered_symbolic_code` returning the registered code's symbolic form, falling through to `INTERNAL_INVARIANT`) | precedent `UnsupportedDurabilityProfile` (`diagnostics.rs:101-103`) |
| `From<...>` conversions | unchanged | no new `From` arm |

### Payload domain

`event_kind` MUST be one of the static strings emitted by the contract-prescribed helper:

| `RuntimeJournalEvent` variant | `event_kind` |
| ----------------------------- | ------------ |
| `RunSubmitted { .. }` | `"RunSubmitted"` |
| `RunAdmission { .. }` | `"RunAdmission"` |
| `RunFinished { .. }` | `"RunFinished"` |
| `RunFailed { .. }` | (unreachable — explicit arm) |
| `RunCancelled { .. }` | (unreachable — explicit arm) |
| `RunKilled { .. }` | (unreachable — explicit arm) |
| `ActionScheduled { .. }` | (unreachable — explicit arm) |
| `ActionCompleted { .. }` | (unreachable — explicit arm) |
| `ActionScheduledTicket { .. }` | (unreachable — explicit arm) |
| `ActionCompletedEnvelope { .. }` | (unreachable — explicit arm) |
| `ActionFailed { .. }` | (unreachable — explicit arm) |
| `ActionAbandoned { .. }` | (unreachable — explicit arm) |
| `WaitScheduled { .. }` | (unreachable — explicit arm) |
| `WaitResolved { .. }` | (unreachable — explicit arm) |
| `AskScheduled { .. }` | (unreachable — explicit arm) |
| `AskAnswered { .. }` | (unreachable — explicit arm) |
| `AskTimedOut { .. }` | (unreachable — explicit arm) |
| `SlotWritten { .. }` | (unreachable — explicit arm) |
| `StepStarted { .. }` | (unreachable — explicit arm) |
| `StepSucceeded { .. }` | (unreachable — explicit arm) |
| `Resumed { .. }` | `"Resumed"` |

`Resumed` is the sole variant that the existing helpers map to all-`None` today. Any future variant added without updating all three helpers will appear here with its own name.

### Diagnostic-code allocation

`DiagnosticCode::new(0x2020)` is allocated. This is the next free code in the `0x20xx` range after the `0x2001`-`0x201F` cluster:

- `0x2001` `QUEUE_FULL_CODE`
- `0x2002` `RUN_NOT_FOUND_CODE`
- `0x2003` `ACTIVE_RUN_CAPACITY_EXCEEDED_CODE`
- `0x2004` `RUN_ALREADY_EXISTS_CODE`
- `0x2005` `UNSUPPORTED_OPERATION_CODE`
- `0x2006` `SHUTDOWN_IN_PROGRESS_CODE`
- `0x2007` `JOURNAL_POISONED_CODE`
- `0x2008` `STORAGE_JOURNAL_APPEND_FAILED_CODE`
- `0x2009` `UNSUPPORTED_ASYNC_STRICT_ACK_CODE`
- `0x200A` `FRAME_POOL_UNAVAILABLE_CODE`
- `0x200B` `INVALID_ACTION_COMPLETION_CODE`
- `0x200C` `INVALID_TIMER_FIRE_CODE`
- `0x200D` `UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE`
- `0x200E` `INVALID_RECOVERY_HYDRATION_CODE`
- `0x200F` `COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE`
- `0x2010` `ACTIVE_RUN_CAPACITY_ZERO_CODE`
- `0x2011` `ADMISSION_ARTIFACT_NOT_FOUND_CODE`
- `0x2012` `ADMISSION_CAPABILITY_DENIED_CODE`
- `0x2013` `ENCODE_FAILED_CODE`
- `0x2014` `ADMISSION_ARTIFACT_INVALID_CODE`
- `0x2015` `ADMISSION_HEADER_PERSISTENCE_FAILED_CODE`
- `0x2016` `SECRET_RESULT_NOT_ALLOWED_CODE`
- `0x2017` `IPC_PAYLOAD_SIZE_EXCEEDED_CODE`
- `0x2018` `ADMISSION_ARTIFACT_DIGEST_MISMATCH_CODE`
- `0x2019` `ADMISSION_ARTIFACT_STALE_CODE`
- `0x201A` `ADMISSION_DIGEST_MISMATCH_CODE`
- `0x201B` `ENGINE_DRIVE_FAILED_CODE`
- `0x201C` `SHARD_NOT_FOUND_CODE`
- `0x201D` `MIGRATE_SELF_CODE`
- `0x201E` `JOURNAL_FULL_CODE`
- `0x201F` `ADMISSION_CAPABILITY_COUNT_MISMATCH_CODE` AND `INTROSPECTION_EPOCH_EXHAUSTED_CODE` (duplicate — out of scope for vb-edvbj; see H-2)
- `0x2020` **`UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE`** ← this bead

### Why `event_kind: &'static str` and not `String`

- **No allocation in the dispatch hot path.** The variant name is a literal in the dispatcher and is a literal cast of an identifier; allocating a `String` would force every unmapped-variant dispatch to allocate. Holzman-Rust Power-of-Ten rule 6 prohibits runtime allocation in error paths where a literal suffices.
- **Stable across calls.** `String` would not be stable across `Display`-prefix boundaries; `&'static str` is binary-equal regardless of formatting.
- **Equality is a pure `&str`-compare.** No `Arc` indirection, no `String::clone` on partial-eq.

### Why this taxonomy

- It mirrors `UnsupportedDurabilityProfile { profile_debug: String }` (`mod.rs:193-196`) as the canonical VB-NOORE precedent for "you gave me a value that is not in the supported set; here's what it was". The payload naming is from the same precedent (`profile_debug` is stringly; we adopt `event_kind` because it is a `&'static str` of the enum identifier).
- It is **not** a `Result<JournalEvent, _>` polymorphic variant because `RuntimeError` is the `Result`-type at this boundary and adding the variant alongside existing ones keeps the `?`-propagation pipeline uniform.
- It is **not** a `From<vb_storage::JournalError>` conversion because the unmapped condition originates in the runtime layer, not the storage layer.

## Open domain questions flagged for downstream

None. The bead's literal text says "Locate the match arm that handles `_ =>` and remove it." The replacement decision (Option A) is fixed by the bead's stated contract focus: **Replace `Ok(JournalEvent::RunFailedEvent)` wildcard with typed `RuntimeError::UnmappedRuntimeJournalEvent { event_kind }`**.
