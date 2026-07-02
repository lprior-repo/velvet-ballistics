# Type Contracts — vb-qxjgx

## 1. Principles Applied (Holzman + Wlaschin)

- **No primitive obsession.** Every wire id, kind discriminant, envelope tag,
  and family kind is encoded as a typed enum arm or a newtype — never a raw
  `u16` or `bool` flowing past the codec boundary.
- **No boolean behavior flags.** Back-compat is a typed predicate
  (`IsLegacyEnvelopeFor { StepSucceeded }`), not a flag.
- **Parse external input once at the boundary.** `decode_journal_event` is
  the only entrypoint into an untrusted `JournalEvent`; it produces the
  typed `JournalEvent` value plus guarantees parity + validity.
- **No `Option<LifecycleState>`.** Step-lifecycle progress is a typestate
  encoded in the variant payload (one variant per transition), not in flags.
- **No `Result` for non-domain outcomes inside the core.** Codec errors
  surface as the existing `JournalError` enum; the new dual-tag handling
  routes through one of the existing variants (`RecordKindPayloadMismatch`)
  with refined inputs.
- **No `unsafe`, no `unwrap`/`expect`, no `panic`, no `dbg`/`todo`/
  `unimplemented` in the contract surface.** All listed downstream symbols
  are `#[forbid(unsafe_code)]` safe-Rust functions; the contract forbids the
  forbidden-construct anti-patterns even when an audit-by-tool is not in
  scope.
- **No unchecked arithmetic.** The id 33 must be a literal const in every
  table; no `RecordKind::index() + 1` shortcuts.

## 2. New / Changed Typed Surfaces

### 2.1 New `RecordKind` variant

```rust
/// crates/vb_storage/src/records.rs — added arm
#[non_exhaustive]
pub enum RecordKind {
    // ... existing variants unchanged ...
    /// Step completed and wrote its output slot.
    ///
    /// Distinct from `SlotWritten = 12` because the event also closes the
    /// step lifecycle. Read-side parity accepts legacy envelope id 12 from
    /// pre-fix journals; write-side ALWAYS emits id 33.
    StepSucceeded = 33,
}
```

- Field type: `pub enum RecordKind` (`#[repr(u16)]`, `#[non_exhaustive]`).
  New arm is exhaustive on the closed surface.
- Wire id: `33` (`u16`). Free per `delivery-scope.jsonl` and `codebase-map.md`.
- Visibility: `pub` (re-export from `vb_storage` crate root).
- No `from(id: u16) -> Option<RecordKind>` constructor is added; rely on
  `id()` + `is_known_record_kind` for symmetry with existing arms.

### 2.2 Updated `RecordKind::id()` arm

```rust
impl RecordKind {
    pub const fn id(self) -> u16 {
        match self {
            // ...
            Self::StepSucceeded => 33,
            // ...
        }
    }
}
```

- Determinism: pure `match`, no fallback. Adding the arm is a compile-time
  exhaustiveness check; removing it is a `non_exhaustive` warning,
  not a panic.

### 2.3 Updated `JournalEvent::record_kind()` arm

```rust
// crates/vb_storage/src/events.rs:406 — split the OR pattern
pub const fn record_kind(&self) -> RecordKind {
    match self {
        // ...
        Self::StepSucceeded { .. } => RecordKind::StepSucceeded,
        Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
        // ...
    }
}
```

- New shape: two distinct arms, no OR pattern. Each variant has exactly
  one canonical `RecordKind`.
- The collapse `Self::StepSucceeded | Self::SlotWrittenEvent => SlotWritten`
  is the line being removed; the change is local to one `match` arm and
  does not alter any other arm.

### 2.4 New `IsLegacyEnvelopeFor` predicate

For the back-compat lane. The contract requires that legacy tolerance be
expressed as a typed relationship, not a `bool` slot in a struct.

```rust
/// crates/vb_storage/src/codec/kind_parity.rs — typed relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyEnvelopeBinding {
    /// No legacy envelope binding; parity is exact match.
    Exact,
    /// The payload variant accepts an envelope id from a closed legacy set.
    Legacy { accepted_ids: &'static [u16] },
}

impl JournalEvent {
    /// Returns the legacy envelope bindings (if any) for this payload variant.
    pub const fn legacy_envelope_bindings(&self) -> LegacyEnvelopeBinding {
        match self {
            Self::StepSucceeded { .. } => LegacyEnvelopeBinding::Legacy {
                accepted_ids: &[12, 33],
            },
            _ => LegacyEnvelopeBinding::Exact,
        }
    }
}
```

