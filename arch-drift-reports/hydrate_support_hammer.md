# ARCHITECTURAL DRIFT FINDINGS
## File: `crates/vb_storage/src/recovery/hydrate_support.rs`
## Line Count: 599 (VIOLATION: exceeds 300-line limit by 299 lines)

---

## EXECUTIVE SUMMARY

**VERDICT: ATOMIC REJECTION**

This file is a **GOD MODULE** that violates every foundational principle of the <300-line rule and Scott Wlaschin DDD. It mixes serialization concerns, domain logic, state machine transitions, and primitive obsession into a 599-line monolith. It MUST be decomposed before any further work proceeds.

---

## VIOLATION MATRIX

| Category | Severity | Count | Examples |
|----------|----------|-------|----------|
| **Line Count** | CRITICAL | 1 | 599 lines (limit: 300) |
| **Primitive Obsession** | CRITICAL | 12 | `u16`, `u32`, `u64`, `[u8; 32]`, raw `Result` |
| **God Module** | CRITICAL | 6 | 6 distinct responsibilities co-located |
| **Feature Envy** | HIGH | 4 | Event matching reaches into vb_core internals |
| **Duplicate Code** | HIGH | 3 | `verify_action_ticket_event` duplicated logic |
| **Invalid Type Safety** | HIGH | 2 | Raw `String` error details instead of domain types |

---

## PRIMITIVE OBSESSION VIOLATIONS (12 instances)

### 1. `u16` for step_count / slot_count (Lines 238-252)
```rust
let step_count = max_step.map(|s| {
    s.get().checked_add(1).ok_or(...)
}).unwrap_or(Ok(0))?;
```
**ISSUE:** Raw `u16` is used where a `StepCount` or `SlotCount` newtype should exist.
**DOMAIN CONCEPT MISSING:** "A bounded non-zero count of steps/slots in a recovery run."

### 2. `u32` for encoded_len (Lines 91-94, 343, 559)
```rust
let actual_len = u32::try_from(value.len()).map_err(|_| ...)?;
```
**ISSUE:** `u32` is used for byte lengths instead of `ByteLength` or `EncodedLength` newtype.
**DOMAIN CONCEPT MISSING:** "A validated byte length from wire encoding."

### 3. `u64` for executed counter (Lines 270, 280, 289, 304, etc.)
```rust
let mut executed = 0u64;
executed = executed.saturating_add(1);
```
**ISSUE:** Raw `u64` counter with no domain semantics.
**DOMAIN CONCEPT MISSING:** "EventApplicationCount" or "AppliedEventCount."

### 4. `[u8; 32]` for digest (Lines 85, 102-109, 354)
```rust
expected: [u8; 32],
found = *blake3::hash(value).as_bytes();
```
**ISSUE:** Raw 32-byte array for digest instead of `Blake3Digest` or `ValueDigest` newtype.
**DOMAIN CONCEPT MISSING:** "A cryptographically bound action value digest."

### 5. Raw `Result` return types without domain wrapping
```rust
pub(crate) fn verified_action_envelope_digest(...) -> RecoveryResult<[u8; 32]>
```
**ISSUE:** Return type is primitive array, not a domain-typed digest wrapper.

### 6. Raw `Option` for dimension tracking (Lines 196-198)
```rust
let mut max_step: Option<vb_core::StepIdx> = None;
let mut min_step: Option<vb_core::StepIdx> = None;
let mut max_slot: Option<vb_core::SlotIdx> = None;
```
**ISSUE:** Raw `Option` instead of a `DimensionBounds` or `FrameDimensions` aggregate.

---

## GOD MODULE DECOMPOSITION (6 responsibilities)

This file contains **6 distinct responsibilities** that must be extracted into separate modules:

### Responsibility 1: Slot Taint Observation/Resolution
**Lines:** 13-55
**Functions:** `SlotTaintReadObservation`, `SlotTaintResolution`, `resolve_slot_taint_read`, `observe_slot_taint_read`
**ISSUE:** Copy-only taint decision logic embedded in hydration module.
**EXTRACT TO:** `slot_taint.rs` or `recovery/slot_taint.rs`

### Responsibility 2: Action Ticket Verification
**Lines:** 57-111
**Functions:** `verify_action_ticket_event`, `verified_action_envelope_digest`, `decode_action_envelope_slot`
**ISSUE:** Action envelope validation mixed with slot decoding.
**EXTRACT TO:** `action_ticket.rs` or `recovery/action_ticket.rs`

