# Waiver Candidates — vb-8mdp.1

## No Behavior Waivers Requested

All proof obligations in this plan address genuine behavior requirements from the contract. No waivers are requested for any behavior-affecting exception.

## Non-Applicable Lane Waivers (not waivers per se — evidence citations)

| Lane | Non-Applicable Evidence |
|------|------------------------|
| Loom | IPC server is single-threaded sequential I/O; no concurrent memory ordering, no lock-free structures, no channel interleavings. Fragmented-frame behavior is about sequential buffer accumulation, not thread interleavings. |
| Miri | vb_ipc crate is `#![forbid(unsafe_code)]`. All byte reads go through safe `std::io::Read::read_exact` and `byteorder::LittleEndian::read_u32_le`. No raw pointers, no `MaybeUninit`, no aliasing. No UB paths exist in the safe IPC code. |
| Flux | Flux RS refinement types not yet in project scope. Decode order theorem is proven adequately via Kani (exhaustive bounded model checking over 2^192 inputs) + Verus (refinement proof binding to actual Rust implementation). |
| Cargo-fuzz | Kani exhausts all 2^192 possible 24-byte header combinations via symbolic execution — this is strictly more thorough than random byte-level fuzzing. Cargo-fuzz would exercise the same decode path with random bytes but would not provide exhaustiveness guarantees. |

All non-applicable lanes are justified with concrete evidence from the codebase and proof artifacts.