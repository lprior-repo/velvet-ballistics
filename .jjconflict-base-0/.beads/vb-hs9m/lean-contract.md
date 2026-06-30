# Lean/Aeneas/Hax Theorem Kernel Projection — vb-hs9m

## Theorem Kernel Boundary

**Lean/Aeneas/Hax is explicitly not required for vb-hs9m.**

The algebraic properties of the trace ring buffer, evidence bundle, and BDD catalog are all expressible as:

1. **Unit test properties** (deterministic property checking over generated inputs)
2. **Kani harnesses** (bounded model checking for panic freedom, index safety, state transition properties)
3. **Verus spec functions** (pure function contracts over the Rust types)

No tiny theorem kernel extracted from the Rust code requires a proof assistant. The `TraceRing` invariants (boundedness, FIFO, monotonic drop count) are data-structure properties provable by testing and bounded model checking. The `EvidenceBundle` invariants (required field non-emptiness, parse format) are decidable properties over finite strings. The `Scenario` catalog invariants are set-theoretic properties over a static slice, checkable by iteration.

---

## Verus-Owned Rust-Local Proof Obligations

Verus owns the Rust-local pure proof obligations for vb-hs9m through the existing verification artifacts:

| File | Verus-owned Invariant |
|------|-----------------------|
| `verification/verus/run_frame_invariant.rs` | RunFrame lifecycle invariants (temporal to vb-hs9m scope but referenced by trace events) |
| `verification/verus/signals_invariant.rs` | Signal handling invariants |

**Note:** These Verus files are in the workspace but are **not** authored by vb-hs9m. They are referenced here to document the existing proof infrastructure that vb-hs9m's trace events interact with.

---

## Lean/Aeneas/Hax Waiver

**Owner:** `rust-contract state 3`
**Reason:** No algebraic theorem kernel in vb-hs9m scope requires extraction to a proof assistant. The trace ring buffer is a standard SPSC queue with bounded capacity — its properties are checkable by Kani and unit tests. The evidence bundle is a product type with serialization — its invariants are checkable by property tests. The BDD catalog is a static slice with validation — its properties are checkable by unit tests.
**Compensating evidence:** Kani harnesses OBL-001 through OBL-011, proptest round-trips OBL-005 through OBL-007, Miri OBL-008, and BDD unit tests in trace.rs
**Expiry:** Not applicable — waiver is permanent for this bead scope
