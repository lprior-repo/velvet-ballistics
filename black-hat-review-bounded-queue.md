# BLACK-HAT REVIEW: LETHAL-5 — Bounded Action Completion Queue

**Reviewer:** black-hat-reviewer
**Date:** 2026-05-17
**Bead:** LETHAL-5
**Files Reviewed:**
- `crates/vb_runtime/src/action_queue.rs`
- `crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs`

---

## VERDICT: **REJECTED** — Mandated Rewrite

The implementation fails Phase 1 (Contract Parity) and Phase 4 (Panic Vector). It cannot proceed to aesthetics.

---

## PHASE 1: Contract & Bead Parity

### 1.1 Panicking Constructor — VIOLATION

**File:** `action_queue.rs:57`
```rust
assert!(capacity > 0, "capacity must be non-zero");
```

**File:** `action_queue.rs:70`
```rust
assert!(capacity > 0, "capacity must be non-zero");
```

**Finding:** The Master Contract Section 2 is unambiguous:
> "No `panic!`."

An `assert!` is a `panic!` under the hood. Constructors that panic on invalid input are forbidden. The `new()` and `with_backpressure()` functions must return `Result<Self>` or `Option<Self>` for invalid inputs, not panic.

**Required Fix:** Change constructors to return `Result<Self, ActionQueueError>` or handle zero capacity gracefully at the call site with a typed error. The panic variant is forbidden.

### 1.2 Panic-Testing — VIOLATION

**File:** `bounded_queue_tests.rs:65-68`
```rust
#[should_panic(expected = "capacity must be non-zero")]
fn bounded_action_queue_new_with_zero_capacity_panics() {
    let _queue = BoundedActionCompletionQueue::new(0);
}
```

**Finding:** This test explicitly validates panic behavior. The Master Contract forbids panic, and therefore forbids tests that validate panic behavior. This test must be removed and replaced with a test that validates the non-panicking error path once the constructor is fixed.

**Required Fix:** Replace with a test that verifies `new(0)` returns an appropriate error (once constructors are fixed to return `Result`).

### 1.3 Bead Parity — PARTIAL

The bead LETHAL-5 specifies 5 behaviors:
1. Queue rejects enqueue when full — **IMPLEMENTED** ✓
2. Queue accepts enqueue when below capacity — **IMPLEMENTED** ✓
3. Queue emits backpressure warning at 80% — **IMPLEMENTED** ✓
4. Queue tracks remaining capacity — **IMPLEMENTED** ✓
5. Queue drains to empty — **IMPLEMENTED** ✓

Contract parity with `test-plan-bounded-queue.md` is **partially met** — behaviors exist, but the panic-on-zero-input error path violates the "typed errors, no panic" rule from Section 2.

---

## PHASE 2: Farley Engineering Rigor

### 2.1 Function Length — PASS

| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| `enqueue` | 23 | 25 | ✓ PASS |
| `dequeue` | 7 | 25 | ✓ PASS |
| `new` | 8 | 25 | ✓ PASS |
| `with_backpressure` | 10 | 25 | ✓ PASS |

No function exceeds 25 lines. Farley constraint satisfied.

### 2.2 Parameter Count — PASS

All functions have ≤ 5 parameters. Farley constraint satisfied.

### 2.3 I/O Separation — ACCEPTABLE

The `BoundedActionCompletionQueue` is a pure in-memory data structure. The backpressure channel is a constructor-time dependency, not I/O hidden inside calculations. This is borderline acceptable for a queue abstraction. No rejectable I/O-inside-calculation violations found.

### 2.4 Test Design — ACCEPTABLE

Tests assert behavior (WHAT): return values, queue lengths, error variants, FIFO ordering. They do not assert implementation details (HOW). Test style is acceptable.

**Minor Note:** The `make_ticket` helper in both files duplicates the same logic. This is not a violation but indicates the integration tests and unit tests should share a common test fixture module.

---

## PHASE 3: Holzman Rust (The Big 6)

### 3.1 Make Illegal States Unrepresentable — ACCEPTABLE

`ActionQueueError::QueueFull { capacity }` is a proper sum type. The queue state (`is_empty`, `is_full`, `len`) is derived from the vector contents, not stored as potentially-inconsistent flags. No `Option`-based state machines present. Acceptable.

### 3.2 Parse, Don't Validate — PASS

