# Verification Layers: vb-h6ix — Replay Latest Execution Attempt Only

## Boundary

- **Verified kernel**: `vb_storage/src/recovery/replay/core.rs` — the pure replay filtering logic.
- **Runtime shell**: `vb_runtime/src/recovery.rs` — the runtime hydration from recovered frame seeds.
- **External systems excluded from formal proof**: Fjall persistence layer (tested via integration tests), IPC layer, generated workflow code.

---

## Layer Assignment

### Core Invariants

| Clause | Layer(s) | Rationale |
|--------|----------|-----------|
| INV-001 (deterministic replay) | `proptest` + `kani` | Proptest for broad input space coverage; Kani for bounded model checking of state machine transitions. |
| INV-002 (attempt selection independent of wall clock) | `lean` | Pure algebraic theorem: max_attempt selection is a total order property, provable as a pure function. |
| INV-003 (stale events cannot allocate live state) | `kani` + `proptest` | Kani for bounded proof of no allocation from filtered events; proptest for adversarial event interleaving. |
| INV-004 (tracker records only latest attempt) | `kani` + `proptest` | Kani bounded model check; proptest for mixed-attempt journal generation. |
| INV-005 (stale terminal does not win) | `proptest` + `kani` | Proptest for property coverage; Kani for state transition proof. |

### Preconditions

| Clause | Layer(s) | Rationale |
|--------|----------|-----------|
| PRE-001 (attempt numbers present) | `proptest` + `cargo-fuzz` | Property test for attempt number extraction; fuzzing for malformed event sequences. |
| PRE-002 (deterministic event ordering) | `proptest` + `kani` | Already covered by existing `events_for_run` tests; extended with mixed-attempt scenarios. |
| PRE-003 (consistent event slice) | `waiver` | Guaranteed by `FjallJournal::events_for_run` which is tested exhaustively in `journal.rs` tests. |

### Postconditions

| Clause | Layer(s) | Rationale |
|--------|----------|-----------|
| POST-001 (latest attempt state only) | `proptest` + `kani` | Core property; proptest for generated mixed-attempt journals; Kani for bounded proof. |
| POST-002 (stale events are diagnostics only) | `proptest` + `kani` | Property: stale events excluded from tracker and frame seed. |
| POST-003 (max attempt wins) | `lean` + `proptest` | Lean for algebraic proof of max selection; proptest for empirical verification. |
| POST-004 (stale retained for diagnostics) | `proptest` | Verify returned event list contains all input events (including stale). |
| POST-005 (stale terminal does not override) | `proptest` + `kani` | Covered by INV-005. |

### Error Handling

| Clause | Layer(s) | Rationale |
|--------|----------|-----------|
| ERR-ReplayDivergence | `proptest` + `kani` | Generate out-of-order step events and verify divergence is caught. |
| ERR-NonIdempotentActionBlocked | `proptest` + `kani` | Generate duplicate action events from stale attempt and verify blocking. |

---

## Lean Scope

- **Theorem module**: `vb_storage/src/recovery/replay/latest_attempt_theorem.lean` (to be created)
- **Rust target**: `replay_events` in `vb_storage/src/recovery/replay/core.rs`
- **Abstraction relation**: The journal event sequence is modeled as a finite list of records `(seq: Nat, attempt: Nat, event_kind)`. The replay output is the sub-sequence of events where `attempt = max_attempt(sequence)`.
- **Theorem shape**:
  - `latest_attempt_deterministic`: For any fixed input list, `max_attempt` is uniquely defined.
  - `stale_events_filtered`: Events with `attempt < max_attempt` do not appear in the live hydration output.
  - `stale_preserved_in_output`: All input events appear in the returned replay list.
- **Non-goals**: Lean will NOT prove properties about Fjall persistence, async scheduling, or runtime frame construction.

---

## Waivers

| Clause ID | Verification Layer Waived | Reason | Compensating Evidence | Owner | Expiration/Follow-up |
|-----------|-------------------------|--------|----------------------|-------|----------------------|
| PRE-003 | `proptest`, `cargo-fuzz` | `events_for_run` is already exhaustively tested in `journal.rs` tests covering sequential ordering, gap detection, and isolation. | Existing 40+ journal tests in `journal.rs`. | vb-h6ix agent | When `events_for_run` behavior changes or when formal proof is prioritized |
| ERR-Journal | `kani`, `proptest` (for journal error paths) | Journal errors are delegated to `JournalError` which is tested in `journal.rs` round-trip tests. The journal layer is external to the replay kernel. | Existing journal round-trip tests in `journal.rs`. | vb-h6ix agent | When journal error handling changes or when formal proof is prioritized |

---

## Test Plan Summary

See `martin-fowler-tests.md` for the full Given-When-Then scenarios.

| Category | Count |
|----------|-------|
| Happy path tests | 2 |
| Error path tests | 2 |
| Edge case tests | 4 |
| Contract verification tests | 5 |

---

## Verification Commands

```bash
# Fast gate (proptest + unit tests)
moon run :test --package vb_storage

# Standard gate (+ Kani)
moon run :verify-standard

# Deep gate (+ Miri on pure crates)
moon run :verify-deep

# Proof gate (+ Lean)
moon run :verify-proof

# Full gauntlet
moon run :verify-all
```
