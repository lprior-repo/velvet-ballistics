# Proof Review — vb-rpch Flux r9

reviewer_skill: proof-reviewer invoked  
flux_skill: flux-rs invoked  
bead: `vb-rpch`  
state: 6 proof review — Flux RS r9  
workdir: `/home/lewis/src/vb-jpq7-jj-fix`  
date: 2026-05-24  
reviewer_scope: Flux sublane only; no approval is claimed for full State 5/6 closure or full production-body Flux verification.

## Findings first

No new Flux-r9 rejection findings.

Residual non-Flux blockers remain preserved and outside this approval:

- `VFR-R2-KANI-005..007`: prior Kani timeout/blocker dispositions remain open; r9 Flux evidence does not close Kani.
- `VFR-R2-RUST-ATTACH-001..007`: prior invalid planned Rust attachment command blocker remains unless separately repaired by the owning state.
- Provenance: `.beads/vb-rpch/agent-invocation-ledger.jsonl` still contains only the earlier `p5-proof-write-r4-verus007-blockers` row. I did not find a Flux-r9 proof-writer row in the ledger. This is preserved as a final/full-gate provenance limitation, not as a rejection of the rerun Flux command evidence.

## Flux reference files used

- `/home/lewis/.opencode/skill/flux-rs/references/flux-harness.md`
- `/home/lewis/.opencode/skill/flux-rs/references/flux-practice.md`
- `/home/lewis/.opencode/skill/flux-rs/references/flux-patterns.md`
- `/home/lewis/.opencode/skill/flux-rs/references/flux-deep-guide.md`

## Inputs reviewed

- `verification/flux/vb_rpch_flux_r9.rs`
- `.beads/vb-rpch/proof-writer-report-flux-r9.md`
- `.beads/vb-rpch/proof-evidence-flux-r9.md`
- `.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl`
- `.beads/vb-rpch/proof-obligations.flux-r9.written.jsonl`
- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r2.planned.jsonl`
- `.beads/vb-rpch/implementation-verus-flux-rust-r2.md`
- `.beads/vb-rpch/rust-refinement-obligations.verus-flux-rust-r2.jsonl`
- Mapped production refs in `crates/vb_storage/src/recovery/{types.rs,hydrate.rs,replay/core.rs,replay/summary.rs}`
- `.beads/vb-rpch/agent-invocation-ledger.jsonl`

## Commands rerun by reviewer

Working directory for all commands: `/home/lewis/src/vb-jpq7-jj-fix`.

| Command | Observed result |
| --- | --- |
| `z3 --version` | exit 0; `Z3 version 4.16.0 - 64 bit` |
| `if command -v fixpoint >/dev/null; then fixpoint --version; else liquid-fixpoint --version; fi` | exit 0; `fixpoint 0.9.6.3.6 (6f214fd7a67c1e61f3f165569b88dfdec2dda0d9)` |
| `flux --version` | exit 0; `flux 4d329f2 (2026-05-23)` |
| `cargo flux -V` | exit 0; `cargo-flux 4d329f2 (2026-05-23)` |
| `flux --crate-type lib --edition 2024 verification/flux/vb_rpch_flux_r9.rs` | exit 0; `summary. 50 functions processed: 50 checked; 0 trusted; 0 ignored. 38 constraints solved. Finished in 241.52ms` |
| `/usr/bin/rg -n '#!?\[(flux_rs::\|flux::)?(trusted\|trusted_impl\|extern_spec\|ignore\|no_panic\|no_panic_if)(\([^]]*\))?\]\|unsafe' --glob '*.rs' --glob '!**/target/**' verification/flux crates/vb_storage/src/recovery/types.rs crates/vb_storage/src/recovery/hydrate.rs crates/vb_storage/src/recovery/replay/core.rs crates/vb_storage/src/recovery/replay/summary.rs` | exit 0; only `#![forbid(unsafe_code)]` matches in mapped scope and r8/r9 Flux harnesses |

## Trust / skip marker review

The verified r9 harness has `#![forbid(unsafe_code)]` and no `#[trusted]`, `#[trusted_impl]`, `#[extern_spec]`, `#[ignore]`, broad skip, `unsafe`, `#[no_panic]`, or `#[no_panic_if]` marker in verified scope. The Flux command independently reported `0 trusted; 0 ignored`.

## Non-vacuity and overclaim review

- The r9 source includes negative `#[should_fail]` checks for supported-state mismatch, union mismatch, source-only-not-full digest, unresolved new tracker, zero dimensions, absent-positive count, missing snapshot evidence, empty events, state-effect-without-stale, and increasing step-order non-divergence.
- The prior `ActionReplayTracker` weakness is materially improved at harness scope: `mark_completed_surface` and `mark_failed_surface` return refined post-state values, and resolution is checked by calling `is_resolved_surface` on those returned values rather than proving a standalone `A || B` shortcut.
- Boolean disjunctions/conjunctions are still used because the production proof surfaces are boolean predicates, but the r9 signatures bind them to refined fields and negative checks exercise the opposite cases. I did not find an unsupported broad `A || B` proof shortcut in the r9 harness.
- The harness does not mechanically link to production bodies. This is honestly ledgered by `TB-R9-FLUX-SCOPED-SINGLE-FILE-HARNESS`, `TB-R9-FLUX-PRODUCTION-CORRESPONDENCE`, `TB-R9-FLUX-HASHSET-MEMBERSHIP-ABSTRACTION`, and `TB-R9-FLUX-CRATE-MODE-ICE`. Approval below is therefore scoped to the single-file harness only.

## Obligation disposition at scoped single-file Flux level

| Obligation | Review disposition |
| --- | --- |
| `VFR-R2-FLUX-001` | Accepted at scoped Flux harness level for all-false supported state, field-wise union OR, helper correspondence surface, and two negative checks. |
| `VFR-R2-FLUX-002` | Accepted at scoped Flux harness level for bounded `u16` `+1` positive count, absent-zero count, observed-dimension predicate, positive seed dimensions, and negative checks. |
| `VFR-R2-FLUX-003` | Accepted as partial/scoped public membership surface only. HashSet insertion/contains semantics remain outside Flux r9 and ledgered. |
| `VFR-R2-FLUX-004` | Accepted at scoped Flux harness level for strict digest rank hierarchy and check-level predicates, including source-only-not-full negative. |
| `VFR-R2-FLUX-005` | Accepted at pure precondition-surface scope only for snapshot-tail conjunction and positive dimensions. Snapshot byte decodability, event iteration, and full hydration behavior remain outside Flux r9. |
| `VFR-R2-FLUX-006` | Accepted at precondition-surface scope for non-empty events with negative empty-events check. |
| `VFR-R2-FLUX-007` | Accepted as partial/scoped pure replay surface only for stale/current attempt, stale-state-effect, and step-order-divergence predicates. Full replay loop effects, output vector preservation, `JournalEvent` matching coverage, and tracker HashSet effects remain outside Flux r9. |

## Limits of this approval

This review approves only the Flux-r9 sublane evidence for `verification/flux/vb_rpch_flux_r9.rs` under the exact single-file command above. It does not approve full crate-mode Flux verification, production-body Flux verification, Kani, Rust attachment, proptest/fuzz, or final provenance closure.

Next state: proceed to the next femdation/go-skill lane that owns the remaining non-Flux blockers, or move the Flux sublane forward with the above scope limitations recorded.

STATUS: APPROVED
