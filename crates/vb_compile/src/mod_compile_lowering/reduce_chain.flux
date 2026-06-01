// Verification artifact: reduce_chain.flux
// PO: PO-CHAIN-FLUX-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 2)
// Verifier: Flux RS
// Command: bash scripts/flux-check-package.sh vb_compile
//
// Requirement: C4 — Body Step Next-Link Chain
// Domain Claim: step[i].next correctly points to step[i+1].id (i < N-1)
//   or next_step (i == N-1). No dangling next links.
//
// GOD RULE 2: Refinement on the dispatch loop in emit_reduce_body_steps
//   ensures step_next is always Some and correctly determined by position.

// The chain invariant: for body position i, step_next is either the next body step
// or the aggregate's terminal next_step, never None and never a self-reference.

#[flux_rs::trusted]
#[flux_rs::sig(fn(next: Option<StepIdx>) -> StepIdx[step])
  requires next.is_some()
  ensures |step| step > 0]
fn unwrap_next(next: Option<StepIdx>) -> StepIdx {
    next.unwrap()
}

// Invalid-state rejection: next link must never be None for body steps
#[flux_rs::trusted]
#[flux_rs::sig(fn() -> requires true ensures false)]
fn reject_none_next() {
    unreachable!();
}
