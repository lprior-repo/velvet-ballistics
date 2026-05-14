# EPIC Test Plan: vb-qi37 — Master-Doc Completion Gap Plan

## Scope

Integration and coordination tests for the 7-child EPIC vb-qi37. Child beads carry individual test artifacts; this plan covers cross-bead coordination, dependency ordering, and end-to-end pipeline verification.

---

## 1. Dependency Ordering Verification

### 1.1 Band Ordering Gates

| Test ID | Description | Precondition | Assert |
|---------|-------------|---------------|--------|
| `T-ord-001` | vb-fb52 reaches State ≥4 before vb-2yb8/vb-2bok enter State 5 | `bd show vb-fb52 --state` | State ≥4 |
| `T-ord-002` | Band 2 (vb-2yb8, vb-78f9, vb-6azo) all reach State ≥4 before Band 3 enters State 5 | `bd show <each> --state` | All State ≥4 |
| `T-ord-003` | vb-2bok is never in State ≥5 while vb-2yb8 is State <4 | `bd show vb-2yb8 --state` then `bd show vb-2bok --state` | vb-2yb8 State ≥4 |
| `T-ord-004` | Band 3 parallel independence (vb-7gs9, vb-99n6 no cross-dependency) | Both enter State 5 independently | No ordering violation |

### 1.2 State Transition Sequence Verification

Run at each phase gate:
```bash
bd seq --epic vb-qi37 --format=table
```
Assert: Sequence respects `Foundation → Evidence → Gate` ordering.

---

## 2. Integration Point Tests

### 2.1 Journal Record Envelope (vb-fb52 → vb-2yb8)

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-int-001` | `encode_record`/`decode_record` roundtrip for every `record_kind_u16` variant | Unit: enumerate all 15 record kinds, encode then decode, assert identity |
| `T-int-002` | Atomic batch isolation: journal write of N records either all succeed or all abort | Integration: inject failure after N-1 records, verify zero partial state |
| `T-int-003` | vb-2yb8 proof matrix covers every `record_kind_u16` sequence emitted by vb-fb52 | Coverage: cross-reference proof matrix table against `record_kind` enum total count |

### 2.2 Action ABI Contract (vb-78f9 → vb-2yb8)

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-int-004` | Every `Idempotency` variant is schema-validated by vb-78f9 | Enum coverage test: parse each variant through validation path |
| `T-int-005` | vb-2yb8 replay-safety proofs enumerate all `Idempotency` variants | Coverage: verify proof matrix has row per variant |
| `T-int-006` | `ActionTicket` shape matches MASTER.md Section 19 schema | Schema test: validate serialized form against canonical JSON schema |

### 2.3 Shard Ownership (vb-7gs9 → vb-2bok)

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-int-007` | `ShardCommand::Submit` is sole admission path for runs | Integration: enumerate all run-submission code paths, assert only `Submit` route exists |
| `T-int-008` | vb-7gs9 ownership invariant holds under concurrent shard submission | Concurrency: spawn N shards submitting simultaneously, assert one-owner-per-run |
| `T-int-009` | vb-2bok gate fires iff vb-7gs9 ownership proof is present | Gate test: with proof, gate passes; with proof revoked, gate rejects |

### 2.4 Timer Wheel Routing (vb-99n6 → vb-7gs9)

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-int-010` | `TimerFired` routes to correct owning shard | Integration: fire timer, assert delivery to shard that owns the scheduled run |
| `T-int-011` | Timer ordering determinism: same schedule produces same fire order | Determinism: replay same timer schedule twice, assert identical fire sequence |
| `T-int-012` | vb-7gs9 run-stays-on-one-shard invariant holds after timer delivery | Invariant: post-timer-delivery, run's shard assignment unchanged |

### 2.5 Accepted Artifact Durability (vb-2bok ← vb-2yb8, vb-fb52)

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-int-013` | `RunAccepted` (record_kind=10) persists atomically before acknowledgement | Integration: intercept ack, verify journal flush completed |
| `T-int-014` | vb-2yb8 proof matrix documents durability boundary for record_kind=10 | Coverage: row exists for record_kind=10 with atomic guarantee statement |
| `T-int-015` | vb-2bok gate accepts artifact only when vb-2yb8 evidence for record_kind=10 is present | Gate test: evidence present → pass; evidence absent → reject |

### 2.6 Property Tests Cross-Coverage (vb-6azo → all)

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-int-016` | All MASTER.md Section 38 invariants pass on current codebase | Run `cargo test -p vb_core -p vb_runtime -- property_tests` |
| `T-int-017` | State machine transition invariants hold across all bead combinations | Integration: exercise all state transitions, assert no illegal state reachable |
| `T-int-018` | Taint safety invariants hold under concurrent load | Concurrency: run with taint-injection, assert no cross-shard taint leakage |
| `T-int-019` | Replay determinism holds for all record_kind sequences | Determinism: replay journal twice, assert identical terminal state |
| `T-int-020` | Ordering invariants hold under timer-wheel stress | Stress: rapid timer fires, assert all ordering guarantees maintained |