- The set of legacy envelopes is per-variant. `StepSucceeded` has the
  `{12, 33}` double-bind; every other variant is `Exact`.
- This replaces the silent "envelope==12 happens to mean StepSucceeded" trap.

### 2.5 Updated `validate_journal_event_record_kind` (read path)

```rust
// crates/vb_storage/src/codec/mod.rs — parity check refit
pub fn validate_journal_event_record_kind(
    envelope: &RecordEnvelope,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    let payload_kind = event.record_kind().id();
    let envelope_kind = envelope.record_kind;
    let accepted = match event.legacy_envelope_bindings() {
        LegacyEnvelopeBinding::Exact => envelope_kind == payload_kind,
        LegacyEnvelopeBinding::Legacy { accepted_ids } => {
            accepted_ids.contains(&envelope_kind)
        }
    };
    if accepted {
        Ok(())
    } else {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        })
    }
}
```

- The default behavior is parity exact match. The ONLY deviation is the
  `StepSucceeded` double-bind.
- This is the single back-compat seam. Other call sites of
  `JournalEvent::record_kind()` (write path, summary aggregation,
  observation normalization) see the new id and DO NOT need changes.

### 2.6 Updated `EnforceKindParity for JournalEvent` (codec parity)

```rust
// crates/vb_storage/src/codec/kind_parity.rs — parity refit
impl EnforceKindParity for crate::JournalEvent {
    fn enforce_kind_parity(
        envelope: &RecordEnvelope,
        value: &Self,
    ) -> Result<(), JournalError> {
        // StepSucceeded accepts envelope 12 (legacy) or 33 (canonical).
        // All other variants require envelope == payload.record_kind().id().
        let payload_kind = value.record_kind().id();
        let accepted = match value {
            Self::StepSucceeded { .. } => matches!(envelope.record_kind, 12 | 33),
            _ => envelope.record_kind == payload_kind,
        };
        if !accepted {
            return Err(JournalError::RecordKindPayloadMismatch {
                envelope_kind: envelope.record_kind,
                payload_kind,
            });
        }
        if !value.is_valid() {
            return Err(JournalError::InvalidEvent);
        }
        Ok(())
    }
}
```

- Mirrors `validate_journal_event_record_kind` and is a defense-in-depth
  self-check inside the canonical decoder.
- Default `EnforceKindParity` impls (`WorkflowSourceRecord`,
  `CompiledIrRecord`, `BlobRecord`, `RunSnapshot`, `RunHeaderRecord`) are
  unchanged.

### 2.7 Updated `is_known_record_kind` (codec validation)

```rust
// crates/vb_storage/src/codec/validation.rs — add id 33
pub(crate) const fn is_known_record_kind(kind: u16) -> bool {
    matches!(kind, 1 | 2 | 3 | 10..=29 | 30 | 31 | 32 | 33 | 40 | 50)
}
```

- Id 33 joins the closed known set. Existing 28-member golden array at
  `kani_record_kind.rs:252-255` MUST be extended.

### 2.8 Updated `validate_kind_family` (codec validation)

```rust
// crates/vb_storage/src/codec/validation.rs — MAGIC_JOURNAL_EVENT kind
pub(crate) fn validate_kind_family(magic: u32, kind: u16) -> Result<(), JournalError> {
    let valid = match magic {
        MAGIC_WORKFLOW_SOURCE => kind == RecordKind::WorkflowSource.id(),
        MAGIC_COMPILED_ARTIFACT => kind == RecordKind::CompiledIr.id(),
        MAGIC_JOURNAL_EVENT => {
            matches!(kind, 10..=29)
                || kind == RecordKind::WaitResolved.id()
                || kind == RecordKind::ActionAbandoned.id()
                || kind == RecordKind::StepSucceeded.id()
        }
        MAGIC_SNAPSHOT => kind == RecordKind::Snapshot.id(),
        MAGIC_BLOB => kind == RecordKind::Blob.id(),
        MAGIC_INDEX_RECORD => matches!(kind, 3 | 50),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(JournalError::RecordKindFamilyMismatch { magic, kind })
    }
}
```