`ActionTicket` is consumed by the queue; it is not parsed from an untrusted representation inside the queue. The queue trusts its caller. This is appropriate.

### 3.3 Types as Documentation — ACCEPTABLE

No boolean parameters. Acceptable.

### 3.4 Workflows as Explicit State Transitions — ACCEPTABLE

Enqueue → Full → Dequeue → Empty is a valid state transition diagram. The queue does not support arbitrary state mutation. Acceptable.

### 3.5 Newtypes — MINOR ISSUE

**File:** `action_queue.rs:37`
```rust
struct Inner {
    items: Vec<ActionTicket>,
}
```

`Inner` is a non-essential wrapper over `Vec<ActionTicket>`. It adds a layer of indirection without providing meaningful encapsulation beyond what `BoundedActionCompletionQueue` already provides. The `Inner` struct isYAGNI — it exists for no reason other than "we might need it later." This is a mild violation of Phase 5.

### 3.6 Library Choice — SUSPECTED VIOLATION

**File:** `action_queue.rs:32`
```rust
backpressure_tx: Option<std::sync::mpsc::Sender<BackpressureWarning>>,
```

The Master Contract Section 5 states:
> "`crossbeam-queue::ArrayQueue` is required for bounded MPMC queues... `rtrb` is required for SPSC trace/action rings"

The backpressure notification channel uses `std::sync::mpsc` which is not on the approved library list for queue/channel patterns. The approved libraries for queue-like behavior are `crossbeam-queue::ArrayQueue` (MPMC) and `rtrb` (SPSC).

However, the `mpsc` here is not the main queue — it's a notification channel. The `rtrb` crate is specified for "SPSC ring buffers and trace/action completion paths." A single `Sender`/`Receiver` pair for backpressure warnings might be considered a trace/action completion path, but this interpretation is not unambiguous.

**Finding:** Ambiguous. The notification channel could use `rtrb` as a single-element SPSC ring, or the code could be using an approved alternative. This requires clarification from the architect. Flag for discussion, not automatic rejection.

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

### 4.1 Option-Based State Machines — PASS

No `Option`-based state machines found. Pass.

### 4.2 CUPID Properties — ACCEPTABLE

The queue is:
- **Composable:** Yes — can be embedded in larger runtime structures
- **Unix-philosophy:** Yes — single responsibility, does one thing
- **Predictable:** Yes — FIFO, bounded, deterministic
- **Idiomatic:** Borderline — `Vec::remove(0)` is idiomatic but inefficient
- **Domain-based:** Yes — `ActionTicket`, `BackpressureWarning` are domain types

Acceptable.

### 4.3 The Panic Vector — **CRITICAL FAILURES**

**File:** `action_queue.rs:57,70`
```rust
assert!(capacity > 0, "capacity must be non-zero");
```
**Violation:** `panic!` via `assert!`. Explicitly forbidden in Master Contract Section 2.

**File:** `action_queue.rs:90,120`
```rust
let mut inner = self.inner.lock().unwrap();
```
**Violation:** `.unwrap()` on `Mutex` lock result. The Master Contract Section 2 says:
> "No `.unwrap()`."

While `std::sync::Mutex::lock()` only panics on poison (a systemic failure), the rule is absolute. The code must use `match` or `expect()` with a descriptive message, or restructure to avoid the unwrap. Given that poison-on-unwind is essentially unrecoverable anyway, this is low severity in practice but still a rule violation.

**Finding:** 3 `.unwrap()` calls on lock results (lines 90, 120, 131). All violate Phase 4.

**Required Fix:** Replace with:
```rust
let inner = match self.inner.lock() {
    Ok(g) => g,
    Err(poisoned) => poisoned.into_inner(),
};
```
Or use a `MutexGuard` wrapper that handles poison gracefully.

### 4.4 Unnecessary `let mut` — MINOR

**File:** `action_queue.rs:90`
```rust
let mut inner = self.inner.lock().unwrap();
```

`inner` is mutated via `inner.items.push()` and `inner.items.remove()`. However, the `MutexGuard` itself doesn't need mutability — only the inner `Vec` does. This is a style issue, not a correctness issue. The `mut` on the binding is unnecessary but not harmful.

---

## PHASE 5: The Bitter Truth (Velocity & Legibility)

### 5.1 Punish Cleverness — FAIL

**File:** `action_queue.rs:124`
```rust
Some(inner.items.remove(0))
```

