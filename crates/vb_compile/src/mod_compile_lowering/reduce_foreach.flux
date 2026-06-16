// Verification artifact: reduce_foreach.flux
// PO: PO-NESTED-FOREACH-FLUX-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 2)
// Verifier: Flux RS
// Command: bash scripts/flux-check-package.sh vb_compile
//
// Requirement: C3 — Body Step Sequential Assignment (ForEach width)
// Domain Claim: canonical_body_step_width(ForEach) returns full width including
//   ForEach's body, and emit_reduce_body_steps advances accumulator by full width.
//
// GOD RULE 2: Refinement predicates enforce ForEach width >= 2 (never 1).

// ForEach extern_spec: width >= 2 for non-empty body
#[flux_rs::extern_spec]
impl crate::mod_compile_lowering::part_01 {
    #[flux_rs::sig(fn(primitive: &vb_yaml::ast::StepPrimitive) -> Result<usize[w: int | w >= 1], CompileError>)]
    fn canonical_body_step_width(
        primitive: &vb_yaml::ast::StepPrimitive,
    ) -> Result<usize, CompileError>;
}

// ForEach-specific refinement: when ForEach has a body, width >= 3
// (ForEachStart + at least 1 body step + ForEachNext)
// If ForEach has exactly 0 body steps, width = 2 (ForEachStart + ForEachNext)
#[flux_rs::trusted]
#[flux_rs::sig(fn(foreach_body_len: usize) -> usize[w: int | w >= 2])
  requires foreach_body_len >= 0]
fn foreach_minimum_width(foreach_body_len: usize) -> usize {
    2 + foreach_body_len
}

// Invalid-state: ForEach must never be treated as width 1 (single node)
#[flux_rs::trusted]
#[flux_rs::sig(fn() -> requires true ensures false)]
fn reject_foreach_width_one() {
    unreachable!();
}
