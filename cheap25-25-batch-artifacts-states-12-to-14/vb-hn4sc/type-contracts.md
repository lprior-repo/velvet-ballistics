# Type Contracts — vb-hn4sc

bead_id: vb-hn4sc
phase: 3 (rust-contract)
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
captured_at: 2026-07-01T15:31:00Z
authoring_agent: rust-contract

This artifact specifies the type-level contracts introduced or extended by the byte-budget gate on `JournalWriterQueue::flush_batch`. Every shape, transition, and parse boundary is listed so proof and implementation can lock onto a single type model.

## 1. Newtypes and Smart Constructors

### 1.1 `EncodedRecordLength` (new)

```text
pub struct EncodedRecordLength(pub u64);
```

- **Invariant:** `0 < self.0 <= MAX_ENCODED_RECORD_BYTES`, where `MAX_ENCODED_RECORD_BYTES = RECORD_HEADER_BYTES + MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 60 + 1_048_576 = 1_048_636`.
- **Smart constructor:** `EncodedRecordLength::new(value: u64) -> Result<Self, JournalError>` returning `Err(JournalError::InvalidConfig { field: "encoded_record_length", reason: "exceeds MAX_ENCODED_RECORD_BYTES" })` if `value > MAX_ENCODED_RECORD_BYTES`. Returning `Err(JournalError::InvalidConfig { field: "encoded_record_length", reason: "must be non-zero" })` if `value == 0`.
- **Derives:** `Debug, Clone, Copy, PartialEq, Eq`.
- **Rationale:** prevents mixing *payload basis* with *encoded basis* in the gate arithmetic; the existing `JournalWriteBatch::append_event` uses raw `u64` for `staged_bytes`, but the queued path's `Value Object` enrichment is allowed because the gate is implemented in the contract stage (not a Verus spec yet).

### 1.2 `AccumulatedFlushBytes` (new)

```text
pub struct AccumulatedFlushBytes(pub u64);
```

- **Invariant:** `self.0 == 0` initially; after each successful `add`, `self.0` equals the sum of all staged `EncodedRecordLength`s in the current flush.
- **Operations:**
  - `pub const ZERO: Self = Self(0);`
  - `pub fn add(self, other: EncodedRecordLength) -> Result<Self, JournalError>` — performs `checked_add`; returns `Err(JournalError::JournalBatchBytesExceeded { attempted: u64::MAX, limit: u64 })` on overflow where `limit` is the active byte budget.
  - `pub fn would_exceed(self, next: EncodedRecordLength, limit: u64) -> bool` — `true` iff `self.0 + next.0 > limit` *without* mutating `self`.
- **Derives:** `Debug, Clone, Copy, PartialEq, Eq`.
- **Rationale:** separates the *decision* (`would_exceed`) from the *commit* (`add`) so the gate can be expressed as a pure predicate suitable for unit tests, proptest, and a future Verus spec.

### 1.3 `JournalBatchByteBudget` (alias for the active limit)

```text
pub type JournalBatchByteBudget = u64;
```

- Alias only; no smart constructor at the type level (the budget is read directly from `StorageLimits::max_journal_batch_bytes`).
- **Invariant at use site:** `budget >= MAX_ENCODED_RECORD_BYTES` is a *recommended* configuration but not enforced at the type level; a `budget < MAX_ENCODED_RECORD_BYTES` is a legal-but-degenerate configuration that rejects the first non-empty event.

## 2. Extended Types

### 2.1 `StorageLimits` (extended)

```text
pub struct StorageLimits {
    pub max_journal_event_payload_bytes: u32,  // existing
    pub max_journal_batch_bytes: u64,         // NEW
}
```

- **Default:**
  ```text
  pub const DEFAULT: Self = Self {
      max_journal_event_payload_bytes: crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,  // 1_048_576
      max_journal_batch_bytes: crate::storage_constants::DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER, // 1_048_636
  };
  ```
  where `DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER = RECORD_HEADER_BYTES + DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 60 + 1_048_576 = 1_048_636`. The existing `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576` (payload basis, used by `JournalWriteBatch`) is preserved unchanged; the new constant is what `StorageLimits::DEFAULT.max_journal_batch_bytes` references because the queued contract uses the *encoded* basis (60-byte header + payload).
- **Source-compat:** the new field is added with `DEFAULT` populated; every existing caller (`vb_cli::ipc_serve`, `RuntimeJournalConfig::shared_journal`, `workspace_tests`, ~30 storage tests) passing `StorageLimits::DEFAULT` keeps its current behavior because the default accommodates at least one max-size event (matching the existing `kani_vb_vzcuf_ps007::check_bridge_accommodates_single_event` evidence).
- **Cross-crate invariant:** `StorageLimits::DEFAULT.max_journal_batch_bytes == vb_core::max_journal_batch_bytes()`. Already proven by `kani_vb_vzcuf_ps007::check_default_batch_byte_limit`; the contract preserves that binding.
- **Derives preserved:** `Debug, Clone, Copy, PartialEq, Eq`.

### 2.2 `JournalWriterQueue` (extended)

The queue struct grows ONE non-state field: a copy of the byte budget read at construction. This makes the budget available to the gate without re-reading `StorageLimits` on every flush.

```text
pub struct JournalWriterQueue {
    state: Mutex<JournalWriterQueueState>,    // unchanged
    capacity: usize,                          // unchanged
    batch_size: usize,                        // unchanged
    byte_budget: u64,                         // NEW: from StorageLimits::max_journal_batch_bytes
}
```

