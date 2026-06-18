// Verification artifact: proofs.rs
// Verifier: Verus
//
// Abstract mathematical models for vb_compile lowering functions.
// These spec functions model production behavior in sibling modules
// (part_01, part_04) as ghost code. No production bindings are present.
//
// REMOVED (proof-writer repair):
//   - 5 exec fn blocks (exec_body_width_uniform, exec_together_width_uniform,
//     exec_leaf_step_width, exec_width_node_parity, exec_together_ordering):
//     These were exec fn re-implementations with ensures clauses that assert
//     the exec fn's own logic satisfies a spec. They are NOT bindings to
//     production code. Production body_width takes &[StepAst], overhead and
//     iterates with canonical_body_step_width — different signature, has loops.
//     Cannot use reveal_with_fuel. GOD RULE 2 violation — re-implementations.
//   - 3 lemma_nat_mul_* trust markers (lemma_nat_mul_le_left, lemma_nat_mul_le_right,
//     lemma_nat_mul_one_right): Used assume(...) which is a GOD RULE 1 violation.
//   - 7 proof lemmas (lemma_together_width_ge_2, lemma_together_width_monotonic,
//     lemma_body_width_formula, lemma_leaf_width, theorem_width_node_parity_theorem,
//     lemma_together_ordering_invariant, lemma_body_width_monotonic): All proved spec
//     function properties only. No extern_spec! bindings to production code.
//
// These spec functions are ghost code (zero runtime cost) and serve as abstract
// models for documentation and future extern_spec! binding targets.

use vstd::prelude::*;

verus! {

// ========================================================================
// Spec functions (ghost code, abstract models)
// ========================================================================

/// Abstract model of body_width: overhead + step_count * step_width.
/// Production: part_01.rs body_width() takes &[StepAst], overhead and iterates.
/// Signature mismatch — cannot reveal_with_fuel. This is an abstract uniform model.
pub closed spec fn spec_body_width(step_count: nat, step_width: nat, overhead: nat) -> Option<nat> {
    if step_count == 0 {
        Some(overhead)
    } else if overhead + step_count * step_width > 65535 {
        None
    } else {
        Some(overhead + step_count * step_width)
    }
}

/// Abstract model of together_width: 2 + branch_count * branch_body_width.
/// Production: part_01.rs together_width() takes &[TogetherBranch] and iterates.
/// Signature mismatch — cannot reveal_with_fuel. This is an abstract uniform model.
pub closed spec fn spec_together_width(branch_count: nat, branch_body_width: nat) -> Option<nat> {
    if branch_count == 0 {
        None
    } else if 2 + branch_count * branch_body_width > 65535 {
        None
    } else {
        Some(2 + branch_count * branch_body_width)
    }
}

/// Abstract model of leaf step width for Set/Do primitives.
/// Production: part_01.rs canonical_body_step_width() returns Ok(1) for Set/Do.
/// Different signature (&StepPrimitive vs. constant) — cannot reveal_with_fuel.
pub closed spec fn spec_leaf_step_width() -> nat {
    1
}

/// Abstract model of Together emit node count (same as together_width).
pub closed spec fn spec_together_emit_count(branch_count: nat, branch_body_width: nat) -> Option<
    nat,
> {
    spec_together_width(branch_count, branch_body_width)
}

/// Width-node parity: together_width == together_emit_count.
pub closed spec fn spec_width_node_parity(branch_count: nat, branch_body_width: nat) -> bool {
    spec_together_width(branch_count, branch_body_width) == spec_together_emit_count(
        branch_count,
        branch_body_width,
    )
}

/// StepIdx ordering property for Together nodes.
pub closed spec fn spec_together_ordering(base_id: nat, branch_count: nat, width: nat) -> bool {
    width >= 2 && base_id + width - 1 >= base_id + branch_count
}

} // verus!
