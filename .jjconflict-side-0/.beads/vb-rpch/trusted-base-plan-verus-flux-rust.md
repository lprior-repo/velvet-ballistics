# Trusted Base Plan — vb-rpch Verus/Flux/Rust

## Preserved TLC trust base

Do not weaken or replace `.beads/vb-rpch/proof-review-tlc-fix-round3.md`. Its approval is bounded finite TLA abstraction only and excludes Rust refinement, byte decoding, cryptographic digest computation, runtime ABI/policy lookup, scheduler queue details, and liveness/fairness.

## Verus/Rust trust base

- `vb_core::RunFrame::{new, write_slot_with_taint, set_pc, increment_executed, mark_*}`: trusted runtime frame construction boundary unless separately verified.
- `postcard` snapshot/slot/taint decoding: trusted codec boundary; Verus must model only typed success/error propagation.
- Fjall journal ordering/durability: external storage boundary; not part of this suffix.
- `HashSet` implementation: trusted library; proofs should model append-only key membership effects, not verify std internals.
- `EventSeq::new(expected.get().saturating_add(1))` in sequence validation: saturating arithmetic caveat must be explicitly modeled or routed to State 11 if it blocks proof soundness.
- `non_exhaustive` enums: proof surfaces must not assume external exhaustive construction beyond crate-local handled variants.

## Flux trust base / blocker

Flux is not currently installed as a cargo subcommand. `cargo flux --version` failed with `no such command: flux`. This is a tooling blocker only; it waives no behavior and proves nothing.

## Holzman Rust implementation constraints for State 11

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` in production changes.
- No unchecked indexing, slicing, casts, or arithmetic.
- Keep proof helpers small, pure, and gated where possible.
- Any behavior-affecting repair discovered by Verus/Flux must be implemented in State 11 and then sent back through proof/test review; proof-writer must not mutate behavior.
