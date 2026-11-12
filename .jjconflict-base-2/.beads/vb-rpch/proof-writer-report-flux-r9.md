# Proof Writer Report — vb-rpch Flux r9

writer_skill: proof-writer  
flux_skill: flux-rs invoked  
bead: `vb-rpch`  
state: 5 proof/model/harness repair — Flux RS r9  
workdir: `/home/lewis/src/vb-jpq7-jj-fix`  
date: 2026-05-24

## Inputs consumed

- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r2.planned.jsonl`
- `.beads/vb-rpch/proof-review-verus-flux-rust-r4.md`
- `.beads/vb-rpch/implementation-verus-flux-rust-r2.md`
- `.beads/vb-rpch/rust-refinement-obligations.verus-flux-rust-r2.jsonl`
- `crates/vb_storage/Cargo.toml`
- `crates/vb_storage/src/recovery/types.rs`
- `crates/vb_storage/src/recovery/hydrate.rs`
- `crates/vb_storage/src/recovery/replay/core.rs`
- `crates/vb_storage/src/recovery/replay/summary.rs`
- Flux references: `/home/lewis/.opencode/skill/flux-rs/references/flux-harness.md`, `flux-practice.md`, `flux-patterns.md`, `flux-deep-guide.md`

## Artifacts written

- `verification/flux/vb_rpch_flux_r9.rs`
- `.beads/vb-rpch/proof-writer-report-flux-r9.md`
- `.beads/vb-rpch/proof-evidence-flux-r9.md`
- `.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl`
- `.beads/vb-rpch/proof-obligations.flux-r9.written.jsonl`

No production behavior was modified. A temporary `[package.metadata.flux] enabled = true` probe was reverted after `cargo flux -p vb_storage` hit a Flux internal compiler error in unrelated `admission.rs` code.

## Obligation disposition

- `VFR-R2-FLUX-001`: proved at scoped Flux harness level for supported false-field invariant, union OR algebra, and union-matches-flags production helper semantics.
- `VFR-R2-FLUX-002`: proved at scoped Flux harness level for positive present-index dimension count, absent zero count, observed-dimension predicate, and positive seed dimensions.
- `VFR-R2-FLUX-003`: repaired r8 weakness. Monotonicity after completion/failure is asserted through refined post-state return types and direct `is_resolved_surface` calls; no vacuous `A || B` proof remains. Production HashSet semantics remain ledgered outside Flux.
- `VFR-R2-FLUX-004`: proved digest strict hierarchy and check-level predicates.
- `VFR-R2-FLUX-005`: proved snapshot-tail precondition conjunction and positive hydrate dimensions at pure-surface scope.
- `VFR-R2-FLUX-006`: proved events-only non-empty precondition.
- `VFR-R2-FLUX-007`: proved stale/current attempt, stale-state-effect, and step-order-divergence pure surfaces; full replay loop effects remain residual.

## Commands run

- `z3 --version` → exit 0, `Z3 version 4.16.0 - 64 bit`.
- `if command -v fixpoint >/dev/null; then fixpoint --version; else liquid-fixpoint --version; fi` → exit 0, `fixpoint 0.9.6.3.6`.
- `flux --version` → exit 0, `flux 4d329f2 (2026-05-23)`.
- `cargo flux -V` → exit 0, `cargo-flux 4d329f2 (2026-05-23)`.
- `flux --crate-type lib --edition 2024 "verification/flux/vb_rpch_flux_r9.rs"` → exit 0, `50 checked; 0 trusted; 0 ignored; 38 constraints solved`.
- Temporary crate-mode metadata probe: `cargo flux -p vb_storage --message-format human` → exit 101, Flux internal compiler error at `crates/vb_storage/src/admission.rs:270`; metadata reverted.
- Post-revert crate driver smoke: `cargo flux -p vb_storage --message-format human` → exit 0, `Finished flux profile ... in 0.05s`.
- Trusted-boundary scan → exit 0, only `#![forbid(unsafe_code)]` matches.
- JSONL validation → exit 0, 5 trusted-base rows and 7 written-obligation rows valid.

## Trust and ignored surfaces

- No `#[trusted]`, `#[trusted_impl]`, `#[extern_spec]`, `#[ignore]`, `#[no_panic]`, or `#[no_panic_if]` markers were added.
- Active limitations are ledgered: single-file harness scope, source-correspondence boundary, HashSet membership abstraction, and crate-mode Flux ICE for full crate verification.

## Residual limitations / next review state

- This closes only the Flux r9 sublane evidence for review. It does not claim full State 5 closure.
- `ActionReplayTracker` production `HashSet` insertion/contains behavior remains outside Flux r9.
- Full hydrate/replay behavior over `JournalEvent` slices, snapshot decoding, vector outputs, and replay loops remains outside Flux r9.
- Crate-mode Flux metadata is not kept because enabling it currently fails with a Flux ICE outside the recovery proof surface.
- Next state: proof-reviewer should review the Flux r9 sublane artifacts and limitations; non-Flux lanes remain separate.
