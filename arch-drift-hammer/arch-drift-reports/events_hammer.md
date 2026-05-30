# Architectural Drift Report: `vb_storage/src/events.rs`

**Agent**: architectural-drift (arch-drift-hammer)
**File**: `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/events.rs`
**Line Count**: 478 lines
**Status**: VIOLATION — exceeds 300-line hard limit by 178 lines (59% over)

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Actual lines | 478 |
| Limit | 300 |
| Excess | +178 lines (+59%) |
| Violation | **YES** |

---

## 2. Storage Event Responsibilities Map

### 2.1 Types Defined

| Type | Kind | Lines | Responsibility |
|------|------|-------|----------------|
| `DurableActionOutcome` | `#[repr(u8)]` enum, 1 variant | 12–18 | Terminal action outcome discriminant captured by durable completion envelopes |
| `JournalEvent` | `#[non_exhaustive]` sum type, 21 variants | 21–272 | Compact binary journal event — the core durable record for the append-only journal |

### 2.2 `JournalEvent` Variants — 9 Lifecycle Groups

| Group | Variants | Count |
|-------|----------|-------|
| **Run lifecycle** | `RunAccepted`, `RunAdmission`, `RunCancelled`, `RunKilled`, `RunFinished`, `RunFailedEvent`, `RunResumed`, `RunRetried`, `RunAnswered` | 9 |
| **Step lifecycle** | `StepStarted`, `StepSucceeded` | 2 |
| **Action scheduling** | `ActionScheduled`, `ActionScheduledTicket` | 2 |
| **Action completion** | `ActionCompletedEvent`, `ActionCompletedEnvelope` | 2 |
| **Action failure** | `ActionFailedEvent` | 1 |
| **Slot** | `SlotWrittenEvent` | 1 |
| **Wait/Ask/Retry** | `WaitScheduledEvent`, `AskScheduledEvent`, `AskAnsweredEvent`, `RetryScheduledEvent` | 4 |
| **Total** | | **21** |

### 2.3 Methods on `JournalEvent` — 6 Methods

| Method | Lines | Responsibility |
|--------|-------|----------------|
| `run_id()` | 275–301 | Extracts `RunId` via 21-arm match |
| `seq()` | 303–332 | Extracts `EventSeq` via 21-arm match |
| `record_kind()` | 334–361 | Maps variant → `RecordKind` discriminant |
| `slot_value()` | 363–399 | Fallible decode of optional `SlotValue` from `SlotWrittenEvent` |
| `attempt()` | 401–432 | Extracts `Option<u16>` attempt number via 21-arm match |
| `is_valid()` | 434–477 | Validates structural invariants (non-zero run, non-max seq, non-zero attempt) |

---

## 3. Primitive Obsession Violations

### 3.1 Clean Areas (No Violations)

The following are proper newtypes, NOT primitive obsession:

| Type | Definition | Verdict |
|------|-----------|---------|
| `RunId`, `ActionId`, `SlotIdx`, `StepIdx`, `EventSeq` | `numeric_id!` macro (`#[repr(transparent)]` wrapper over `u64`/`u16`) in `vb_core/src/ids/mod.rs` | ✅ Clean |
| `WorkflowDigest` | `#[repr(transparent)]` wrapper over `[u8; 32]` | ✅ Clean |
| `CapabilitySet` | `vb_core/src/capability.rs` | ✅ Clean |
| `RuntimePolicy` | Used as a value type | ✅ Clean |
| `Taint` | `vb_core/src/value.rs` enum | ✅ Clean |
| `ActionTicket` | `vb_core/src/action.rs` struct | ✅ Clean |
| `ConstValue` | Used as value type | ✅ Clean |
| `RecordKind` | `vb_storage/src/records.rs` `#[repr(u16)]` enum | ✅ Clean |
| `DateTime<Utc>` | Chrono type, acceptable for timestamps | ✅ Acceptable |

### 3.2 Violations

#### V-001: `ActionCompletedEnvelope::value` — Naked `Vec<u8>`

**Location**: Line 120

```rust
/// Encoded output value bytes.
value: Vec<u8>,
```

**Problem**: A raw byte vector carries no semantic meaning. Callers must remember what encoding was used (postcard), what the bytes represent (output value), and what the invariants are.

**Recommendation**: Newtype as `EncodedOutput(Vec<u8>)` with:
- A `try_decode<T: serde::de::DeserializeOwned>(&self) -> Result<T, JournalError>` method
- A `encoded_len(&self) -> u32` accessor that returns `self.0.len() as u32` with bounds check
- Invariant: `encoded_len == self.0.len() as u32` enforced at construction

