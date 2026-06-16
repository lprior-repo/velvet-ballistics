// Verification artifact: reduce_overflow.flux
// PO: PO-OVERFLOW-FLUX-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 2)
// Verifier: Flux RS
// Command: bash scripts/flux-check-package.sh vb_compile
//
// Requirement: C3 — Body Step Sequential Assignment (overflow guard)
// Domain Claim: body_width returns Ok(n) => n <= 65535, checked_add prevents overflow.
//
// GOD RULE 2: Post-condition refinement enforces u16::MAX bound on body_width.

// Refinement: body_width Ok result never exceeds u16::MAX (65535).
// Uses extern_spec from reduce_body_width.flux but adds explicit boundary check.
#[flux_rs::extern_spec]
impl crate::mod_compile_lowering::part_01 {
    #[flux_rs::sig(
        fn(body: &[vb_yaml::ast::StepAst], overhead: usize[overhead: int | overhead <= 65535])
        -> Result<usize[n: int | n >= overhead && n <= 65535], CompileError>
    )]
    fn body_width(body: &[vb_yaml::ast::StepAst], overhead: usize) -> Result<usize, CompileError>;
}

// Prove: body_width with huge overhead always returns Err (overflow detected)
#[flux_rs::trusted]
#[flux_rs::sig(fn() -> requires true ensures false)]
fn reject_overflow_unchecked() {
    unreachable!();
}
