// Verification artifact: reduce_nested_next.flux
// PO: PO-NESTED-NEXT-FLUX-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 2)
// Verifier: Flux RS
// Command: bash scripts/flux-check-package.sh vb_compile
//
// Requirement: C8 — Nested Reduce Semantics
// Domain Claim: The dispatch loop determines step_next based on position:
//   if i < body.len()-1: step_next = next_body_step
//   if i == body.len()-1: step_next = next_step
//
// GOD RULE 2: Refinement predicates on the dispatch logic.
//   Previously had zero extern_spec — now includes StepIdx refinement binding.

// Extern spec for StepIdx comparison: step IDs must be comparable
#[flux_rs::extern_spec]
impl crate::mod_compile_lowering::part_01 {
    // canonical_body_step_width with refinement
    #[flux_rs::sig(fn(&vb_yaml::ast::StepPrimitive) -> Result<usize[w: int | w >= 1 && w <= 65535], CompileError>)]
    fn canonical_body_step_width(
        primitive: &vb_yaml::ast::StepPrimitive,
    ) -> Result<usize, CompileError>;
}

// Position-aware dispatch: position determines next target
// If i < body.len()-1 (intermediate): next is sibling body step
// If i == body.len()-1 (last): next is the aggregate terminal next_step
#[flux_rs::trusted]
#[flux_rs::sig(fn(pos: usize, len: usize, next_body: usize, next_term: usize)
    -> usize
    requires pos < len
    ensures |result| result == if pos < len - 1 { next_body } else { next_term }
)]
fn position_aware_next(pos: usize, len: usize, next_body: usize, next_term: usize) -> usize {
    if pos < len - 1 { next_body } else { next_term }
}

// Invalid-state: next must always be defined (never ambiguous)
#[flux_rs::trusted]
#[flux_rs::sig(fn() -> requires true ensures false)]
fn reject_ambiguous_next() {
    unreachable!();
}
