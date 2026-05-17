# Theorem Kernel Projection — vb-qi37.1.5

## Non-applicability Rationale

Lean/Aeneas/Hax is **not needed** for this bead.

The digest mismatch detection is a Rust-local pure function problem fully expressible in Verus:
- `WorkflowDigest` equality is a simple `[u8; 32]` byte comparison — trivially provable in Verus
- The recovery functions (`check_workflow_source_digest`, `check_compiled_ir_digest`, `verify_digests`) are pure functions over owned data — ideal for Verus spec/ghost functions
- No algebraic protocol lattice, no refinement chain beyond Rust, no theorem kernel extraction needed

**Verus is the sole formal proof layer for this bead.** Lean would add overhead without discriminatory power.

## Theorem-Owned Clauses

None.
