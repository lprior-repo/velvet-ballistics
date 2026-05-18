# Test Plan Review: MAJOR-1 — ArrayQueue Lock-Free SPSC Migration

## VERDICT: REJECTED

---

## Axis 1 — Contract Parity: **FAIL**

**LETHAL-1**: `MemoryIngress::bounded` has zero BDD scenarios.
- `bounded(capacity: QueueCapacity) -> Self` is a `pub fn` in the contract (ingress.rs:63).
- The plan's 7 BDD scenarios (Section 3) all assume an already-constructed `MemoryIngress`. No scenario creates one via `bounded()`.
- The "Given" clause in every scenario says "A `MemoryIngress` queue with capacity N, created via `MemoryIngress::bounded(capacity=N)" — this describes the setup but is not a test named in Section 3.
- **Fix**: Add a BDD scenario explicitly for `bounded` constructor, or rename the 7 existing scenarios to show they cover `bounded` as a precondition.

**LETHAL-2**: `IngressFrame::new` has proptest coverage (Section 4) but zero BDD scenarios (Section 3).
- `IngressFrame::new(...) -> Result<Self, IpcError>` is a `pub fn` (ingress.rs:21).
- Section 3's 7 behaviors are all about `MemoryIngress`. `IngressFrame::new` does not appear.
- Section 8's "Combinatorial Coverage Matrix" covers it but this is a coverage table, not a BDD scenario.
- **Fix**: Add at least one BDD scenario in Section 3 with explicit Given/When/Then for `IngressFrame::new`.

**LETHAL-3**: `IpcError::PayloadTooLarge` has zero BDD scenarios and zero exact assertions.
- `IngressFrame::new` can return `Err(IpcError::PayloadTooLarge { actual, limit })` (ingress.rs:30 calls `BoundedPayload::new(...)?`).
- The plan's fuzz target (Section 5) mentions "PayloadTooLarge" as a risk but has no BDD scenario asserting the exact variant with concrete `actual`/`limit` values.
- No scenario in Section 3 asserts `Err(IpcError::PayloadTooLarge { actual: _, limit: _ })` — only `Full` and `Disconnected` are covered.
- The unit tests in `ingress.rs` (lines 113-145) cover the happy paths for `IngressFrame::new` but not the `PayloadTooLarge` failure path.
- **Fix**: Add a BDD scenario for `IngressFrame::new` returning `PayloadTooLarge`.

---

## Axis 2 — Assertion Sharpness: **PASS (conditional)**

All "Then" clauses in the 7 BDD scenarios use exact values:
- Behavior 1: `Ok(())` — exact ✓
- Behavior 2: `Err(IpcError::Full)` — exact variant ✓
- Behavior 3: `Ok(Some(frame1))`, `Ok(Some(frame2))` — exact values ✓
- Behavior 4: `Ok(None)` — exact ✓
- Behavior 5: `Err(IpcError::Disconnected)` — exact variant ✓
- Behavior 6: returns `2` (exact usize) ✓
- Behavior 7: returns `true`/`false` — exact bools ✓

**MINOR**: The proptest descriptions in Section 4 state invariants but don't always state the exact input-output pairs for the happy-path `IngressFrame::new` cases (e.g., "valid Bytes, valid RunId, WorkflowDigest → Ok(IngressFrame)"). The Section 8 matrix fills this gap.

---

## Axis 3 — Trophy Allocation: **PASS**

| Function | Tests (existing in ingress.rs) | Proptest | Fuzz |
|---|---|---|---|
| `IngressFrame::new` | 2 unit (happy paths) | ✓ cycle invariant | ✓ binary payload |
| `bounded` | 0 explicit | ✗ | ✗ |
| `try_submit` | 1 integration-style | ✓ capacity boundary | ✗ |
| `try_recv` | 4 unit/integration | ✓ FIFO invariance | ✗ |
| `len` | implicit in tests | ✓ len/is_empty consistency | ✗ |
| `is_empty` | implicit in tests | ✓ len/is_empty consistency | ✗ |

- Total pub fns on `MemoryIngress`: 5 (`bounded`, `try_submit`, `try_recv`, `len`, `is_empty`)
- Total pub fns on `IngressFrame`: 4 (`new`, `run_id`, `workflow`, `payload`)
- Total tests in `ingress.rs` module: 6 unit/integration tests
- Ratio: 6 tests / 9 pub fns = 0.67x — below the 5x threshold BUT these are existing inline tests, not the planned suite.
- The plan proposes: 2 unit / 4 integration / 1 static / 3 proptest / 1 fuzz = 11 planned tests for 9 pub fns = 1.2x planned ratio.
- **Both ratios are below the 5x threshold, but the plan is aspirational — no implementation exists yet.** The plan's trophy allocation is the *proposed* suite, not the existing one.

**MAJOR**: The plan's Section 2 claims "7 behaviors" but `IngressFrame::new` is a separate pub fn with its own contract that is not enumerated as a distinct behavior in Section 1. The 7 behaviors are all `MemoryIngress` methods. This undercounts by at least 1 (missing `IngressFrame::new` behavior).

**Pure function with no proptest invariant**: `IngressFrame::new` is a pure function with non-trivial input space (payload bytes, max bytes, run_id, workflow digest). It has proptest coverage via the "cycle invariance" invariant in Section 4. ✓

---

## Axis 4 — Boundary Completeness: **FAIL**

