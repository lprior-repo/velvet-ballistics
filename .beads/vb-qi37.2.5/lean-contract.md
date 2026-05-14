# Lean Contract / Theorem Kernel Plan — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 5 (proof-writer repair)

## Lean/Theorem Kernel Scope Determination

### Analysis

All obligations in this bead are **Rust-local** boundedness properties:

| Obligation | Type | Rust-Local? | Kernel Needed? |
|------------|------|-------------|----------------|
| INV-001 | StepBudget invariant | YES | NO |
| INV-002 | ValueStore cap invariant | YES | NO |
| INV-003 | count_total_steps bound | YES | NO |
| INV-004 | loop termination | YES | NO |
| INV-005 | budget monotonic | YES | NO |
| INV-006 | try_take monotonic | YES | NO |

### Rust-Local Determination

All obligations are proven within the Rust type system using:
- **Verus**: Pure functional specifications + ghost proofs
- **Kani**: Bounded model checking over concrete Rust types
- **Miri**: Undefined behavior detection for unsafe-free Rust

No obligation requires:
- External theorem prover (Lean, Coq, Isabelle)
- Algebraic specification beyond Rust's type system
- Protocol verification for external systems

## Theorem Kernel Plan

### Verus as Primary Kernel

Verus provides theorem-proving capability directly within Rust:

```
spec_fn spec_step_budget_invariant(...)
proof fn proof_remaining_bounded(...)
```

All obligations are verified with **0 errors** (proof-reviewer confirmed).

### No External Kernel Required

**Rationale**: This bead's obligations are boundedness properties over
finite Rust data structures. They do not require:
- Dependent types beyond Rust's trait system
- Algebraic semantics outside Rust's expression language
- Theorem prover interoperability

### Waiver Request

**Lean/theorem kernel waiver** is requested because:
- All obligations are Rust-local (VERUS-INV-001 through VERUS-INV-006)
- Verus owns all proof obligations
- No algebraic kernels beyond Verus scope
- Kani provides complementary bounded model checking
- Miri provides UB detection

## Artifact Cross-Reference

- Verus specs: `verification/verus/*.rs` (owned by Verus)
- Kani harnesses: `crates/vb_core/src/kani/*.rs` (owned by Kani)
- Miri: `cargo miri test --package vb_core`
- This file acknowledges **N/A for external theorem kernel**

## Owner

- **Kernel owner**: Verus (rust-local proofs)
- **Proof reviewer**: proof-reviewer (State 6)