- **Invariant at construction:** `byte_budget == limits.max_journal_batch_bytes`. Captured by a debug assertion in `with_contracts`.
- **Invariant at all times:** `byte_budget` is immutable post-construction; no public setter exists. Per-flush arithmetic references `self.byte_budget` directly.

### 2.3 `JournalWriterQueue::flush_batch` (extended signature)

The signature does NOT change (preserves source-compat). The return type gains nothing new; the byte-budget rejection rides on the existing `Result<_, JournalError>`.

```text
pub fn flush_batch(&self, journal: &FjallJournal) -> Result<JournalWriterFlushReport, JournalError>;
```

- **Returns `Err(JournalError::JournalBatchBytesExceeded { attempted, limit })`** when the next staged event would push `accumulated_bytes + next_event_encoded_len > self.byte_budget`.
- **`limit`** is `self.byte_budget`.
- **`attempted`** is the sum that would have resulted, i.e. `accumulated_bytes + next_event_encoded_len`, or `u64::MAX` on `checked_add` overflow.

### 2.4 `JournalError::JournalBatchBytesExceeded` (reused, not extended)

```text
#[error("journal batch byte budget exceeded: attempted {attempted} > limit {limit}")]
JournalBatchBytesExceeded { attempted: u64, limit: u64 },
```

- **No new variant.** No new diagnostic code. `JOURNAL_BATCH_BYTES_EXCEEDED_CODE = 0x4022` is reused.
- **Display condition:** `attempted > limit` always holds for `Ok`-rejected cases (overflow case uses `attempted = u64::MAX` which is also `> limit` for any realistic `limit`).

## 3. Parsers at Boundaries

The byte gate operates on values already produced by `encode_record`, which is the canonical parser at the storage encoding boundary. There is no new parser introduced by this bead.

- **Boundary 1 (encode).** `encode_record(MAGIC_JOURNAL_EVENT, kind, seq, event, MAX_PAYLOAD) -> Result<Vec<u8>, JournalError>` at `crates/vb_storage/src/codec.rs`. Returns the full encoded record including the 60-byte header.
- **Boundary 2 (size basis).** The gate reads `value.len() -> usize`, then converts to `u64` via `u64::try_from` (the bounded payload guarantees `value.len() <= 1_048_636`, so the conversion cannot fail in practice, but the `try_from` is mandatory per Holzman Rust).
- **Boundary 3 (gate).** `EncodedRecordLength::new(value.len() as u64)` is the only parser the gate introduces. It rejects `0` (caller bug) and values above `MAX_ENCODED_RECORD_BYTES` (encode_record already caps payloads at `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`, so this branch is unreachable in production but defended in depth).

## 4. Behavior of the Pure Gate Predicate

The pure decision the gate implements is:

```text
fn gate_decision(
    accumulated: AccumulatedFlushBytes,
    next: EncodedRecordLength,
    limit: JournalBatchByteBudget,
) -> GateDecision {
    if accumulated.0 > limit {
        // Defensive: should never happen if previous `add` succeeded.
        return GateDecision::Reject { attempted: accumulated.0, limit };
    }
    let attempted = match accumulated.0.checked_add(next.0) {
        Some(total) => total,
        None => return GateDecision::Reject { attempted: u64::MAX, limit },
    };
    if attempted > limit {
        return GateDecision::Reject { attempted, limit };
    }
    GateDecision::Accept { new_accumulated: AccumulatedFlushBytes(attempted) }
}

enum GateDecision {
    Accept { new_accumulated: AccumulatedFlushBytes },
    Reject { attempted: u64, limit: u64 },
}
```

- **Pure:** no I/O, no `Mutex`, no time, no Fjall.
- **Suitable for:** unit tests, proptest, Kani, and (with `pub(crate)` visibility on the newtypes) a future Verus spec at `writer_contract.rs`.
- **Returns the same error variant as `JournalWriteBatch::append_event`** for the same `(attempted, limit)` shape; this is the contract-parity claim.

## 5. Concrete StorageLimits::DEFAULT Binding (cross-crate invariant)

```text
const STORAGE_LIMITS_DEFAULT_BATCH_BYTES_BOUND: () = {
    assert!(
        StorageLimits::DEFAULT.max_journal_batch_bytes
            == crate::storage_constants::DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER
    );
    assert!(
        crate::storage_constants::DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER
            == crate::batch::types::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
                + crate::constants::RECORD_HEADER_BYTES
    );
    assert!(
        StorageLimits::DEFAULT.max_journal_event_payload_bytes
            == crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES
    );
};
```

- This `const _` block lives at the bottom of `types.rs` and fails to compile if either binding drifts.
- Mirrors the existing `IndexStatusState::_INDEX_STATUS_STATE_EXHAUSTIVE` exhaustiveness assertion pattern at `types.rs:324-334`.

## 6. Forbidden Type Patterns

- **No `Option<u64>` for the byte budget.** `StorageLimits::max_journal_batch_bytes` is `u64` directly; no "disable enforcement" sentinel. Disable is the caller's responsibility (pass `u64::MAX`).
- **No boolean flags** like `enforce_byte_budget: bool`. The contract is always enforced; the limit is the knob.
- **No `Box<dyn Trait>`** for the gate predicate; the gate is a pure `fn` over newtypes.
- **No stringly-typed error messages** beyond the existing `JournalError::InvalidConfig { field, reason }` shape.
- **No `Option<JournalWriterQueueProfileCounts>`** for the optional `pending_bytes` observability field (deferred per Open Question 2).
- **No `unsafe`** anywhere in the new types or gate (Holzman Rust).