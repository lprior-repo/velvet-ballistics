# ARCHITECTURAL DRIFT REPORT
## kani_idempotency_tracker.rs — DRIFT HAMMER STRIKES

**File:** `crates/vb_runtime/src/verification/kani/kani_idempotency_tracker.rs`
**Line Count:** 303 (EXCEEDS 300-LINE LIMIT BY 3 LINES)
**Classification:** CRITICAL — LINE COUNT VIOLATION + PRIMITIVE OBSESSION + VERIFICATION CODE SMELL

---

## VIOLATION 1: LINE COUNT BREACH (ZERO TOLERANCE)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 303 | 300 | **EXCEEDED** |

**Hammer:** File MUST be split into at least 2 modules. Suggested split:
- `kani_idempotency_tracker_core.rs` — FWH-001, FWH-005, FWH-006 (completion logic)
- `kani_idempotency_tracker_eviction.rs` — FWH-016, FWH-020 (eviction/monotonicity)
- `kani_idempotency_tracker_policy.rs` — Policy-aware tracking proofs

---

## VIOLATION 2: PRIMITIVE OBSESSION IN ARBITRARY GENERATION

### Finding: `any_bounded_ticket()` Duplicates Domain Logic

**Location:** Lines 20-44

**Problem:** The function generates raw primitives (`u64`, `u32`, `u128`, etc.) and manually constructs `ActionTicket` via field-by-field wrapping:

```rust
fn any_bounded_ticket() -> ActionTicket {
    let run_id = kani::any::<u64>();           // PRIMITIVE
    kani::assume(run_id > 0);
    let step = kani::any::<u32>();            // PRIMITIVE
    kani::assume(step < 64);
    let seq = kani::any::<u64>();             // PRIMITIVE
    let action_id = kani::any::<u32>();       // PRIMITIVE
    kani::assume(action_id < 256);
    let attempt = kani::any::<u16>();         // PRIMITIVE
    kani::assume(attempt > 0);
    let key = kani::any::<u128>();            // PRIMITIVE
    let capacity = kani::any::<u16>();        // PRIMITIVE
    kani::assume(capacity > 0);
    ActionTicket {
        run: RunId::new(run_id),              // DOMAIN WRAPPER
        step: StepIdx::new(step),              // DOMAIN WRAPPER
        seq: SeqNo::new(seq),                  // DOMAIN WRAPPER
        action: ActionId::new(action_id),     // DOMAIN WRAPPER
        attempt,
        idempotency_key: key,
        capacity,
    }
}
```

**Root Cause:** `ActionTicket` lacks a `kani::Arbitrary` implementation. Compare with `vb_core/src/ids/kani_id_arbitrary.rs` where ALL ID types properly implement `kani::Arbitrary`:

```rust
impl kani::Arbitrary for RunId {
    fn any() -> Self {
        Self::new(kani::any())
    }
}
```

**DDD Violation:** Scott Wlaschin Principle — "Make illegal states unrepresentable." The verification harness bypasses the type system by generating raw primitives, creating a parallel generation path that can drift out of sync with production invariants.

**Fix Required:** Implement `kani::Arbitrary` for `ActionTicket` in `vb_core/src/action.rs` (or a `kani_action.rs` module next to `kani_id_arbitrary.rs`), then replace `any_bounded_ticket()` with a simple `kani::any::<ActionTicket>()`.

---

## VIOLATION 3: PRIMITIVE OBSESSION IN CAPACITY HANDLING

### Finding: Raw `usize` Leaks Into Domain

**Location:** Lines 47-51, 64, 88, 127, 165, 222

```rust
fn any_bounded_capacity() -> usize {  // RETURNS RAW USIZE
    let cap = kani::any::<usize>();
    kani::assume(cap >= 1 && cap <= 16);
    cap
}
```

**Usage:**
```rust
let capacity = any_bounded_capacity();
let mut tracker = IdempotencyTracker::new(capacity);  // RAW USIZE
```

**DDD Violation:** `IdempotencyTracker::new()` takes a raw `usize` for capacity. This is a capacity primitive obsession violation — the domain should own a `TrackerCapacity` type with validation.

**Evidence:** Compare with `vb_core/src/ids/kani_id_arbitrary.rs` where domain types (`RunId`, `SeqNo`, etc.) wrap primitives immediately via `Self::new(kani::any())`. The same pattern should apply to tracker capacity.

**Fix Required:** Create a `TrackerCapacity(u16)` wrapper in vb_runtime idempotency module with bounded validation (1..=N), implement `kani::Arbitrary` for it, then change `IdempotencyTracker::new()` to accept `TrackerCapacity`.

---

## VIOLATION 4: VERIFICATION/PRODUCTION BOUNDARY BLUR

### Finding: Generator Function Duplicates Domain Validation

**Location:** Lines 22-44 vs `vb_core/src/ids/kani_id_arbitrary.rs`

The `any_bounded_ticket()` function duplicates what SHOULD be a type-level concern:

