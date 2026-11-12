# Proof Writer Report — vb-rpch verus-flux-rust-r8 Flux RS

writer_skill: proof-writer  
bead: `vb-rpch`  
state: 5 proof/model/harness writing — Flux RS lane after tooling install  
workdir: `/home/lewis/src/vb-jpq7-jj-fix`  
date: 2026-05-24

## Inputs consumed

- `.beads/vb-rpch/proof-review-verus-flux-rust-r4.md`
- `.beads/vb-rpch/proof-findings-verus-flux-rust-r4.jsonl`
- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r2.planned.jsonl`
- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r7.written.jsonl`
- `.beads/vb-rpch/implementation-verus-flux-rust-r2.md`
- `.beads/vb-rpch/rust-refinement-obligations.verus-flux-rust-r2.jsonl`
- Mapped production files under `crates/vb_storage/src/recovery/`
- Existing Verus `verification/verus/vb_rpch_*.rs` shapes for property alignment

## Artifacts written

- `verification/flux/vb_rpch_flux_r8.rs`
- `.beads/vb-rpch/proof-writer-report-verus-flux-rust-r8.md`
- `.beads/vb-rpch/proof-evidence-verus-flux-rust-r8.md`
- `.beads/vb-rpch/trusted-base-ledger.verus-flux-rust-r8.jsonl`
- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r8.written.jsonl`

No TLA files were touched. No production behavior was changed. `crates/vb_storage/Cargo.toml` was not changed because single-file Flux mode did not require crate metadata or a `flux-rs` dependency.

## Obligation disposition

- `VFR-R2-FLUX-001`: proved at scoped Flux harness level for `UnsupportedRecoveryState` supported false-field invariant and union OR algebra.
- `VFR-R2-FLUX-002`: proved at scoped Flux harness level for checked u16 present-index positive count and positive seed-dimension predicate.
- `VFR-R2-FLUX-003`: partially proved at pure support-surface level for resolution monotonicity; production `HashSet` remains outside Flux r8.
- `VFR-R2-FLUX-004`: proved at scoped Flux harness level for `DigestCheck` rank/check hierarchy.
- `VFR-R2-FLUX-005`: proved hydrate snapshot-tail precondition predicate conjunction at pure-surface level.
- `VFR-R2-FLUX-006`: proved events-only non-empty precondition at pure-surface level.
- `VFR-R2-FLUX-007`: partially proved replay precondition support surface; full `replay_events` loop/effects remain outside Flux r8.

## Commands run

- `z3 --version` → exit 0, `Z3 version 4.16.0 - 64 bit`.
- `if command -v fixpoint >/dev/null; then fixpoint --version; else liquid-fixpoint --version; fi` → exit 0, `fixpoint 0.9.6.3.6`.
- `flux --version` → exit 0, `flux 4d329f2 (2026-05-23)`.
- `cargo flux -V` → exit 0, `cargo-flux 4d329f2 (2026-05-23)`.
- `cargo flux -p vb_storage --message-format human` → exit 0, crate driver smoke pass.
- `flux --crate-type lib --edition 2024 "verification/flux/vb_rpch_flux_r8.rs"` → exit 0, `37 checked; 0 trusted; 0 ignored; 24 constraints solved`.
- Trusted-boundary scan → exit 0, only `#![forbid(unsafe_code)]` matches.
- JSONL validation → expected after report write; evidence file records the command.

## Trust and ignored surfaces

- No `#[trusted]`, `#[trusted_impl]`, `#[extern_spec]`, `#[ignore]`, `#[no_panic]`, or `#[no_panic_if]` markers were added.
- Trusted-base ledger records limitations rather than hidden assumptions:
  - scoped single-file harness,
  - source-correspondence boundary,
  - `HashSet` membership abstraction for tracker monotonicity,
  - no executable trust/ignore markers found.

## Residual limitations / blockers

- Flux r8 does not mechanically prove production Rust bodies; it proves a scoped harness mapped to pure public proof surfaces.
- `ActionReplayTracker` `HashSet` insertion/contains behavior remains unproved by Flux.
- Full hydrate/replay behavior over `JournalEvent` slices, vector outputs, snapshot decoding, and replay loop effects remains outside Flux r8.
- Non-Flux lanes from prior review remain outside this sublane: Kani/proptest/fuzz/Rust attachment/provenance are not closed here.

## Reviewer rerun readiness

The Flux tooling blocker from r4 is repaired for this sublane. Proof-reviewer may rerun for the Flux r8 artifacts, with the above scoped-harness limitations treated as explicit review inputs rather than hidden proof passes.