#### V-002: `ActionCompletedEnvelope::encoded_len` — Raw `u32` Without Invariant Binding

**Location**: Line 122

```rust
/// Encoded output byte length validated before persistence.
encoded_len: u32,
```

**Problem**: This field is documented as "validated before persistence" but nothing in the type system enforces that `encoded_len == value.len() as u32`. A mismatch is silent data corruption.

**Recommendation**: Merge into `EncodedOutput` newtype above, eliminating the separate field and the class of bugs it enables.

#### V-003: `ActionCompletedEnvelope::value_digest` — Untyped `[u8; 32]`

**Location**: Line 126

```rust
/// BLAKE3 digest of `value` used to reject divergent duplicate evidence.
value_digest: [u8; 32],
```

**Problem**: A raw 32-byte array carries no type-level proof that this is a BLAKE3 digest and not some other 32-byte key. Type-level confusion between digests and keys is a known source of security bugs.

**Recommendation**: Newtype as `OutputDigest([u8; 32])` with:
- Constructor that takes raw bytes and a domain separator
- `ValueDigester` helper that computes BLAKE3 with proper domain separation tag

#### V-004: `RunCancelled::reason` — `Option<String>`

**Location**: Line 210

```rust
/// Optional cancellation reason.
reason: Option<String>,
```

**Problem**: `String` is a primitive. "Cancellation reason" is a domain concept that should be a value object. Additionally, `String` in a journal record creates encoding/framing questions (UTF-8, max length, etc.).

**Recommendation**: Newtype as `CancellationReason(InternedString)` or `CancellationReason { code: u16, detail: Option<SmallString<64>> }` depending on the intended taxonomy of cancellation reasons.

#### V-005: `SlotWrittenEvent::value` — `Option<Vec<u8>>`

**Location**: Line 150

```rust
/// Encoded slot value bytes (postcard-encoded `SlotValue`), if captured.
value: Option<Vec<u8>>,
```

**Problem**: Same class as V-001. A raw `Option<Vec<u8>>` with no type-level binding to the encoding format or the semantic slot type.

**Recommendation**: Newtype as `CapturedSlotValue(Option<EncodedSlotValue>)` where `EncodedSlotValue` wraps `Vec<u8>` and provides the decode method.

#### V-006: `SlotWrittenEvent::extra` — `Option<Vec<u8>>`

**Location**: Line 153

```rust
/// Versioned slot-write extra envelope, or legacy encoded frame extra data.
#[serde(default)]
extra: Option<Vec<u8>>,
```

**Problem**: The comment admits "legacy encoded frame extra data" — this is a type-level confession ofSchema drift. The raw bytes have no semantic type.

**Recommendation**: Newtype as `SlotWriteExtra(Option<Vec<u8>>)` and add a version discriminant. Or eliminate the legacy path entirely if possible.

---

## 4. `#[non_exhaustive]` Assessment

Both `DurableActionOutcome` (line 14) and `JournalEvent` (line 22) are `#[non_exhaustive]`. This is appropriate for a journal event type that may evolve, but it means external code cannot match exhaustively. This is a deliberate trade-off, not a violation.

---

## 5. Method Quality Assessment

### 5.1 `run_id()`, `seq()`, `attempt()` — 21-Arm Match Repetition

Each method repeats the full 21-variant match. This is mechanical boilerplate, not business logic. The 21 arms are identical in structure (extract one field).

**Refactor**: These could be derived automatically via a macro, or the enum could be restructured to share a common `RunMeta` struct that all variants embed, collapsing the three methods into one.

### 5.2 `record_kind()` — Canonical Mapping

This method is the storage contract bridge. It is well-structured and maps cleanly to `RecordKind`. No issues.

### 5.3 `slot_value()` — Correct Error Handling

**Good**:
- Explicit size check before decode (bounds validation)
- Uses `postcard::from_bytes` with explicit error mapping
- Distinguishes `Ok(None)` (absent) from `Ok(Some(_))` (decoded)
- Uses `#[must_use]` annotation

**Note**: The `Err(JournalError::PostcardDecodeFailed)` discards the postcard error details. This is acceptable (no leaking internal details to journal layer) but worth noting.

### 5.4 `is_valid()` — Correct Invariants

**Good**:
- Validates `run_id != 0` (zero is placeholder)
- Validates `seq != u64::MAX` (overflow sentinel)
- Validates attempt numbers are non-zero when present

