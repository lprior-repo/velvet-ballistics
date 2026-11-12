# Proof Review — vb-rpch verus-flux-rust-r4

reviewer_skill: proof-reviewer
reviewer_invocation_id: p6-proof-review-r4-verus007
proof_writer_task: ses_1a326072bffeWiPRkpLvihtix6
bead: vb-rpch
state: 6 proof review
workdir: /home/lewis/src/vb-jpq7-jj-fix
date: 2026-05-24

## Findings

### HIGH — Required non-Verus lanes remain honest blockers, not proof passes

Artifacts:
- `.beads/vb-rpch/proof-evidence-verus-flux-rust-r4.md:100-163`
- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r4.written.jsonl:1-7,15-28,31-41`
- `.beads/vb-rpch/trusted-base-ledger.verus-flux-rust-r4.jsonl:3-7`

Obligations: `VFR-R2-FLUX-001..007`, `VFR-R2-KANI-001..007`, `VFR-R2-PROPTEST-001..007`, `VFR-R2-FUZZ-001..004`, `VFR-R2-RUST-ATTACH-001..007`

Raw evidence references:
- R4 evidence records Flux as `BLOCKED_TOOLING`: `cargo flux --version` returned `error: no such command: flux`.
- R4 evidence records Kani as `BLOCKED_GLOBAL_COMPILE`: `cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani` fails before verification in `crates/vb_storage/src/kani_admission.rs` because `vb_core::RuntimePolicy` and `journal::core::FjallJournal` do not implement `kani::Arbitrary`.
- R4 evidence records proptest as `BLOCKED_MISSING_ARTIFACT`: `recovery_property_tests` test target is absent.
- R4 evidence records fuzz as `BLOCKED_MISSING_ARTIFACT`: `vb_rpch_seed_dimensions_fuzz` fuzz target is absent.
- R4 evidence records Rust attachment as `BLOCKED_PLAN_COMMAND`: `rtk cargo check -p vb_storage --features verus` is invalid because `vb_storage` has no `verus` feature.
- Reviewer rerun reproduced all five blocker classes with the same classifications.

Disposition: blocker classification is honest. These are not proof passes and they still block full State 5 proof closure.

Required fix: repair or explicitly waive/block each owning lane under its appropriate state before claiming full proof-gate closure.

### MEDIUM — Invocation provenance remains partial

Artifact: `.beads/vb-rpch/agent-invocation-ledger.jsonl:1`

Obligation: review provenance / self-approval guard

Evidence: R3 review found the ledger absent. R4 now contains one proof-writer row for `p5-proof-write-r4-verus007-blockers`; it explicitly says prior invocations were not reconstructed. There is no row in the read ledger for this R4 reviewer invocation yet and no prior planning/review provenance rows.

Disposition: this does not invalidate the R4 Verus sublane repair by itself, but it remains a final proof-gate blocker.

Required fix: append truthful, non-fabricated invocation rows for current and prior proof-plan/write/review roles, or keep provenance as an explicit blocker for final State 5 closure.

## Accepted evidence / repaired finding

### VFR-R2-VERUS-007 no longer encodes its conclusion in `requires`

Artifact: `verification/verus/vb_rpch_replay_events.rs:60-119`

Obligation: `VFR-R2-VERUS-007`

Evidence:
- The rejected R3 predicate `old_attempts_have_no_state_effect` is absent from all scanned `verification/verus/vb_rpch_*.rs` files.
- `proof_stale_replay_step_is_noop` requires only `production_replay_attempt_is_stale(...)` and proves `replay_step(...) == state` from the `replay_step` definition, whose first branch returns `state` for stale attempts.
- `proof_all_stale_replay_prefix_is_noop` and `proof_stale_attempt_filter_preserved` require a stale-input condition over the sequence, not `!state_effect` and not the no-op conclusion itself.
- The model explicitly allows stale state-effecting variants: `production_replay_event_is_stale_state_effect(event, max_attempt)` remains `event.state_effect && stale(...)`; stale events are no-ops because replay filters them before state-effect branches.

Disposition: `PF-VFR-R3-001` is repaired. `VFR-R2-VERUS-007` is approved at standalone Verus ghost-model scope, subject to the active production-symbol correspondence trust boundary.

## Commands run

1. Full Verus smoke rerun:

```sh
verus "verification/verus/vb_rpch_unsupported_recovery_state.rs" --crate-type lib && verus "verification/verus/vb_rpch_seed_dimensions.rs" --crate-type lib && verus "verification/verus/vb_rpch_action_replay_tracker.rs" --crate-type lib && verus "verification/verus/vb_rpch_digest_check.rs" --crate-type lib && verus "verification/verus/vb_rpch_hydrate_snapshot_tail.rs" --crate-type lib && verus "verification/verus/vb_rpch_hydrate_events.rs" --crate-type lib && verus "verification/verus/vb_rpch_replay_events.rs" --crate-type lib && verus "verification/verus/vb_rpch_production_bridge.rs" --crate-type lib
```

Result: exit 0. Output summary: `4/4/5/2/3/3/9/1 verified, 0 errors`; Verus emitted automatic trigger notes in `vb_rpch_replay_events.rs` only.

2. Requires/conclusion/trust scan:

```sh
python3 - <<'PY'
from pathlib import Path
import re
print('REQUIRES_ENSURES')
for p in sorted(Path('verification/verus').glob('vb_rpch_*.rs')):
    for i,l in enumerate(p.read_text().splitlines(),1):
        if re.search(r'\brequires\b|\bensures\b', l):
            print(f'{p}:{i}:{l.strip()}')
