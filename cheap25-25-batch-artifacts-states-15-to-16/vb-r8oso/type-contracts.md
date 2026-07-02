# Type Contracts — vb-r8oso

**bead_id:** vb-r8oso
**owner_stage:** rust-contract
**upstream_artifacts:** `domain-model.md`

This artifact fixes the exact types, signatures, semantic invariants,
constructor rules, and parse-at-boundary rules that bind the fix to Rust.
Type signatures here are normative; downstream implementations must compile,
and downstream tests must assert on these signatures without rewriting them.

---

## 1. Reused Types (No Definition Change)

| Type | Module | Notes |
|---|---|---|
| `RunId` | `vb_core` | `u64` newtype. `RunId::ZERO` is invalid; writes to invalid identifiers emit `JournalError::InvalidRunId` (existing path), never `SequenceMismatch`. |
| `EventSeq` | `crates::vb_storage::types` | `u64` newtype. Constructed via `EventSeq::new(n: u64)`. `EventSeq::ZERO` is the first valid seq. `EventSeq::MAX` is the codec-reserved sentinel. `succ()` returns `Result<EventSeq, JournalError>` mapped by `codec::next_seq`. |
| `JournalEvent` | `crates::vb_storage::events` | Sum type; `event.seq()` returns `EventSeq`. |
| `JournalError` | `crates::vb_storage::error` | Existing enum; **adds one variant** — see §3. |
| `FjallJournal` | `crates::vb_storage::journal` | Existing struct; **adds one method** — see §2. |
| `JournalWriteBatch<'j>` | `crates::vb_storage::batch` | Existing batch type; existing `append_event` adds the guard — see §2.4. |

## 2. New and Updated Method Signatures

### 2.1 `FjallJournal::next_sequence_at_write` (NEW)

```rust
impl FjallJournal {
    /// Returns the sequence value that the next successful append for `run`
    /// must carry.
    ///
    /// Semantics:
    /// - `EventSeq::ZERO` when no event has been durably written for `run`.
    /// - `last_durable_event_seq(run).succ()` otherwise.
    /// - `Err(JournalError::SequenceOverflow)` if the succ overflows `u64::MAX`.
    ///
    /// Implementation contract:
    /// - Key-only Fjall `prefix().next_back()` traversal; no event-value decode.
    /// - Caller-observable atomic with the next append that follows.
    /// - Lock-free: uses the durable LSM snapshot visible to `events.contains_key`.
    /// - NEVER returns `Ok(EventSeq::MAX)` and never panics.
    pub fn next_sequence_at_write(
        &self,
        run: RunId,
    ) -> Result<EventSeq, JournalError>;
}
```

#### 2.1.1 Pre-conditions (requires)

- `run != RunId::ZERO`. The implementation MAY emit `JournalError::InvalidRunId` for `RunId::ZERO` rather than returning a seq value; preferred behaviour is to return `Err(InvalidRunId)` so callers cannot confuse an invalid identifier with a fresh-run answer.

#### 2.1.2 Post-conditions (ensures)

- On `Ok(seq)`: `seq.get()` is a valid `EventSeq` value (`0 <= seq <= EventSeq::MAX - 1`).
- On `Ok(seq)`: `seq >= EventSeq::ZERO`.
- On `Ok(seq)` for a run with zero stored events: `seq == EventSeq::ZERO`.
- On `Ok(seq)` for a run with at least one stored event: `seq == last_durable_event_seq(run).succ()`.
- On `Err(SequenceOverflow)`: the stored tail is `EventSeq::MAX`, and `succ()` saturated.
- The lookup did not decode any event value (`BLAKE3 + postcard` cost avoided).

#### 2.1.3 Helper pair

A private helper is introduced in the same `impl`:

```rust
impl FjallJournal {
    /// Returns the largest seq currently present in the `events` keyspace
    /// for `run`, or `None` if no events are stored. Key-only lookup.
    /// Returns `Err(JournalError::MalformedKeyspaceRow)` on a malformed row.
    fn last_durable_event_seq(
        &self,
        run: RunId,
    ) -> Result<Option<EventSeq>, JournalError>;
}
```

`next_sequence_at_write` is implemented in terms of
`last_durable_event_seq(run)?`:
- `None` → `Ok(EventSeq::ZERO)`.
- `Some(seq)` → `codec::next_seq(seq)` which maps `u64::MAX` to `SequenceOverflow`.

### 2.2 `JournalError::SequenceMismatch` (NEW VARIANT)