**Note**: The check `run_id().get() == 0` (line 446) uses the `.get()` accessor on the newtype. This is correct — the validation logic uses the raw value but the types are properly wrapped.

---

## 6. Scott Wlaschin DDD Assessment

### 6.1 What Works

| Principle | Status | Notes |
|-----------|--------|-------|
| Types model domain concepts | ✅ | `JournalEvent` models the workflow event lifecycle |
| No primitive obsession for IDs | ✅ | All IDs are newtypes via `numeric_id!` macro |
| Exhaustive enums for state | ✅ | 21 variants cover all event categories |
| `#[non_exhaustive]` for evolvability | ✅ | Correct use of non-exhaustive |
| Parse, don't validate | ⚠️ | `is_valid()` validates after construction; `encoded_len` invariant not enforced at type level |
| No invalid states representable | ❌ | `value.len() != encoded_len as usize` is representable (V-002) |

### 6.2 Workflow Modeling Gap

`JournalEvent` is a **data** enum, not a **workflow** enum. Each variant carries the raw fields, but the *transitions* between states are not modeled as functions. For example:
- There is no `JournalEvent::next_state(&self) -> NextState` transition function
- There is no `JournalEvent::can_transition_to(&self, other: &JournalEvent) -> bool`
- The lifecycle ordering constraints are enforced elsewhere (in the codec/replay layer), not in this type

This is not necessarily a violation — the journal is a storage boundary, not a workflow engine — but it means the **valid state transitions** are not encoded in the type system here.

---

## 7. Required Refactors

### 7.1 Mandatory (for line count compliance)

| # | Action | Target Lines | Split Into |
|---|--------|-------------|------------|
| M1 | Extract `DurableActionOutcome` to `events/durable_outcome.rs` | ~7 lines | `events/durable_outcome.rs` |
| M2 | Extract run-lifecycle variants to `events/run_lifecycle.rs` | ~3 event variants | `events/run_lifecycle.rs` |
| M3 | Extract step-lifecycle variants to `events/step_lifecycle.rs` | ~2 event variants | `events/step_lifecycle.rs` |
| M4 | Extract action/slot variants to `events/action_lifecycle.rs` | ~4 event variants | `events/action_lifecycle.rs` |
| M5 | Extract `JournalEvent::run_id`, `seq`, `attempt` to a macro or derive | 3 × ~21 arms | Reduce match boilerplate via `events_impl!` macro |
| M6 | Reduce `impl JournalEvent` block via macro | 204 lines of impl | `events/journal_event_impl.rs` |

### 7.2 Recommended (for primitive obsession cleanup)

| # | Action | Fix |
|---|--------|-----|
| R1 | Wrap `Vec<u8>` in `ActionCompletedEnvelope::value` | `EncodedOutput(Vec<u8>)` newtype |
| R2 | Eliminate `encoded_len` field | Merge into `EncodedOutput` newtype |
| R3 | Wrap `[u8; 32]` in `value_digest` | `OutputDigest([u8; 32])` newtype |
| R4 | Wrap `Option<Vec<u8>>` in `SlotWrittenEvent::value` | `CapturedSlotValue` newtype |
| R5 | Wrap `Option<Vec<u8>>` in `SlotWrittenEvent::extra` | `SlotWriteExtra` newtype with version |
| R6 | Replace `Option<String>` in `RunCancelled::reason` | `CancellationReason` value object |

---

## 8. Summary

| Category | Finding |
|----------|---------|
| **Line Count** | 478 lines — **VIOLATION** (+59%) |
| **Primitive Obsession** | 6 violations — 4 × naked `Vec<u8>`, 1 × naked `u32` (unbound invariant), 1 × `Option<String>` |
| **DDD Type Safety** | Core IDs are clean; envelope/slot bytes are not |
| **Workflow Modeling** | Data enum; transitions not encoded in type |
| **Error Handling** | `slot_value()` is correctly implemented |
| **Exhaustive Safety** | `#[non_exhaustive]` used correctly |

---

## 9. Recommendations Priority

1. **CRITICAL**: Split file to comply with 300-line limit
2. **HIGH**: Address V-001, V-002, V-003 (action completion envelope bytes) — highest data corruption risk
3. **MEDIUM**: Address V-004, V-005, V-006 (slot write / cancellation reason)
4. **LOW**: Consider `JournalEvent` macro derive for `run_id`/`seq`/`attempt` to eliminate 63 arms of mechanical match boilerplate

---

*Report generated: 2026-05-29*
*Agent: arch-drift-hammer (JJ workspace)*
