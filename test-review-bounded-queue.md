# Test Plan Review: LETHAL-5 — Bounded Action Completion Queue

## VERDICT: REJECTED

---

## Axis 1 — Contract Parity

**Result**: PASS (with minor gaps)

The test plan implies the following pub fn API for `BoundedActionCompletionQueue`:

| Pub Fn | Scenario Coverage |
|--------|-----------------|
| `new(capacity: usize) -> Self` | Implied, not explicit |
| `enqueue(item) -> Result<(), ActionQueueError>` | 5 scenarios |
| `dequeue() -> Option<ActionCompletion>` | 3 scenarios |
| `len() -> usize` | 2 scenarios |
| `is_empty() -> bool` | 1 scenario |
| `is_full() -> bool` | 1 scenario (combinatorial matrix) |
| `remaining_capacity() -> usize` | 4 scenarios |
| `capacity() -> usize` | 0 explicit scenarios |
| `ActionQueueError::QueueFull { capacity }` | Exact variant asserted in 2 scenarios |

**Minor Gap**: `capacity() -> usize` has zero explicit scenarios. It is tested implicitly via `remaining_capacity` scenarios but no scenario directly asserts `queue.capacity() == N`.

**Minor Gap**: No scenario for `new(capacity = 0)`. The proptest anti-invariant mentions "must fail or panic" but no explicit BDD scenario names this constructor boundary.

---

## Axis 2 — Assertion Sharpness

**Result**: PASS

All scenarios assert exact values:

- `Err(ActionQueueError::QueueFull { capacity: 3 })` — exact variant ✓
- `Err(ActionQueueError::QueueFull { capacity: 1 })` — exact variant ✓
- `Ok(())` — explicit unit value, not bare `is_ok()` ✓
- `Queue length is 3` — exact integer ✓
- `remaining_capacity()` returns `5`, `6`, `16`, `0` — exact values ✓
- `is_empty()` returns `true` — exact boolean ✓
- `dequeue` returns A, B, C in order — exact values ✓
- `dequeue` returns `None` — exact variant ✓
- Backpressure warning scenarios assert "warning is emitted" — but `backpressure_warning_contains_depth_and_capacity` asserts exact `depth: 4, capacity: 5` payload ✓

**No `is_ok()` or `is_err()` as sole assertions found.**

---

## Axis 3 — Trophy Allocation

**Result**: FAIL — **LETHAL**

The plan allocates:
- **8 unit tests**
- **4 integration tests**
- **1 e2e test**

The `BoundedActionCompletionQueue` exposes **8 pub fns** (constructor + 7 methods).

**LETHAL Rule**: "Planned unit test count < 5× public function count → **LETHAL**"

Required minimum: 8 × 5 = **40 unit tests**
Allocated: **8 unit tests**

Ratio: **1:1** — far below the required 5:1 threshold.

Even if we count the 19 named BDD scenarios as the unit test count, the plan explicitly labels "8 unit / 4 integration / 1 e2e" in the Trophy Allocation section. The mismatch between 19 scenarios and 8 unit tests is unexplained.

---

## Axis 4 — Boundary Completeness

**Result**: PASS (with one MINOR gap)

For `enqueue`:
- Minimum valid: `len = 0` (empty queue) ✓
- Maximum valid: `len = capacity - 1` ✓ (one below capacity scenario)
- At capacity: `len = capacity` ✓ (multiple scenarios)
- One above max: implicit — `QueueFull` error ✓
- Overflow potential: addressed via Kani harness and anti-invariant ✓

For `dequeue`:
- Empty queue: returns `None` ✓
- Full queue: returns `Some(item)` ✓
- FIFO order: explicitly verified ✓

For `remaining_capacity`:
- Empty: returns `capacity` ✓
- Full: returns `0` ✓
- Partial: returns `capacity - len` ✓

**MINOR Gap**: `new(capacity = 0)` has no explicit scenario. Anti-invariant mentions it but no BDD scenario names this boundary.

**MINOR Gap**: `capacity()` has no dedicated scenario.

---

## Axis 5 — Mutation Survivability

**Result**: PASS (mutation table is well-structured)

| Mutation | Catching Scenario |
|----------|-----------------|
| `enqueue` ignores capacity | `action_queue_returns_queue_full_error_when_enqueue_at_capacity` ✓ |
| `backpressure` uses `>` not `>=` | `action_queue_emits_backpressure_warning_at_80_percent_capacity` ✓ |
| `backpressure` removed entirely | `action_queue_backpressure_warning_contains_depth_and_capacity` ✓ |
| `remaining_capacity` underflows | `action_queue_remaining_capacity_is_zero_when_full` ✓ |
| `dequeue` doesn't decrement len | `action_queue_len_is_zero_after_draining_all_items` ✓ |
| `is_full` checks `len == capacity + 1` | `action_queue_returns_queue_full_error_when_enqueue_at_capacity` ✓ |

All six critical mutations have named catching scenarios. The Kani harnesses add formal verification beyond BDD coverage.

---

## Axis 6 — Evidence Plan Audit

**Result**: PASS

- All scenarios have explicit `Given → When → Then` structure ✓
- Preconditions are stated explicitly in each `Given:` clause ✓
- Input values are concrete (N=3, N=4, N=10, etc.) not vague ✓
- Backpressure warning scenarios use concrete capacity values and explicit depth assertions ✓
- Kani harnesses use `kani::any()` for action items, not hardcoded data ✓
- Generated coverage (proptest) has bounded strategies with proptest's standard regression mechanism ✓
- Side effects (backpressure notification) are named explicitly ✓

---

## Summary of Findings

### LETHAL FINDINGS (1)

1. **Trophy allocation ratio 1:1 vs required 5:1** — 8 unit tests allocated for 8 pub fns. Required: 40 unit tests minimum. The test plan states "8 unit / 4 integration / 1 e2e" but the combinatorial coverage matrix shows 16 unit rows, and there are 19 named BDD scenarios. The discrepancy between allocated tests (8) and scenarios (19) is unexplained. Every pub fn needs ≥5 distinct test cases targeting different input boundaries and behavior subspaces.

### MINOR FINDINGS (2)

1. **`new(capacity = 0)` has no explicit BDD scenario** — the proptest anti-invariant mentions it but no scenario names this constructor boundary. Should have a scenario: "Given: queue with capacity 0; When: enqueue is attempted; Then: returns Err(...)" or "panics" if that is the specified behavior.

2. **`capacity() -> usize` has no dedicated scenario** — implicitly tested via remaining_capacity scenarios but not explicitly named. Add scenario: "Given: queue with capacity N; When: `capacity()` is called; Then: Returns N."

---

## Mandated Changes for Resubmission

1. **Expand trophy allocation to 40+ unit tests** — every pub fn needs ≥5 test cases covering: min input, max input, empty/zero state, full state, one-above-max (error), and key invariants. Current 8-unit allocation is a hard rejection gate.

2. **Add `new(capacity = 0)` explicit scenario** — must specify whether this is a panic, an error at construction, or an allowed degenerate queue.

3. **Add explicit `capacity()` scenario** — or merge into existing scenarios that already validate it implicitly.

4. **Resolve scenario count discrepancy** — clarify whether the 19 named BDD scenarios are individual tests or grouped. If grouped, specify which scenarios share a single test function and why.