---

## 3. End-to-End Pipeline Tests

### 3.1 Full EPIC Pipeline Run

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-e2e-001` | All 7 children reach State 8 sequentially per dependency order | `bd seq --epic vb-qi37` → manually verify each child reaches Landed |
| `T-e2e-002` | `bd dolt push` syncs complete EPIC state to remote | Run `bd dolt push`, then `bd dolt clone` to fresh dir, compare state |

### 3.2 MASTER.md Round 2 DoD Alignment

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-e2e-003` | Phase 40 recovery evidence chain: vb-2yb8 + vb-2bok produce documented evidence | Review proof matrix and gate artifact; assert coverage of all Phase 40 entries |
| `T-e2e-004` | Phase 33 live-frame recovery: vb-7gs9 + vb-99n6 produce bounded ownership + timer determinism evidence | Review shard ownership proof and timer fire log; assert invariants documented |
| `T-e2e-005` | Phase 18 Action ABI: vb-78f9 validates all `Idempotency` variants | Run vb-78f9 test suite; assert 100% variant coverage |
| `T-e2e-006` | Phase 36 invariant coverage: vb-6azo property tests all green | `cargo test -p vb_core -p vb_runtime -- property`; assert all pass |
| `T-e2e-007` | Phase 16 atomic batches: vb-fb52 atomic write verified | Run vb-fb52 integration tests; assert all batch ops atomic |

### 3.3 Black-Hat Finding Coverage (MASTER.md Section 42)

| Test ID | Description | Method |
|---------|-------------|--------|
| `T-e2e-008` | Every Section 42 black-hat finding has ≥1 child bead addressing it | Cross-reference Section 42 findings table against child bead contracts |

---

## 4. Child Bead Coordination Smoke Tests

Run after each band closes:

### 4.1 Post-Foundation (vb-fb52 closed)

```bash
# Verify atomic batch API surface is stable
cargo test -p vb_storage -- atomic_batch_smoke
# Verify no breaking changes to journal record envelope
cargo test -p vb_storage -- record_envelope
```

### 4.2 Post-Evidence Band (vb-2yb8, vb-78f9, vb-6azo closed)

```bash
# Verify durability proof matrix compiles
cargo build -p vb_runtime --features evidence
# Verify action schema validation does not break runtime
cargo test -p vb_runtime -- action_schema
# Verify property tests pass
cargo test -p vb_core -- properties
```

### 4.3 Post-Gate Band (vb-7gs9, vb-2bok, vb-99n6 closed)

```bash
# Verify shard scheduler ownership proof compiles
cargo build -p vb_runtime --features gate_shard
# Verify durability gate triggers correctly
cargo test -p vb_validate -- durability_gate
# Verify timer wheel determinism
cargo test -p vb_runtime -- timer_determinism
```

---

## 5. Test Execution Schedule

| Phase | Gate | Tests Run |
|-------|------|------------|
| After vb-fb52 State ≥4 | Band 2 opens | `T-ord-001`, `T-int-001`–`T-int-003` |
| After Band 2 State ≥4 | Band 3 opens | `T-ord-002`, `T-ord-003`, `T-int-004`–`T-int-020` |
| After Band 3 State ≥4 | EPIC close gate | `T-ord-004`, `T-e2e-001`–`T-e2e-008` |
| On `bd dolt push` | Remote sync | `T-e2e-002` |

---

## 6. Pass Criteria

- All `T-*` tests pass
- No ordering constraint violations in `bd seq` output
- `bd dolt push` succeeds without conflict
- MASTER.md Round 2 status table updated to reflect closed evidence

---

## 7. Out of Scope

- Detailed unit test plans for individual child beads (per their own test-plan.md)
- Generated/IR parity equivalence
- CLI or Makepad UI coverage
