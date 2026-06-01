// Verification artifacts: Kani harnesses for ForEach digest coverage
// Bead: vb-xi2f.28
// State: 5 (proof-writer)
// PO-K-FE-01 through PO-K-FE-10
//
// GOD RULE 1: Uses kani::any() with kani::assume() bounds for exhaustive
//   input generation. No hardcoded structural inputs.
// GOD RULE 2: Binds to actual production digest_step_primitive and canonical_digest.
// GOD RULE 4: Loop-free: proofs use bounded recursion via assume-constrained depth.

#![cfg(kani)]

mod kani_digest_determinism;
mod kani_digest_foreach_at_once;
mod kani_digest_foreach_at_once_equiv;
mod kani_digest_foreach_body;
mod kani_digest_foreach_delimiter;
mod kani_digest_foreach_exhaustive;
mod kani_digest_foreach_input;
mod kani_digest_foreach_variable;

// === vb-xi2f.21 Nested ForEach Harnesses ===
mod kani_emit_body_set;
mod kani_nested_foreach_dispatch;
mod kani_nested_foreach_offsets;
mod kani_nested_foreach_recursion;
mod kani_nested_foreach_width;