| Aspect | Domain Type | Generator Function |
|--------|-------------|-------------------|
| `run_id > 0` | `RunId::new()` accepts any `u64` | `kani::assume(run_id > 0)` |
| `step < 64` | `StepIdx` is `u16` but no upper bound | `kani::assume(step < 64)` |
| `action_id < 256` | `ActionId` is `u16` but no upper bound | `kani::assume(action_id < 256)` |
| `attempt > 0` | `ActionTicket::attempt` is `u16` | `kani::assume(attempt > 0)` |
| `capacity > 0` | `ActionTicket::capacity` is `u16` | `kani::assume(capacity > 0)` |

**Problem:** The validation assumptions in `any_bounded_ticket()` are scattered across the harness file. If `ActionTicket`'s invariants change, these assumptions may become inconsistent.

**Fix Required:** Lift ALL validation into type invariants:
- `RunId` should reject 0 (or document the invariant)
- `StepIdx` should document its 0-63 bound
- `ActionId` should document its 0-255 bound
- `ActionTicket` should implement `kani::Arbitrary` that uses internally-consistent bounds

---

## VIOLATION 5: UNWRAP IN PROOF HARNESS

### Finding: `.unwrap()` in Verification Code

**Location:** Line 169

```rust
tracker.mark_completed(&first_ticket).unwrap();  // UNWRAP!
```

**Problem:** Even in proof harnesses, `.unwrap()` is forbidden by the engineering rules. While this is in a Kani harness (not production), the rule is zero tolerance.

**Impact:** If `mark_completed` fails, the proof produces an incorrect result silently rather than propagating the error.

**Fix Required:** Use `kani::assume()` or `kani::assert()` to handle the Result:

```rust
kani::assume(tracker.mark_completed(&first_ticket).is_ok());
```

---

## VIOLATION 6: ID TYPE INCONSISTENCY

### Finding: `ActionId` Size Mismatch

**Location:** Lines 28-29 vs `vb_core/src/ids/mod.rs`

Harness says:
```rust
let action_id = kani::any::<u32>();
kani::assume(action_id < 256);
```

But `vb_core/src/ids/mod.rs` defines:
```rust
numeric_id!(ActionId, u16, get);  // ActionId is u16!
```

**This is a bug.** The harness generates a `u32` but `ActionTicket` expects `ActionId` which wraps `u16`. The cast `ActionId::new(action_id)` would truncate/compile incorrectly if `action_id > u16::MAX`.

**Fix Required:** Change to `kani::any::<u16>()` and keep `kani::assume(action_id < 256)` for domain-appropriate bound.

---

## SUMMARY TABLE

| # | Violation | Severity | Type |
|---|-----------|----------|------|
| 1 | 303 lines exceeds 300 limit | CRITICAL | Line Count |
| 2 | `any_bounded_ticket()` bypasses domain types | HIGH | Primitive Obsession |
| 3 | Raw `usize` capacity leaks | HIGH | Primitive Obsession |
| 4 | Generator duplicates domain validation | MEDIUM | Boundary Blur |
| 5 | `.unwrap()` in proof harness | HIGH | Engineering Rule |
| 6 | `ActionId` is `u16` not `u32` | CRITICAL | Type Bug |

---

## PRESCRIBED REMEDIATION

### Phase 1: Fix Type Bug (Immediate)

Change line 28 from `u32` to `u16`:
```rust
let action_id = kani::any::<u16>();
```

### Phase 2: Add `kani::Arbitrary` for `ActionTicket`

In `vb_core/src/action.rs` (or `kani_action.rs` module):
```rust
impl kani::Arbitrary for ActionTicket {
    fn any() -> Self {
        Self {
            run: kani::any(),
            step: kani::any(),
            seq: kani::any(),
            action: kani::any(),
            attempt: {
                let a = kani::any::<u16>();
                kani::assume(a > 0);
                a
            },
            idempotency_key: kani::any(),
            capacity: {
                let c = kani::any::<u16>();
                kani::assume(c > 0);
                c
            },
        }
    }
}
```

### Phase 3: Replace `any_bounded_ticket()` with `kani::any()`

All proofs should use `let ticket = kani::any::<ActionTicket>();` directly.

### Phase 4: Create `TrackerCapacity` Type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct TrackerCapacity(u16);

impl TrackerCapacity {
    pub fn new(cap: u16) -> Self {
        assert!(cap > 0 && cap <= 128);  // or use TryFrom
        Self(cap)
    }
}

impl kani::Arbitrary for TrackerCapacity {
    fn any() -> Self {
        let cap = kani::any::<u16>();
        kani::assume(cap >= 1 && cap <= 128);
        Self::new(cap)
    }
}
```

### Phase 5: Split File

Split at logical proof group boundaries to meet 300-line limit.

---

## SCOTT WLASCHIN DDD ASSESSMENT

| Principle | Status | Finding |
|-----------|--------|---------|
| Make illegal states unrepresentable | FAIL | `any_bounded_ticket()` generates primitives that bypass type invariants |
| Value objects over primitives | FAIL | Raw `usize` for capacity, raw `u32` for ActionId (should be u16) |
| Domain objects own their validation | FAIL | Validation scattered across harness, not in domain types |
| Ubiquitous language | PASS | Proof names match FWH spec |
| Bounded contexts respected | PASS | vb_runtime/verification/kani is a legitimate verification context |

---

**REPORT STATUS:** DRIFT CONFIRMED — HAMMER AUTHORIZED
**NEXT ACTION:** Agent must refactor before code can be merged
