# TLA+ Temporal Model Plan — VerificationProof Gate Inputs

## Boundary

- **Temporal/workflow behavior**: None — this bead is about Rust type-level verification proof derivation, not workflow protocols or concurrent state machines.
- **Rust/core behavior excluded from TLA+**: Gate validation (structure/checksum), proof flag defaults, policy-gated admission decisions.
- **External systems abstracted**: None — no external distributed systems involved.
- **Non-applicability rationale**: The admission flow is a sequential Rust function with no temporal properties, liveness requirements, fairness conditions, or state machine transitions that would benefit from TLA+ modeling. The gates are pure codec/validation functions.

---

## TLA+-Owned Clauses

**None** — no temporal, protocol, scheduler, queue, retry, claim/lease, lifecycle, concurrent, or distributed behavior is in scope for this bead.

---

## Gate 1 — Structure Validation (Rust-local)

This is a pure function: `CompiledWorkflow::try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError>`.

- No temporal behavior
- No state machine
- No concurrency
- Pure codec validation

**Rust-local proof layer**: Verus (pure function postconditions, loop invariants, panic freedom)

---

## Gate 2 — Checksum Validation (Rust-local)

This is a pure function: BLAKE3 hash computation + comparison.

- No temporal behavior
- No state machine
- No concurrency
- Pure codec/hashing validation

**Rust-local proof layer**: Verus + Kani (bounded model check for hash mismatch path)

---

## Policy State Machine (Trivial)

The `RuntimePolicy` enum has three variants:

```rust
enum RuntimePolicy {
    Relaxed,   // skip gates
    Journaled, // enforce gates, no SyncAll
    Strict,    // enforce gates + SyncAll
}
```

This is not a TLA+ state machine — it is a single dispatch enum resolved in one match expression with no transitions, liveness, or fairness requirements.

**Proof**: Unit test coverage via `vb_2bok_durability_gate_tests.rs` BDD scenarios:
- `bdd_relaxed_policy_accepts_without_gate_validation`
- `bdd_journaled_policy_enforces_both_gates`
- `bdd_strict_policy_enforces_gates_and_syncall`

---

## Waivers

| Clause | Waiver Reason | Owner | Expiry |
|--------|--------------|-------|--------|
| Any TLA+ model | Admission flow is a sequential Rust function with no temporal properties | vb-core-proof-gate-inputs | N/A — non-applicable by design |

---

## Verification Layers for Gates

Since no TLA+ applies, the gates are covered by:

- **Gate 1 (structure)**: Verus (pure postconditions on `try_from_parts`), Kani (bounded model check), proptest (shrinking invalid inputs)
- **Gate 2 (checksum)**: Verus (pure postconditions), Kani (bounded model check for mismatch path)
- **Policy dispatch**: Unit tests + BDD scenarios in `vb_2bok_durability_gate_tests.rs`
- **Warnings**: Unit tests for `VerificationWarning::is_valid`, gate range

---

## Evidence Commands

- `cargo test -p vb_storage submit_artifact_relaxed submit_artifact_journaled submit_artifact_strict` — policy dispatch BDD tests
- `cargo test -p vb_storage verification_proof gate_count warning` — unit tests
- `moon run :verify-proof` — Verus/Kani gauntlet lane
