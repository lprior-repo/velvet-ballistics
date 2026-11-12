# Proof Strategy — vb-rpch Verus/Flux/Rust R2

Scope: repair rejected proof-planner artifacts for remaining Rust/Verus/Flux/Kani/proptest/fuzz attachment work. No production Rust or proof code is written here. The round-3 TLC approval is preserved only as bounded TLA/TLC evidence and is not used as Rust, Flux, Kani, Loom, Miri, proptest, or fuzz evidence.

## Discovery evidence

Run from `/home/lewis/src/vb-jpq7-jj-fix`:

```bash
command -v verus && verus --version; cargo flux --version; cargo kani --version; cargo fuzz --version; rustc --version; cargo --version; rtk grep -n "#!\[forbid\(unsafe_code\)|unsafe\b|flux_rs|verus!|proof fn|spec fn|tokio|spawn|async fn" "crates/vb_storage/src/recovery"
```

Observed: Verus `0.2026.05.05.d03e906`; Flux blocked with `error: no such command: flux`; cargo-kani `0.67.0`; cargo-fuzz `0.13.1`; Rust nightly `1.97.0-nightly`; recovery target files report `#![forbid(unsafe_code)]` and no unsafe/async/spawn/proof annotations.

## Risk classification

- Temporal/state-machine: `PRE-001`, `POST-009`; TLC round-3 bounded model preserved for causal/replay abstraction only.
- Rust-local invariant: `INV-002`, `INV-004`, `INV-005`.
- Bounded state/arithmetic: `INV-003`, `PRE-001`, `PRE-002`, `POST-009`.
- Refinement/type-state: all seven remaining clauses; Flux is applicable but tooling-blocked.
- Concurrency: not applicable to these synchronous recovery targets unless State 11 introduces concurrency, which is outside scope.
- Unsafe/UB: not applicable to current targets because recovery files forbid unsafe; State 11 must preserve this.
- Untrusted input: `INV-003`, `PRE-001`, `PRE-002`, `POST-009`; proptest and cargo-fuzz obligations planned.

## Execution routing

1. State 11 / Holzman Rust owns behavior-preserving proof attachment in production files.
2. State 5 proof-writer owns Verus/Kani/proptest/fuzz artifacts and must not edit production behavior.
3. Flux remains `blocked_tooling` until `cargo flux --version` succeeds; no behavior waiver is claimed.
4. TLC round-3 approval remains TLA/TLC-only evidence per `.beads/vb-rpch/proof-review-tlc-fix-round3.md` lines 83-87.
5. Rerun proof-plan-reviewer on the R2 suffix artifacts before proof-writer starts.

## Counts

Lane decisions: {'tla-plus': 7, 'verus': 7, 'kani': 7, 'flux-rs': 7, 'loom': 7, 'miri': 7, 'proptest': 7, 'cargo-fuzz': 7}. Planned obligations: {'production-rust-holzman': 7, 'verus': 7, 'kani': 7, 'flux-rs': 7, 'tla-plus': 2, 'proptest': 7, 'cargo-fuzz': 4}.