- The `MAGIC_JOURNAL_EVENT` family now admits `33` alongside `31` and `32`.

### 2.9 Durability matrix rows (storage contract projection)

The matrix at `crates/vb_runtime/src/durability_matrix.rs:70-204` encodes
the per-primitive emission contract. The Step-Success rows that currently
use `RecordKind::SlotWritten` MUST be migrated to `RecordKind::StepSucceeded`.
The migration is semantic, not wire-only — `set/do/choose/for_each/
parallel/collect/aggregate/repeat/wait/ask` all close a step with a
`StepSucceeded` event before the resulting slot-binding is observable to
durability consumers.

The `SlotWritten` arm is preserved for purely slot-binding rows (none
currently exist as stand-alone records; the embedded "slot written as
part of step success" is what the matrix calls StepSucceeded).

### 2.10 Kani hard-coded set

The exhaustive harness predicate at
`kani_record_kind.rs:265-289` and the golden array at
`kani_record_kind.rs:252-255` MUST be extended to include 33. The closing
assertion `is_valid_journal_kind` extends to `(10..=29) ∪ {31, 32, 33}`.

### 2.11 Flux refinements (DISABLED lane)

`flux_validation.rs:14,33` currently refine a model hard-coding
`{10..=29} ∪ {31, 32}`. The contract requires these literals be updated to
include `33` even though the module is gated off (per `codec/mod.rs:184-186`
and the `vb-b8i8f` decision). The artifact stays in source for future
re-enable.

### 2.12 Proptest id→kind generators

`proptests.rs:62,148` and `proptest_storage.rs:126` map `u16 → RecordKind`
for property cases. The mapping MUST add an arm `33 => RecordKind::StepSucceeded`
and the `proptest_storage.rs:115` selection array MUST include `33`.

## 3. Forbidden Type-Level Patterns

The contract explicitly forbids the following (downstream review MUST
reject any of these):

| Pattern                                                              | Why forbidden                                                                 |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `Self::StepSucceeded { .. } \| Self::SlotWrittenEvent { .. } => …`  | The bug pattern; collapses two semantics onto one discriminant.               |
| `bool` flag discriminating StepSucceeded vs SlotWritten in any parser | Replaces the typed envelope id with a runtime false/true.                     |
| `Option<EnvelopeSource>` on `JournalEvent`                            | Back-compat is bounded per-variant; runtime should not carry a value.         |
| `From<u16> for RecordKind`                                           | Closed conversion would let `12` resolve to two variants; no conversion.     |
| New id `33` for `RecordKind::StepSucceeded` without paired `events.rs:406` split | Decouples the wire change from the semantic map; round-trip breaks.      |
| Hand-rolled `match` on `envelope.record_kind` in non-codec paths     | Bypasses the typed `record_kind()` authority; MUST use the typed accessor.   |

## 4. Verification Type Surfaces (Sketch)

These are the type-level proof surfaces the contract expects downstream
proof-writer and test-writer to instantiate; this section is non-normative.

```rust
// Sketch only — proof-writer owns the concrete harness:
// 1. For any u16 kind value, parity accepts iff the variant's accepted set
//    contains the envelope id.
// 2. For any u16 kind value, validate_kind_family(Je_magic, kind) is
//    consistent with the journal-kind set including 33.
// 3. For any valid JournalEvent payload, is_valid() && envelope.id ∈
//    accepted_set implies round-trip succeeds.
// 4. For any (legacy envelope 12, StepSucceeded payload), decode succeeds.
// 5. For any (envelope 12, SlotWrittenEvent payload), decode succeeds.
// 6. For any (envelope 33, SlotWrittenEvent payload), decode rejects as
//    RecordKindPayloadMismatch.
// 7. For any (envelope 33, StepSucceeded payload), decode succeeds and is
//    the preferred writer-emitted form.
```

Sketch points 4 and 7 together encode the dual-bind invariant; 6 encodes
the cross-bind rejection. The proof obligations are owned by
proof-planner and proof-writer; this contract only states that the type
shapes must support such obligations.