### Responsibility 3: Snapshot Slot Decoding
**Lines:** 139-185
**Function:** `decode_snapshot_slots`
**ISSUE:** Deserialization logic for snapshot data.
**EXTRACT TO:** `snapshot_decoding.rs` or `recovery/snapshot_slots.rs`

### Responsibility 4: Dimension Derivation
**Lines:** 187-257
**Function:** `derive_dimensions_from_snapshot_and_tail`
**ISSUE:** Computing frame dimensions from events is a separate concern.
**EXTRACT TO:** `dimension_derivation.rs` or `recovery/dimensions.rs`

### Responsibility 5: Tail Event Application
**Lines:** 259-472
**Function:** `apply_tail_events`
**ISSUE:** STATE MACHINE TRANSITIONS embedded in hydration. This is 213 lines of event matching that should be in a state machine module.
**EXTRACT TO:** `tail_event_application.rs` or `recovery/event_application.rs`

### Responsibility 6: Parallel In-Flight Computation
**Lines:** 474-599
**Function:** `compute_parallel_in_flight`
**ISSUE:** Duplicates much of `apply_tail_events` logic. 126 lines of near-duplicate event matching.
**EXTRACT TO:** `parallel_tracking.rs` or `recovery/parallel_in_flight.rs`

---

## DUPLICATE CODE VIOLATIONS

### Duplicate 1: `verify_action_ticket_event` calls (Lines 57-77, 313, 513)
```rust
pub(crate) fn verify_action_ticket_event(run: RunId, ticket: ActionTicket) -> RecoveryResult<()> {
    if ticket.run != run { ... }
    if ticket.attempt == 0 || ticket.capacity == 0 || ticket.attempt > ticket.capacity { ... }
    if !vb_core::action::action_ticket_has_valid_key(ticket) { ... }
}
```
**CALLS:** Line 87 (`verified_action_envelope_digest`), Line 313 (`apply_tail_events`), Line 513 (`compute_parallel_in_flight`)
**ISSUE:** Should be a method on `ActionTicket` or a shared `ActionTicketValidator` module.

### Duplicate 2: Event matching for action scheduling (Lines 291-305 vs 489-505)
Both `apply_tail_events` and `compute_parallel_in_flight` have identical logic:
```rust
JournalEvent::ActionScheduled { action, step, .. } => {
    if tracker.is_resolved(*action, *step) { return Err(...); }
    frame.add_parallel_in_flight(1)...;
}
```
**ISSUE:** 15+ lines of exact duplicate event handling.

### Duplicate 3: `sub_tail_parallel_in_flight` (Lines 124-137)
**CALLED FROM:** Lines 334, 379, 390
**ISSUE:** A helper that's called in multiple places but is a single primitive operation that suggests `RunFrame` itself should handle this.

---

## SCOTT WLASCHIN DDD VIOLATIONS

### 1. MIXED DOMAINS (Feature Envy)
The event matching in `apply_tail_events` reaches into `vb_core::RunFrame` for:
- `mark_running`, `mark_succeeded`, `mark_waiting`, `mark_asking` (state transitions)
- `add_parallel_in_flight`, `sub_parallel_in_flight` (counter operations)
- `write_slot_with_taint` (slot mutations)
- `step_state` (state queries)

**ISSUE:** Hydration is doing state machine work that belongs in `RunFrame` or a `RunFrameHydrator` aggregate.

### 2. INVALID TYPE SAFETY (Primitive Strings in Errors)
Lines 61, 67, 73, 93, 98, 108, etc.:
```rust
detail: String::from("action ticket run mismatch"),
detail: String::from("action completion value length exceeds u32"),
```
**ISSUE:** Raw `String` details instead of domain-typed error details like `ActionTicketError::RunMismatch`, `EncodingError::LengthOverflow`.

### 3. UNWRAPPED PRIMITIVES (No Value Objects)
- `u16` for counts → should be `StepCount`, `SlotCount`
- `u32` for lengths → should be `EncodedByteLength`
- `u64` for counters → should be `EventCount`
- `[u8; 32]` for digests → should be `Blake3Digest` or `ValueDigest`
- `bool` returns → should be `ReplayEffect::Duplicate/Apply/Skip`

