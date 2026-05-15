# Theorem Kernel Projection — vb-0253.1

## Boundary

- **TLA+-owned temporal model**: Bounded-queue state machine, FIFO ordering, at-most-one-pop-per-tick. See `tla-spec.md`.
- **Verus-owned Rust core**: All pure Rust invariants and postconditions on `ShardCommandQueue` methods. Verus is the primary formal proof layer for this bead.
- **Theorem-owned kernel**: **None.** This bead describes a bounded queue wrapper. The algebraic properties of the queue (FIFO, capacity bounds, non-blocking enqueue) are straightforward enough to express in Verus without requiring a Lean/Aeneas/Hax extraction. No parser grammar, codec, algebraic protocol lattice, or arithmetic bound theorems are needed.
- **Rust/runtime shell**: Non-blocking `enqueue` failure is the shell behavior (delegates to `ArrayQueue.push`). `tick` consumption limit is enforced by `Shard`, not the wrapper.
- **External systems excluded**: None.

## Theorem-Owned Clauses

**None.** This bead does not require a theorem-prover kernel. All pure Rust critical invariants are expressible in Verus:

| Contract Clause | Formal Approach | Reason No Theorem Kernel |
|-----------------|-----------------|--------------------------|
| INV-001 (capacity fixed) | Verus `spec fn` + `proof fn` | Simple field assignment; no algebraic extraction needed |
| INV-002 (0 ≤ len ≤ capacity) | Verus `invariant` | Bounded nat relation; direct in Verus |
| INV-003 (len + remaining = capacity) | Verus `invariant` | Trivial arithmetic; direct in Verus |
| INV-005 (FIFO order) | Verus `spec fn` + `proof fn` | Queue abstraction directly in Verus |
| POST-002 (enqueue Ok/Err) | Verus postcondition | Result mapping from `ArrayQueue.push` |
| POST-005 (pop Option) | Verus postcondition | Option result with FIFO refinement |

## Lean/Aeneas/Hax Obligations

**None.** No obligations for this bead.

## Waivers

- **Theorem kernel waiver**: No Lean/Aeneas/Hax required for `ShardCommandQueue`. The wrapper is a thin domain-named boundary over `ArrayQueue`; all meaningful invariants are local to Rust's type system and expressible in Verus. Owner: vb-0253.1 contract phase. Reason: scope does not include algebraic theorem extraction beyond what Verus can cover. Expiry: bead close. Compensating evidence: Verus proofs + unit tests + property-based tests.
