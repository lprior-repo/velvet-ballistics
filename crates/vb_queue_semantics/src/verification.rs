#![forbid(unsafe_code)]
// Verus verification artifacts for vb_queue_semantics.
//
// The proof files live outside `src/` so normal Rust builds keep production
// code separate from verifier artifacts while `pub mod verification` remains
// discoverable for Verus-oriented tooling.

#[cfg(verus)]
#[path = "../verification/verus/mod.rs"]
pub mod verus;
