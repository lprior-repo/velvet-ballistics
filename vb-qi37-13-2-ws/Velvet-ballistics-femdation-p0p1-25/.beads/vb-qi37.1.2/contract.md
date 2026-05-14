# Contract Specification: vb-qi37.1.2

## Context

- **Feature**: Journal slot writes with taint propagation and recovery
- **Domain terms**:
  - `Taint`: Secret propagation marker (`Clean`, `DerivedFromSecret`, `Secret`)
  - `SlotValue`: Runtime slot value (Bool, Null, U64, F64, String, Symbol, List, Object, Blob)
  - `SlotIdx`: Slot index identifier
  - `CoreResult<T>`: `Result<T, CoreError>` with slot-specific errors
  - `RuntimeResult<T>`: `Result<T, RuntimeError>` with journal-specific errors
  - `extra`: Optional byte vector carrying postcard-encoded taint in journal events
- **Assumptions**:
  - Taint is postcard-serializable via serde derives
  - Legacy slot taint inference is a stable fallback for pre-extra encoded events
  - Slot index is always in bounds for the calling frame's slot array
- **Open questions**:
  - None

---

## Contract: `write_slot_with_taint`

**File**: `crates/vb_core/src/frame.rs:229`

### Function Signature

```rust
pub fn write_slot_with_taint(
    &mut self,
    slot: SlotIdx,
    value: SlotValue,
    taint: Taint,
) -> CoreResult<()>
```

### Preconditions

- **PRE-wst-001**: `slot` must be within the bounds of `self.slots` and `self.taint` arrays.
- **PRE-wst-002**: Caller must possess the mutable frame reference (frame isolation enforced by caller).

### Postconditions

- **POST-wst-001**: After successful return, `self.slots[slot]` equals `Some(value)`.
- **POST-wst-002**: After successful return, `self.taint[slot]` equals `taint`.
- **POST-wst-003**: Slot value and taint are written atomically (same index, same call, no partial state visible).
- **POST-wst-004**: On `Err(CoreError::SlotOutOfBounds { slot })`, neither `self.slots` nor `self.taint` are modified.

### Error Taxonomy

| Error Variant | Condition | Semantic Meaning |
|---|---|---|
| `CoreError::SlotOutOfBounds { slot }` | Index outside slot array | Slot index is invalid |
| `CoreError::SlotUninitialized { slot }` | N/A for write path | N/A (write initializes) |

### Invariants

- **INV-wst-001**: For any slot index `i`, after any sequence of `write_slot_with_taint` calls, `slots[i]` and `taint[i]` are always set to the value and taint from the most recent call targeting `i`, or remain their initial state if never written.

---

## Contract: `recovered_slot_taint`

**File**: `crates/vb_storage/src/recovery/replay/summary.rs:428`

### Function Signature

```rust
fn recovered_slot_taint(value: SlotValue, extra: &Option<Vec<u8>>) -> Taint
```

### Preconditions

- **PRE-rst-001**: `value` must be a valid `SlotValue` reconstructed from journal bytes.
- **PRE-rst-002**: `extra` must be either `None` or contain bytes previously written by `postcard::to_allocvec(&taint)`.

### Postconditions

- **POST-rst-001**: If `extra` is `Some(bytes)` and `postcard::from_bytes::<Taint>(&bytes)` succeeds, returns the decoded `Taint`.
- **POST-rst-002**: If `extra` is `None` or decoding fails, returns `legacy_slot_taint(value)`.
- **POST-rst-003**: Return value is always one of `{Clean, DerivedFromSecret, Secret}`.

### Taint Resolution Table (legacy_slot_taint)

| SlotValue variant | Inferred Taint |
|---|---|
| `Bool(false)` | `Clean` |
| `Bool(true)` | `DerivedFromSecret` |
| `Null` | `DerivedFromSecret` |
| All other variants | `Secret` |

### Invariants

- **INV-rst-001**: `recovered_slot_taint` is a pure total function: same `(value, extra)` input always yields same `Taint` output.

---

## Contract: `encoded_slot_taint_extra`

**File**: `crates/vb_runtime/src/journal/chunk_002.rs:192`

### Function Signature

```rust
fn encoded_slot_taint_extra(taint: Taint, extra: Option<Vec<u8>>) -> Option<Vec<u8>>
```

### Preconditions

- **PRE-est-001**: `taint` must be a valid `Taint` variant (`Clean`, `DerivedFromSecret`, `Secret`).
- **PRE-est-002**: `extra` may be `None` or any previously accumulated `Option<Vec<u8>>`.

### Postconditions

- **POST-est-001**: If `extra` is `Some(existing)`, returns `Some(existing)` (preserves existing extra).
- **POST-est-002**: If `extra` is `None`, returns `postcard::to_allocvec(&taint)` if serialization succeeds, or `None` if it fails.
- **POST-est-003**: Return is `Some(Vec<u8>)` containing postcard-encoded taint when `extra` was `None` and encoding succeeded.

### Invariants

- **INV-est-001**: When `extra` is `Some(existing)`, the taint parameter is ignored (preservation semantics).
- **INV-est-002**: Roundtrip property: for any `taint: Taint`, `encoded_slot_taint_extra(taint, None)` yields `Some(bytes)` such that `postcard::from_bytes::<Taint>(&bytes) == Ok(taint)` — when postcard encoding is sound.

---

## Error Taxonomy (Global)

| Error Variant | Crate | Semantic Meaning |
|---|---|---|
| `CoreError::SlotOutOfBounds { slot }` | vb_core | Slot index outside frame slot array |
| `CoreError::SlotUninitialized { slot }` | vb_core | Slot has no value (read-only) |
| `CoreError::InternalInvariantViolation { reason }` | vb_core | Internal state diverged from invariant |
| `RuntimeError::JournalPoisoned` | vb_runtime | Journal mutex poisoned |
| `RuntimeError::JournalAppendFailed` | vb_runtime | Storage append failed |
| `RuntimeError::UnsupportedLiveFrameState` | vb_runtime | Live frame state not supported during recovery |

---

## Verus-Owned Clauses

- **INV-wst-001** (`write_slot_with_taint` atomic write): Rust-local pure frame state machine. Verus spec/proof shows `slots` and `taint` arrays are updated atomically with no partial state observable. `owner_state: 3`
- **INV-rst-001** (`recovered_slot_taint` totality): Pure total function property. Verus spec fn proves deterministic output for all inputs. `owner_state: 3`
- **INV-est-001** (`encoded_slot_taint_extra` preservation): Verus spec fn proves `extra` is returned unchanged when `Some`. `owner_state: 3`

## TLA+-Owned Clauses

- **INV-wst-001** (atomic slot write): Temporal model of journal append shows every `SlotWritten` event is durably recorded before recovery can observe it. Refinement: Rust `write_slot_with_taint` refines TLA+ `WriteSlot` action. `owner_state: 3`
- No other temporal/state-over-time behavior in the 3 scoped functions.

## Non-goals

- Codegen `write_slot_with_journal` behavior (upstream bead)
- `FrameSeedAccumulator::record_slot_write` decode/encode orchestration (upstream bead)
- `StorageRuntimeJournal::append` mutex poisoning recovery (separate concern)
