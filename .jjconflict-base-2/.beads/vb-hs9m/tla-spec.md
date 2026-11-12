# TLA+ Temporal Model Plan — vb-hs9m

## Non-applicability Rationale

**TLA+ is explicitly not applicable to vb-hs9m.**

The bead scope covers three distinct areas, none of which exhibit temporal/state-over-time behavior:

1. **TraceRing (bounded SPSC ring buffer):** A local data structure with purely local state transitions. `push` either succeeds or returns `false` (drop count increment). `drain` removes events. There are no liveness properties, fairness constraints, deadlock possibilities, concurrent writer conflicts (SPSC guarantees a single producer and single consumer), or asynchronous state machines. The ring buffer's "overflow" behavior (drop count) is observable but is a monotonic counter, not a state machine.

2. **EvidenceBundle (evidence container):** A passive serialization/deserialization container. `write_bundle` produces a file artifact; `read_bundle` reads it back. There are no asynchronous operations, retry logic, claim/lease protocols, or coordination protocols. Evidence files are written atomically (no partial-write states visible to consumers).

3. **Scenario/Catalog (BDD acceptance catalog):** A static compile-time data structure validated synchronously. `catalog()` returns a fixed slice; `validate_catalog()` performs a pure synchronous validation. No runtime state transitions occur.

**Why TLA+ does not apply:**

| Property | TraceRing | EvidenceBundle | Scenario/Catalog |
|----------|-----------|----------------|------------------|
| Workflow/Protocol | No | No | No |
| Scheduler | No | No | No |
| Queue with retry | No | No | No |
| Claim/Lease | No | No | No |
| Lifecycle state machine | No | No | No |
| Concurrent writers | No (SPSC enforced) | No | No |
| Distributed coordination | No | No | No |
| Liveness/eventuality | No | No | No |
| Fairness | No | No | No |
| Deadlock freedom | No (no locks) | No | No |
| State machine with N states | No | No | No |

**Compensating evidence for the no-TLA+ decision:**

- `TraceRing` boundedness and FIFO ordering are covered by 1077 lines of BDD-style adversarial unit tests (trace.rs) and Kani harness OBL-009 through OBL-011
- `EvidenceBundle` parse/serialize invariants are covered by Kani harnesses OBL-001 through OBL-004 and proptest round-trip properties OBL-005 through OBL-007
- `Scenario` catalog validation is covered by unit tests and integration tests in `vb_hxm0_acceptance_catalog.rs`

**TLA+ waiver granted with owner: `rust-contract state 3`, reason: no temporal/state-over-time behavior in bead scope, compensating evidence: unit tests + Kani + integration tests.**

---

## TLA+-Owned Clauses

**None.** All temporal behavior clauses are explicitly waived above.

---

## Alternative Formal Methods Used

Since TLA+ is not applicable, the following formal methods cover the critical invariants:

| Clause | Method | Evidence |
|--------|--------|----------|
| TraceRing boundedness | Kani harness + BDD unit tests | `kani/verify_trace_ring_bounds.rs`, `trace.rs` |
| EvidenceBundle parse never panics | Kani harness | `xtask/tests/bundle_tests.rs::schema_version_parse_non_panic` |
| EvidenceBundle round-trip | Proptest | `xtask/tests/bundle_tests.rs::OBL-005, OBL-006, OBL-007` |
| EvidenceBundle Miri clean | Miri | `cargo +nightly miri test --test bundle_tests` |
| Scenario catalog validation | Unit + integration tests | `vb_hxm0_acceptance_catalog.rs` |