```rust
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ... existing variants ...

    /// Write-time typed error: the supplied event.seq() did not match
    /// `FjallJournal::next_sequence_at_write(run)`. The event was NOT
    /// durably committed.
    #[error(
        "journal append sequence mismatch for run {run:?}: \
         expected {expected:?}, actual {actual:?}"
    )]
    SequenceMismatch {
        run: RunId,
        expected: EventSeq,
        actual: EventSeq,
    },
}
```

#### 2.2.1 Field invariants

- `expected == next_sequence_at_write(run)` at the moment the append was rejected.
- `actual == event.seq()` of the offending call.
- `expected != actual` always (the constructor pre-condition enforces this; if the implementation somehow constructs an `Ok`-shaped variant, downstream tests assert non-equality before the variant is exposed).
- `run` matches `event.run_id()` of the offending call.

#### 2.2.2 Diagnostic code (mandatory)

- `pub const SEQUENCE_MISMATCH_AT_WRITE_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);` in `crates/vb_storage/src/error/codes.rs`.
- Symbolic code: `"JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"`.
- `diagnostic_code()` match arm: `Self::SequenceMismatch { .. } => Self::SEQUENCE_MISMATCH_AT_WRITE_CODE`.
- `symbolic_code()` match arm: `Self::SequenceMismatch { .. } => "JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"`.
- Symbolic-code registration in `SymbolicCode::CODE_REGISTRY` is preferred but fallback to `INTERNAL_INVARIANT` is acceptable for v1.

### 2.3 `append_journaled`, `append_strict`, `append_strict_batch`, `append_unfsynced` (TIGHTEN, NO SIGNATURE CHANGE)

Existing signatures stay:

```rust
pub fn append_journaled(&self, event: &JournalEvent) -> Result<(), JournalError>;
pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError>;
pub fn append_strict_batch(&self, events: &[JournalEvent]) -> Result<(), JournalError>;
pub(crate) fn append_unfsynced(&self, event: &JournalEvent) -> Result<(), JournalError>;
```

The doc-comments for each MUST grow to state the new post-condition: on
`Ok(())`, `event.seq() == next_sequence_at_write(event.run_id()) pre-call`.
No silent rewrites; the implementation must reject with `SequenceMismatch`
when the in-memory `event.seq()` disagrees.

### 2.4 `JournalWriteBatch::append_event` (TIGHTEN, NO SIGNATURE CHANGE)

Existing signature stays:

```rust
pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>;
```

The C6 guard precedence in the doc-comment is updated:

```
// Guard precedence (C6)
// 1. Key construction
// 2. Semantic event validation
// 3. next_sequence_at_write guard -> aborts batch with SequenceMismatch
// 4. Same-batch duplicate check (HashSet guard)
// 5. Durable duplicate check -> aborts batch
// 6. Count capacity check
// 7. Per-record encoding / payload size
// 8. Accumulated byte admission
// 9. Insert into inner OwnedWriteBatch
```

When the new guard fires, the batch is marked `aborted = true` and `staged_event_keys` is unchanged. The same outcome fires for `append_strict_batch` via the same `append_event` path.

### 2.5 Public-API Wrapper (NEW)

```rust
// crates/vb_storage/src/public_api.rs
pub fn next_sequence_at_write(
    journal: &FjallJournal,
    run: RunId,
) -> Result<EventSeq, JournalError> {
    journal.next_sequence_at_write(run)
}
```

Mirrors the existing wrapper pattern (`append_journal_event`, `read_run_events`).

## 3. New Diagnostic and Symbolic Code Constants

```rust
// crates/vb_storage/src/error/codes.rs
impl JournalError {
    /// Diagnostic code for write-time sequence mismatch.
    /// Distinct from `SEQUENCE_GAP_CODE` (replay-time) and from
    /// `REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE`/`REPLAY_KEY_MISMATCH_CODE`
    /// (which are also read-time). Reserved block: 0x404x.
    pub const SEQUENCE_MISMATCH_AT_WRITE_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);
}
```

Symbolic string: `"JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"`.

The pre-existing diagnostic codes `SEQUENCE_GAP_CODE = 0x4009`,
`SEQUENCE_OVERFLOW_CODE = 0x400A`, `REPLAY_KEY_MISMATCH_CODE = 0x4040`,
`REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE = 0x4041` are unchanged.

## 4. Smart-Constructor Rules

The error variant itself is constructed only at the moment of write-time rejection:

