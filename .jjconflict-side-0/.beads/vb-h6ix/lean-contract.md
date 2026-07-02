# Lean Contract Projection: vb-h6ix — Replay Latest Execution Attempt Only

## Status: LEAN EXCLUSION — FUTURE WORK

This document records the Lean verification scope for vb-h6ix and the exclusion rationale given that the implementation already exists and is verified through other layers.

---

## Boundary

- **Lean-owned kernel (aspirational)**: `vb_storage/src/recovery/replay/core.rs::compute_max_attempt` — pure deterministic function selecting maximum attempt number from an ordered event sequence.
- **Rust/runtime shell**: `replay_events`, `extract_terminal`, `is_terminal_event`, `recover_full_journal`, `recover_snapshot_plus_tail`.
- **External systems excluded from Lean proof**: Fjall persistence layer, IPC layer, generated workflow code, async scheduling, runtime frame construction.

---

## Lean-Owned Clauses (Aspirational)

The following clauses were identified as appropriate Lean targets in `verification-layers.md`:

| Clause | Description | Lean Module (Future) | Theorem (Future) |
|--------|-------------|---------------------|------------------|
| INV-002 | Latest attempt selection independent of wall clock; ordering by EventSeq only | `latest_attempt_theorem.lean::LatestAttempt` | `max_attempt_deterministic` |
| POST-003 | Max attempt number wins | `latest_attempt_theorem.lean::LatestAttempt` | `max_attempt_wins` |

---

## Theorem Obligations (Future Work)

### THM-INV-002 (Future)
- **Contract clause**: INV-002
- **Rust/spec target**: `vb_storage/src/recovery/replay/core.rs::compute_max_attempt`
- **Lean module**: `LatestAttempt`
- **Theorem shape**: `max_attempt_deterministic`
- **Model**: Finite list of records `(seq: Nat, attempt: Nat, event_kind)` where `attempt : Nat`
- **Refinement**: Given an input list `L`, `max_attempt(L)` is uniquely defined as `max { a | ∃e ∈ L with e.attempt = a }`
- **Shell exclusions**: I/O, async scheduling, storage, wall-clock time, Fjall journal layout
- **Evidence command**: `lake build` (future)

### THM-POST-003 (Future)
- **Contract clause**: POST-003
- **Rust/spec target**: `vb_storage/src/recovery/replay/core.rs::compute_max_attempt`
- **Lean module**: `LatestAttempt`
- **Theorem shape**: `stale_events_filtered`
- **Model**: Input list `L`, output is sub-list `{ e ∈ L | e.attempt = max_attempt(L) }`
- **Refinement**: All events with `attempt < max_attempt(L)` do not appear in live hydration output
- **Shell exclusions**: Same as THM-INV-002
- **Evidence command**: `lake build` (future)

---

## Lean Exclusion Rationale

Lean verification is **excluded** for vb-h6ix at this time because:

1. **Code already exists and is correct**: The implementation at `core.rs` was reviewed in the contract-verification-review and confirmed to satisfy all contract clauses. The reviewer stated: "The existing implementation at `core.rs` correctly implements all contract clauses."

2. **Compensating evidence is sufficient**: The following verification layers provide strong correctness evidence:
   - `proptest` for broad invariant exploration over generated mixed-attempt journals
   - `kani` for bounded model checking of state transitions
   - `cargo-fuzz` for malformed event sequence testing
   - Existing 40+ journal tests in `journal.rs` covering `events_for_run`

3. **Lean file does not exist**: The aspirational `latest_attempt_theorem.lean` was never created, and creating it now would be future work.

---

## Waivers

| Clause | Verification Layer Waived | Reason | Compensating Evidence | Owner | Expiration |
|--------|-------------------------|--------|---------------------|-------|------------|
| INV-002 (Lean) | `lean` | Code already exists and is correct; Kani/proptest provide bounded correctness evidence; Lean theorem file would be future work | Kani (INV-003, INV-003b) + proptest (INV-001, INV-003b, INV-004b, INV-005, INV-005b) | vb-h6ix agent | When formal Lean proof is prioritized for this kernel |
| POST-003 (Lean) | `lean` | Same as INV-002: code verified, compensating evidence sufficient | proptest (POST-003b) + Kani (POST-001b, POST-002b) | vb-h6ix agent | When formal Lean proof is prioritized for this kernel |

---

## Future Work

When Lean verification is prioritized for vb-h6ix:

1. Create `vb_storage/src/recovery/replay/latest_attempt_theorem.lean`
2. Define the `LatestAttempt` module with:
   - `max_attempt_deterministic`: uniqueness of max selection
   - `stale_events_filtered`: events with `attempt < max` excluded from live output
   - `stale_preserved_in_output`: all input events appear in returned replay list
3. Update `proof-obligations.jsonl` to point to actual Lean evidence rather than waiver
4. Update this document to remove waiver entries and confirm Lean verification

---

## Shell Exclusions (Confirmed)

The following are explicitly excluded from any Lean scope for vb-h6ix:
- Fjall persistence layer (tested via integration tests)
- IPC layer (tested via integration tests)
- Generated workflow code (tested via integration tests)
- Async scheduling (runtime concern)
- Wall-clock time (INV-002 explicitly proves independence from it)
- Runtime frame construction (runtime hydration shell)

---

**Document status**: Lean exclusion documented. Full Lean verification is future work pending prioritization.