`Vec::remove(0)` is O(n) — it shifts every remaining element. For a queue that is meant to be used in production hot paths with potentially thousands of action completions, this is a performance trap.

**Required Fix:** Use `VecDeque` instead of `Vec` for the inner storage. `VecDeque::pop_front()` is O(1), not O(n). This is not a stylistic preference — it is a correctness issue for a queue implementation.

**Severity:** HIGH. The entire point of a bounded action completion queue in a hot runtime path is O(1) enqueue and dequeue. O(n) dequeue makes this unsuitable for production use at scale.

### 5.2 YAGNI — MINOR VIOLATION

**File:** `action_queue.rs:35-38`
```rust
#[derive(Debug)]
struct Inner {
    items: Vec<ActionTicket>,
}
```

`Inner` adds a layer of indirection that provides zero value. `BoundedActionCompletionQueue` already wraps the mutex and capacity. The `Inner` struct isYAGNI — it was added "just in case" but provides no extra abstraction benefit.

**Required Fix:** Remove `Inner` and store `Vec<ActionTicket>` directly in `BoundedActionCompletionQueue`'s mutex.

### 5.3 Sniff Test

Does this code look like it was written by a junior developer trying to prove how smart they are? No. It is straightforward and readable. The comments are helpful. The function names are clear. The test names follow the BDD convention. No clever tricks.

**Verdict:** The code passes the sniff test for legibility. The failures are in correctness (Vec vs VecDeque) and rule compliance (assert!, unwrap), not in readability.

---

## SUMMARY OF VIOLATIONS

| Phase | Severity | Finding | Location |
|-------|----------|---------|----------|
| 1 | **CRITICAL** | Constructor panics on invalid input (`assert!`) | `action_queue.rs:57,70` |
| 1 | **CRITICAL** | Test validates panic behavior (`#[should_panic]`) | `bounded_queue_tests.rs:65-68` |
| 4 | **CRITICAL** | O(n) dequeue via `Vec::remove(0)` — wrong data structure | `action_queue.rs:124` |
| 4 | **HIGH** | `.unwrap()` on `Mutex::lock()` (x3) | `action_queue.rs:90,120,131` |
| 5 | **HIGH** | `Inner` struct isYAGNI | `action_queue.rs:35-38` |
| 3 | **MEDIUM** | `std::sync::mpsc` not on approved library list for queue patterns | `action_queue.rs:32` |

---

## MANDATED FIXES (in order of priority)

1. **Replace `Vec` with `VecDeque`** in `Inner`. `VecDeque::pop_front()` is O(1); `Vec::remove(0)` is O(n). A queue that dequeues in O(n) is not a proper queue.

2. **Change constructors to return `Result<Self, ActionQueueError>`** instead of panicking on `capacity == 0`. Add an `InvalidCapacity` error variant.

3. **Remove the `#[should_panic]` test** and replace with an error-path test once constructors return `Result`.

4. **Replace all `lock().unwrap()` with poison-safe lock handling.** Use `match` or a helper that collapses poisoned mutexes gracefully.

5. **Remove `struct Inner`** — it adds indirection with no value.

6. **Justify or replace `std::sync::mpsc`** backpressure channel with an approved alternative (`rtrb` single-element ring or `crossbeam-queue::ArrayQueue` as a 1-element MPMC).

---

## WHAT WAS DONE WELL

- `ActionQueueError::QueueFull { capacity }` is a clean, domain-appropriate error type
- BDD test naming convention is consistent and readable
- 45+ tests provide excellent coverage of edge cases
- FIFO ordering is correctly specified and tested
- Backpressure threshold calculation `(capacity * 8) / 10` is correct integer math
- `remaining_capacity()` correctly uses `saturating_sub`
- `#[must_use]` annotations on query methods are proper hygiene
- Module documentation is clear about what LETHAL-5 this implements

---

## CONCLUSION

The implementation has strong test coverage and correct algorithmic intent, but it violates critical rules:
- **Panic in constructors** (forbidden by Section 2)
- **Wrong data structure for a queue** (O(n) dequeue — performance trap)
- **`.unwrap()` on lock results** (forbidden by Section 2)

**REJECTED.** Rewrite the constructor error handling, switch to `VecDeque`, remove the panic tests, and fix the lock unwraps. Then resubmit for black-hat review.