### 4. SIDE-EFFECT PROCEDURAL CODE (Not Functional Core)
`apply_tail_events` and `compute_parallel_in_flight` are imperative event loops that mutate `frame` directly. They should be:
- A fold over events returning a new state
- Or at minimum, a dedicated `HydrationContext` that encapsulates the mutation logic

---

## MANDATORY REFACTORING PLAN

### Phase 1: Extract Value Objects (Safety-critical, no logic change)
1. Create `vb_storage/src/recovery/values.rs`:
   - `StepCount(u16)` - non-zero validated
   - `SlotCount(u16)` - non-zero validated
   - `ByteLength(u32)` - from encoding
   - `EventCount(u64)` - applied event counter
   - `Blake3Digest([u8; 32])` - wrapped digest
   - `DimensionBounds { max_step, min_step, max_slot }` - aggregated dimensions

### Phase 2: Extract Action Ticket Module
2. Create `vb_storage/src/recovery/action_ticket_support.rs`:
   - `validate_action_ticket(run, ticket) -> RecoveryResult<()>`
   - `decode_and_verify_envelope(ticket, output, value, encoded_len, expected) -> RecoveryResult<SlotValue>`
   - Move `verify_action_ticket_event`, `verified_action_envelope_digest`, `decode_action_envelope_slot`

### Phase 3: Extract Slot Taint Module
3. Create `vb_storage/src/recovery/slot_taint_support.rs`:
   - Move `SlotTaintReadObservation`, `SlotTaintResolution`, `resolve_slot_taint_read`, `observe_slot_taint_read`

### Phase 4: Extract Snapshot Decoding
4. Create `vb_storage/src/recovery/snapshot_decoding.rs`:
   - Move `decode_snapshot_slots`

### Phase 5: Extract Dimension Derivation
5. Create `vb_storage/src/recovery/dimension_derivation.rs`:
   - Move `derive_dimensions_from_snapshot_and_tail`

### Phase 6: Extract Event Application (Largest single chunk)
6. Create `vb_storage/src/recovery/event_application.rs`:
   - Move `apply_tail_events` (213 lines)
   - Move `sub_tail_parallel_in_flight`

### Phase 7: Extract Parallel Tracking
7. Create `vb_storage/src/recovery/parallel_tracking.rs`:
   - Move `compute_parallel_in_flight` (126 lines)

### Phase 8: Create Re-export Module
8. Update `vb_storage/src/recovery/mod.rs` to re-export all extracted modules under a unified API.

---

## ESTIMATED POST-REFACTOR LINE COUNTS

| Module | Estimated Lines |
|--------|----------------|
| `hydrate_support.rs` (stub re-export) | ~20 |
| `values.rs` (new value objects) | ~80 |
| `action_ticket_support.rs` | ~90 |
| `slot_taint_support.rs` | ~45 |
| `snapshot_decoding.rs` | ~50 |
| `dimension_derivation.rs` | ~75 |
| `event_application.rs` | ~220 |
| `parallel_tracking.rs` | ~130 |

**Result:** Largest module is 220 lines (still over limit, needs Phase 6 sub-split)

---

## REJECTION CRITERIA

This file is **NOT APPROVED** for:
- [ ] Any PR merge
- [ ] Any feature development
- [ ] Any test addition
- [ ] Any verification work

**UNTIL:**
1. File reduced to ≤300 lines
2. All 6+ responsibilities extracted into separate modules
3. Primitive types replaced with domain value objects
4. Duplicate event matching logic consolidated into shared helper
5. Error details replaced with typed domain errors
6. All extracted modules have isolated unit tests

---

## EVIDENCE COMMANDS

```bash
# Verify line count
wc -l crates/vb_storage/src/recovery/hydrate_support.rs
# Expected: <= 300

# Count responsibility clusters (git grep patterns)
rg -c "fn apply_tail_events|fn compute_parallel_in_flight|fn decode_snapshot_slots|fn derive_dimensions" crates/vb_storage/src/recovery/hydrate_support.rs

# Verify no primitive returns in RecoveryResult
rg "RecoveryResult<\[u8; 32\]>" crates/vb_storage/src/recovery/
```

---

**FINDING ID:** ARCH-DRIFT-001
**FILE:** `crates/vb_storage/src/recovery/hydrate_support.rs`
**STATUS:** REJECTED
**REVIEWED BY:** architectural-drift agent
**DATE:** 2026-05-29
