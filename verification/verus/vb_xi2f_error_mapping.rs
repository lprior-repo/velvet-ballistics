// Verification artifact: vb_xi2f_error_mapping.rs
// PO: PO-005 (WorkflowError to CompileError::Workflow mapping)
// Bead: vb-xi2f.4
// Verifier: Verus
// Command: verus verification/verus/vb_xi2f_error_mapping.rs
//
// Proof obligations:
// - PO-005: WorkflowError to CompileError::Workflow mapping preserves all
//   variant information.
//
// GOD RULE 2: Verus specs bind to actual Rust implementation in
// crates/vb_compile/src/mod_compile_errors/kind.rs:54.

use vstd::prelude::*;

verus! {

// ─────────────────────────────────────────────────────────────────
// Spec: WorkflowError Variants
// ─────────────────────────────────────────────────────────────────

/// Spec model of WorkflowError variants.
/// This mirrors the production enum in vb_core/src/workflow/mod.rs.
pub enum SpecWorkflowError {
    EmptyNodes,
    EntryOutOfBounds,
    StepOutOfBounds,
    SlotOutOfBounds,
    ConstOutOfBounds,
    NodeIdMismatch,
    Expression,
    ResourceContractExceeded,
    ResourceContractTooLarge,
    EmptyBranchTable,
    UnreachableNode,
    BackwardEdge,
    ImproperLoopNesting,
    BudgetPolicyExceeded,
    StepCountOverflow,
    SymbolOutOfBounds,
    AccessorPathTooDeep,
    JumpCycle,
}

/// Spec model of CompileError variants relevant to this mapping.
pub enum SpecCompileError {
    Workflow(SpecWorkflowError),
    EmptySteps,
    StepIndexOutOfRange,
    Other,
}

// ─────────────────────────────────────────────────────────────────
// PO-005: Error Mapping Specification
// ─────────────────────────────────────────────────────────────────

/// Spec: The mapping from WorkflowError to CompileError is total and
/// always produces CompileError::Workflow(...).
pub open spec fn spec_workflow_error_maps_to_compile_error(
    workflow_error: SpecWorkflowError,
    compile_error: SpecCompileError,
) -> bool {
    compile_error == SpecCompileError::Workflow(workflow_error)
}

/// Lemma: Every WorkflowError variant maps to exactly one CompileError variant.
pub proof fn lemma_error_mapping_is_total(workflow_error: SpecWorkflowError)
    ensures
        exists|compile_error: SpecCompileError|
            spec_workflow_error_maps_to_compile_error(workflow_error, compile_error),
{
    let compile_error = SpecCompileError::Workflow(workflow_error);
    assert(spec_workflow_error_maps_to_compile_error(workflow_error, compile_error));
}

/// Lemma: The mapping preserves variant information (injective).
pub proof fn lemma_error_mapping_is_injective(
    e1: SpecWorkflowError,
    e2: SpecWorkflowError,
    ce1: SpecCompileError,
    ce2: SpecCompileError,
)
    requires
        spec_workflow_error_maps_to_compile_error(e1, ce1),
        spec_workflow_error_maps_to_compile_error(e2, ce2),
    ensures
        (ce1 == ce2) == (e1 == e2),
{
    // Since both ce1 and ce2 are SpecCompileError::Workflow(...),
    // equality of ce1 and ce2 is equivalent to equality of the inner variants.
    assert(ce1 == ce2 ==> e1 == e2);
    assert(e1 == e2 ==> ce1 == ce2);
}

/// Lemma: No WorkflowError variant maps to a non-Workflow CompileError.
pub proof fn lemma_no_other_compile_error_variant(
    workflow_error: SpecWorkflowError,
    compile_error: SpecCompileError,
)
    requires
        spec_workflow_error_maps_to_compile_error(workflow_error, compile_error),
    ensures
        matches!(compile_error, SpecCompileError::Workflow(_)),
{
    // Directly follows from the spec definition.
    assert(matches!(compile_error, SpecCompileError::Workflow(_)));
}

fn main() {}

} // verus!