print('OLD_ATTEMPTS_NEEDLE')
needle='old_attempts_have_no_state_effect'
for p in sorted(Path('verification/verus').glob('vb_rpch_*.rs')):
    for i,l in enumerate(p.read_text().splitlines(),1):
        if needle in l:
            print(f'{p}:{i}:{l.strip()}')
print('TRUST_MARKERS')
pat=re.compile(r'\b(assume|admit|external_body|trusted|axiom|opaque|reveal_with_fuel|unimplemented|todo|panic|unwrap|expect)\b')
for p in sorted(Path('verification/verus').glob('vb_rpch_*.rs')):
    for i,line in enumerate(p.read_text().splitlines(),1):
        if pat.search(line):
            print(f'{p}:{i}:{line}')
PY
```

Result: exit 0. `OLD_ATTEMPTS_NEEDLE` produced no matches. `TRUST_MARKERS` produced only comment-only `trusted` wording in `verification/verus/vb_rpch_production_bridge.rs:11-12`, with no executable trust marker in scanned files.

3. Blocker discovery rerun:

```sh
cargo flux --version; cargo kani --version; cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani; PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_unsupported_recovery_state_union --exact --nocapture; cargo fuzz --version; cargo fuzz list; cargo fuzz run vb_rpch_seed_dimensions_fuzz -- -runs=1 -max_len=16; rtk cargo check -p vb_storage --features verus
```

Result: non-zero sequence as expected for discovery. Flux unavailable; Kani `0.67.0` installed but `vb_storage` fails on five `kani_admission.rs` compile errors before harness verification; proptest target absent; cargo-fuzz `0.13.1` installed but requested fuzz target absent; Rust attachment command invalid because `vb_storage` has no `verus` feature.

## Approved sublanes

- `VFR-R2-VERUS-001` through `VFR-R2-VERUS-006`: approved as in R3, standalone Verus smoke-passing ghost proofs with ledgered production-symbol/source-correspondence trust boundary.
- `VFR-R2-VERUS-007`: approved in R4 at standalone Verus ghost-model scope; the R3 conclusion-encoding `requires` defect is repaired.
- `VFR-R2-TLA-PRESERVE-001`, `VFR-R2-TLA-PRESERVE-002`: unchanged, preserved only at prior bounded TLA/TLC abstraction scope.

## Blocking lanes / obligations

- Flux: `VFR-R2-FLUX-001..007` blocked by missing `cargo flux` tooling.
- Kani: `VFR-R2-KANI-001..007` blocked by global `kani_admission.rs` compile errors before harness verification.
- Proptest: `VFR-R2-PROPTEST-001..007` blocked by absent `recovery_property_tests` target.
- Fuzz: `VFR-R2-FUZZ-001..004` blocked by absent `vb_rpch_seed_dimensions_fuzz` target.
- Rust attachment: `VFR-R2-RUST-ATTACH-001..007` blocked by invalid planned `vb_storage --features verus` command.
- Provenance: invocation ledger remains partial/current-only and cannot validate prior role separation.

## Disposition

The Verus sublane `VFR-R2-VERUS-001..007` is approved at its stated standalone ghost-model scope. Full State 5 proof closure remains rejected because Flux, Kani, proptest, fuzz, Rust attachment, and final provenance remain blocked or partial.

## Next state

Return to the owning State 5 sublanes for Flux/Kani/proptest/fuzz repair or waiver, return to State 4/11 for the invalid Rust attachment command, and repair provenance before final proof-gate approval. No additional Verus repair is required for `VFR-R2-VERUS-007` unless production source-correspondence changes.

STATUS: REJECTED