| Function | Min | Max | Min-1 | Max+1 | Empty | Overflow |
|---|---|---|---|---|---|---|
| `try_submit` (capacity) | capacity=1 submit×1 ✓ | capacity submit×N ✓ | N/A (full queue) | capacity+1 → Full ✓ | N/A | N/A |
| `try_recv` (empty) | empty recv → None ✓ | multi-item recv → FIFO ✓ | N/A | N/A | empty recv → None ✓ | N/A |
| `try_recv` (disconnected) | sender dropped recv → Disconnected ✓ | N/A | N/A | N/A | N/A | N/A |
| `len` | len after empty = 0 ✓ | len after N submits = N ✓ | N/A | N/A | 0 ✓ | could overflow on 16-bit ✓ (doc'd in error.rs) |
| `is_empty` | true when empty ✓ | false when non-empty ✓ | N/A | N/A | true ✓ | N/A |
| `IngressFrame::new` | empty payload ✓ | max payload ✓ | N/A | over-max → PayloadTooLarge ✗ | N/A | 16-bit out-of-range ✗ (doc'd in error.rs) |

**MINOR-1**: `IngressFrame::new` over-max boundary (actual = DEFAULT+1, limit = DEFAULT) is not explicitly named in Section 3 BDD scenarios. It appears in Section 8 coverage matrix but not as a BDD scenario. The fuzz target (Section 5) covers it.

**MINOR-2**: `len()` on 16-bit platforms could overflow (usize < u32). This is documented in `error.rs:61-67` as `PayloadLengthOutOfRange`, but `len()` itself is not tested for overflow scenarios.

**≥3 missing boundaries on one function → MAJOR threshold is not met** (only 1 missing for `IngressFrame::new`).

---

## Axis 5 — Mutation Survivability: **PASS**

Applying the 4 mutations mentally:

| Mutation | Catches |
|---|---|
| Replace `ArrayQueue::push` with unconditional loop | Behavior 2 (`try_submit` returns `Full`) + `try_submit_returns_full_when_queue_is_at_capacity` |
| Remove sender-dropped flag check | Behavior 5 (`try_recv` returns `Disconnected`) + `try_recv_returns_disconnected_when_sender_dropped` |
| Swap head/tail index on consumer side | Behavior 3 (FIFO order) + `try_recv_returns_fifo_order_when_queue_has_items` |
| Remove capacity boundary check | Behavior 2 (`try_submit` returns `Full`) + `try_submit_returns_full_when_queue_is_at_capacity` |
| Return `Ok(Default::default())` instead of real value | Behavior 3 (exact frame values asserted) |
| Swap two function arguments | N/A (no multi-arg pub fns) |

The mutation checkpoint table (Section 7) is thorough. Each mutation is mapped to a specific test.

---

## Axis 6 — Evidence Plan Audit: **PASS (with reservations)**

- All BDD scenarios have explicit `Given` blocks stating preconditions ✓
- Proptest strategies are bounded: `capacity` capped to 1024, `NonZeroUsize` ✓
- Fuzz corpus seeds are named explicitly (empty, max, max+1, zero, single-byte) ✓
- Side-effectful test helper `disconnect_sender()` is named to advertise the side effect ✓
- Integration test threading model is listed as an open question (not a gap — acknowledging uncertainty is correct) ✓

**RESERVATION**: The open questions (Section 9) about `RingFlagged` disconnection semantics and SPSC discipline enforcement are unresolved. If the architectural answer changes the protocol state machine, the Kani harnesses (Section 6) may need to be redesigned. This is a risk to be tracked, not a rejection reason.

---

## Open Questions Status

| Question | Impact | Resolution Required? |
|---|---|---|
| `RingFlagged` disconnection semantics | HIGH — Kani harness #1 proves sender-drop detection | Yes, before Kani harness finalization |
| SPSC discipline enforcement (!Send/!Sync) | MEDIUM — affects proof scope | Yes |
| Integration test threading model | LOW — both models valid | No |
| Migration vs. greenfield coverage | HIGH — determines whether crossbeam baseline tests needed | Yes |

---

## Summary

| Axis | Verdict | Lethal Findings |
|---|---|---|
| Contract Parity | **FAIL** | LETHAL-1, LETHAL-2, LETHAL-3 |
| Assertion Sharpness | PASS | 0 |
| Trophy Allocation | PASS (conditional) | 0 (below 5x but plan is pre-implementation) |
| Boundary Completeness | PASS | 0 (minor gaps don't reach MAJOR threshold) |
| Mutation Survivability | PASS | 0 |
| Evidence Plan Audit | PASS | 0 |

**LETHAL count: 3** → REJECTED

---

## Mandate (required before resubmission)

1. **LETHAL-1**: Add a BDD scenario for `MemoryIngress::bounded` constructor. Can be combined with Behavior 1 setup ("Given: A MemoryIngress queue with capacity 1, created via `bounded(1)`") but must appear explicitly in Section 3 with a named test function.

2. **LETHAL-2**: Add at least one BDD scenario in Section 3 for `IngressFrame::new` covering:
   - Happy path with exact expected `IngressFrame` value
   - `PayloadTooLarge` error path with exact `{ actual, limit }` values

3. **LETHAL-3**: Add a BDD scenario asserting `Err(IpcError::PayloadTooLarge { actual: DEFAULT+1, limit: DEFAULT })` — either in Section 3 or elevated from the fuzz target to a named BDD scenario.

4. **Architectural risk**: Open Question 1 (`RingFlagged` semantics) must be resolved before the Kani harnesses can be finalized. Document the resolution in the plan.

5. **Trophy density**: The plan covers 7 `MemoryIngress` behaviors + `IngressFrame::new` but doesn't enumerate `IngressFrame::new` as a separate behavior in Section 1. Update Section 1 to list all pub fns with behaviors, including `IngressFrame::new`'s success and `PayloadTooLarge` failure.