```rust
// In each append path, immediately before returning Err:
let expected = journal.next_sequence_at_write(event.run_id())?;
let actual = event.seq();
// (expected != actual already verified by the guard above)
return Err(JournalError::SequenceMismatch {
    run: event.run_id(),
    expected,
    actual,
});
```

No manual constructor exists outside the write paths. Tests that fabricate
this variant are forbidden to set `expected == actual`.

## 5. Boundary Parsers (No Change)

No new external input parses a `seq`. `JournalEvent.seq()` is already a typed
read of an internally-encoded `u64`. The new method `next_sequence_at_write`
takes `RunId`, which is already validated by its own constructor.

## 6. Functional-Core / Imperative-Shell Split

### 6.1 Pure-Core Helpers (no I/O)

- `codec::next_seq(seq: EventSeq) -> Result<EventSeq, JournalError>` — already pure; reused. Maps `EventSeq::MAX` to `SequenceOverflow`.

### 6.2 Storage-Boundary Helper

- `last_durable_event_seq(&self, run) -> Result<Option<EventSeq>, JournalError>` — single LSM-tree `prefix().next_back()` lookup; callable on a `ReadOnlyJournal` if necessary, but in this bead only the writer-context path uses it.

### 6.3 Public Boundary Method

- `next_sequence_at_write(&self, run) -> Result<EventSeq, JournalError>` — thin composition of `last_durable_event_seq` + `codec::next_seq`.

### 6.4 Imperative Shell

- All five append paths are imperative shell; the new guard is a single helper call inserted at C6 step 3. No new I/O surface; no extra locks.

## 7. Forbidden Internals

- No `unsafe` is introduced. (Already `#![forbid(unsafe_code)]` at crate level.)
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg` in the new code.
- No unchecked indexing, slicing, casts, or arithmetic. `succ()` uses `checked_add`.
- No fallback to `EventSeq::ZERO` when an overflow is observed; `SequenceOverflow` propagates.

## 8. Compilation/Type-Level Invariants Tests Must Verify

- `error_code_tests::sequence_mismatch_at_write_code` — `JournalError::SequenceMismatch { run: _, expected: _, actual: _ }.diagnostic_code() == JournalError::SEQUENCE_MISMATCH_AT_WRITE_CODE (0x4042)`.
- `error_code_tests::sequence_mismatch_at_write_symbolic` — `.symbolic_code()` resolves to `"JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"` or `INTERNAL_INVARIANT` fallback (whichever the implementation registers).
- `error_tests::sequence_mismatch_display` — the `Display` impl mentions `expected` and `actual`.
- `error_tests::sequence_mismatch_constructor_fields` — the variant carries `{run, expected, actual}` with `expected != actual` invariant.

## 9. Subtype / Trait Coverage

The new method does NOT introduce new trait impls. `JournalError` continues to be `Debug + thiserror::Error`; the new variant must satisfy both automatically because `EventSeq` and `RunId` are both `Debug`.

`HasSymbolicCode` continues to apply; the symbolic arm for `SequenceMismatch` is mandatory and must register `0x4042` if not already present, or fall back to `INTERNAL_INVARIANT` per the existing pre-SC-009 fallback convention for the `0x40xx` block.

## 10. Migration / Versioning

- The change is **additive** for both the type and the API.
- Existing downstream `match`es over `JournalError` need a new arm but do not break (Rust's exhaustiveness checker will surface the requirement).
- Existing tests at `crates/vb_storage/src/tests.rs:1737` (`append_strict_rejects_out_of_order_sequence`) and `:4612` (`adversarial_read_events_with_sequence_gap_returns_exact_gap`) are updated (NOT deleted) to assert the new typed error at the append site rather than the read site. See `boundary-map.md` §5 and `contract.md` §6 for the exact rewrite pattern.
- No schema migration is required (no on-disk format change).
- The new `kani-sequence-at-write` Cargo feature is additive and gate-isolated per AGENTS.md kani-harness-isolation rule.

## 11. Type-Contract Acceptance

A reviewer downstream should be able to read this file plus `error/mod.rs` plus
the new `next_sequence_at_write` signature, and predict:

1. Every successful append's tail matches `next_sequence_at_write` at write time.
2. Every disagreement emits the new typed error with three exact fields.
3. `SequenceGap` (read) and `SequenceMismatch` (write) coexist with disjoint diagnostic codes.
4. No silent rewrite happens.
5. Overflow is reported, not panicked.

If a reviewer cannot reconcile the production code with these five statements,
the contract has been violated.
