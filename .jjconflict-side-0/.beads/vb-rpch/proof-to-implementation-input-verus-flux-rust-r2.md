# Proof-to-Implementation Input — vb-rpch Verus/Flux/Rust R2

## State 11 attachment obligations

Production behavior attachment must satisfy `VFR-R2-RUST-ATTACH-001` through `VFR-R2-RUST-ATTACH-007`. Edits must be behavior-preserving, Holzman-compliant, and expose stable proof surfaces for Verus/Kani/Flux without proof-writer mutating production behavior.

## Proof artifact obligations

- Verus: `VFR-R2-VERUS-001` through `VFR-R2-VERUS-007`.
- Kani: `VFR-R2-KANI-001` through `VFR-R2-KANI-007`.
- Flux: `VFR-R2-FLUX-001` through `VFR-R2-FLUX-007`, blocked until cargo-flux exists.
- proptest: `VFR-R2-PROPTEST-001` through `VFR-R2-PROPTEST-007`.
- cargo-fuzz: `VFR-R2-FUZZ-001` through `VFR-R2-FUZZ-004`.
- TLA preservation: `VFR-R2-TLA-PRESERVE-001` and `VFR-R2-TLA-PRESERVE-002` cite round-3 TLC only as TLA evidence.

## Bridge caveats

TLC does not prove Rust implementation refinement. Flux blocked_tooling does not prove any refinement. Kani/proptest/fuzz evidence must be attached independently to source refs and cannot be replaced by TLC approval.
