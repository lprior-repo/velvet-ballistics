# Proof Review — vb-rpch verus-flux-rust-r3

reviewer_skill: proof-reviewer
reviewer_invocation_id: p6-proof-review-r3-after-holzman
proof_writer_task: ses_1a32f7a60ffen926d5iDTCyXOI
bead: vb-rpch
state: 6 proof review
workdir: /home/lewis/src/vb-jpq7-jj-fix
date: 2026-05-24

## Disposition

REJECTED for full proof gate. The R2 critical disconnect is materially improved: State-11 production helper surfaces exist, the R3 Verus artifacts name them, source refs match the read production Rust, and the source-correspondence trust boundary is ledgered in `trusted-base-ledger.verus-flux-rust-r3.jsonl`. However the Verus sublane is not fully approved because `VFR-R2-VERUS-007` still contains a conclusion-encoding wrapper lemma: it requires the exact quantified stale-attempt no-state-effect property and only ensures the named predicate whose definition is that same property.

Flux, Kani, proptest, fuzz, and the planned Rust attachment command remain honest blockers. No full State 5 proof closure may be claimed.

## Commands run

1. Verus smoke rerun:

```sh
verus verification/verus/vb_rpch_unsupported_recovery_state.rs --crate-type lib && verus verification/verus/vb_rpch_seed_dimensions.rs --crate-type lib && verus verification/verus/vb_rpch_action_replay_tracker.rs --crate-type lib && verus verification/verus/vb_rpch_digest_check.rs --crate-type lib && verus verification/verus/vb_rpch_hydrate_snapshot_tail.rs --crate-type lib && verus verification/verus/vb_rpch_hydrate_events.rs --crate-type lib && verus verification/verus/vb_rpch_replay_events.rs --crate-type lib && verus verification/verus/vb_rpch_production_bridge.rs --crate-type lib
```

Result: exit 0. Output: `4/4/5/2/3/3/6/1 verified, 0 errors`; replay events emitted Verus automatic trigger notes for quantified predicates.

2. Trust-marker scan:

```sh
python3 - <<'PY'
from pathlib import Path
import re
root=Path('verification/verus')
pat=re.compile(r'\b(assume|admit|external_body|trusted|axiom|opaque|reveal_with_fuel|unimplemented|todo|panic|unwrap|expect)\b')
for p in sorted(root.glob('vb_rpch_*.rs')):
    for i,line in enumerate(p.read_text().splitlines(),1):
        if pat.search(line):
            print(f'{p}:{i}:{line}')
PY
```

Result: only `trusted` in comments at `verification/verus/vb_rpch_production_bridge.rs:11-12`, ledgered by `TB-VFR-R3-VERUS-PRODUCTION-SYMBOL-BRIDGE`. No executable `assume`, `admit`, `external_body`, `trusted`, or `axiom` marker found in the scanned `vb_rpch_*.rs` files.

3. Requires/ensures scan:

```sh
python3 - <<'PY'
from pathlib import Path
import re
for p in sorted(Path('verification/verus').glob('vb_rpch_*.rs')):
    for i,l in enumerate(p.read_text().splitlines(),1):
        if re.search(r'\brequires\b|\bensures\b', l):
            print(f'{p}:{i}:{l.strip()}')
PY
```

Result: R2 direct named precondition restatements are mostly gone, but `verification/verus/vb_rpch_replay_events.rs:67-70` still requires the definition of `old_attempts_have_no_state_effect` and ensures only that wrapper predicate.

4. Production source refs read and compared:

- `crates/vb_storage/src/recovery/types.rs:275-297`, `367-383`, `405-437`
- `crates/vb_storage/src/recovery/hydrate.rs:18-68`
- `crates/vb_storage/src/recovery/replay/core.rs:27-74`
- `crates/vb_storage/src/recovery/replay/summary.rs:481-508`

Result: R3 Verus model definitions match the named helper surfaces at the inspected source refs, subject to the active trusted source-correspondence boundary.

5. Blocker discovery rerun:

```sh
cargo flux --version; cargo kani --version; cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani; PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_unsupported_recovery_state_union --exact --nocapture; cargo fuzz --version; cargo fuzz list; cargo fuzz run vb_rpch_seed_dimensions_fuzz -- -runs=1 -max_len=16
```

Result: Flux unavailable (`no such command: flux`); Kani installed (`cargo-kani 0.67.0`) but `vb_storage` fails before verification on unrelated `kani_admission.rs` `RuntimePolicy`/`FjallJournal: kani::Arbitrary` errors; proptest target `recovery_property_tests` absent; cargo-fuzz installed (`0.13.1`) but requested `vb_rpch_seed_dimensions_fuzz` target absent.

6. Rust attachment planned command rerun:

```sh
rtk cargo check -p vb_storage --features verus
```

Result: non-zero, `the package 'vb_storage' does not contain this feature: verus`.

7. Provenance ledger check:

Result: `.beads/vb-rpch/agent-invocation-ledger.jsonl` is absent, so reviewer provenance cannot be independently validated from the required ledger.

## Findings

### HIGH — VFR-R2-VERUS-007 still encodes a conclusion in `requires`

Artifact: `verification/verus/vb_rpch_replay_events.rs:51-70`

Obligation: `VFR-R2-VERUS-007`

Evidence: `old_attempts_have_no_state_effect(events, max_attempt)` is defined as the quantified property at lines 51-53. `proof_old_attempt_filter_preserved` then requires the same quantified property at line 68 and ensures only the wrapper predicate at line 69. This is still a definitional restatement, not a derivation from replay transition/filter execution.

Required fix: prove the stale-attempt no-state-effect property from a modeled replay transition/filter step or split the claim into a lower-strength predicate-equivalence lemma plus a separate transition lemma that derives the premise from production replay filtering.

### HIGH — Required non-Verus lanes remain blockers, not proof passes

Artifacts: `.beads/vb-rpch/proof-evidence-verus-flux-rust-r3.md`, `.beads/vb-rpch/proof-obligations.verus-flux-rust-r3.written.jsonl`, `.beads/vb-rpch/trusted-base-ledger.verus-flux-rust-r3.jsonl`

Obligations: `VFR-R2-FLUX-001..007`, `VFR-R2-KANI-001..007`, `VFR-R2-PROPTEST-001..007`, `VFR-R2-FUZZ-001..004`, `VFR-R2-RUST-ATTACH-001..007`

Evidence: rerun commands reproduced the Flux tooling absence, Kani global compile blocker, absent proptest target, absent fuzz target, and invalid `vb_storage --features verus` planned attachment command.

Required fix: repair or formally waive these lanes before full State 5 proof closure. Do not advance them as passed.

### MEDIUM — Required invocation provenance ledger is still absent

Artifact: `.beads/vb-rpch/agent-invocation-ledger.jsonl`

Obligation: review provenance / self-approval guard

Evidence: file not found during review.

Required fix: add ledger rows for proof planning/writing/reviewing with distinct invocations before any final proof-gate approval.

## Accepted evidence / non-findings

- R2 critical bridge disconnect is improved: `vb_rpch_production_bridge.rs` and comments in the seven Verus files now identify State-11 production surfaces, and inspected production helpers match the ghost model definitions at the cited source refs.
- Trust boundary is explicit: `TB-VFR-R3-VERUS-PRODUCTION-SYMBOL-BRIDGE` classifies the production-symbol correspondence as trusted source correspondence. No hidden executable trust marker was found in the scanned Verus artifacts.
- The old named conclusion-encoding patterns for seed dimensions, hydrate snapshot-tail, hydrate events, and tracker monotonicity are either removed or reduced to legitimate aggregate/monotonicity preconditions. The remaining Verus blocker is `VFR-R2-VERUS-007`.
- TLC round-3 preservation remains bounded TLA/TLC-only evidence and is not overclaimed as Rust/Flux/Kani proof.

## Approved sublanes

- `VFR-R2-VERUS-001` through `VFR-R2-VERUS-006`: approved as Verus smoke-passing ghost proofs with ledgered trusted source-correspondence bridge to State-11 production helper surfaces.
- `VFR-R2-TLA-PRESERVE-001`, `VFR-R2-TLA-PRESERVE-002`: preserved only at prior bounded TLA/TLC abstraction scope.

## Blocking lanes / obligations

- Verus: `VFR-R2-VERUS-007` blocked by conclusion-encoding restatement.
- Flux: `VFR-R2-FLUX-001..007` blocked by missing cargo subcommand.
- Kani: `VFR-R2-KANI-001..007` blocked by global `kani_admission.rs` compile errors.
- Proptest: `VFR-R2-PROPTEST-001..007` blocked by absent test target.
- Fuzz: `VFR-R2-FUZZ-001..004` blocked by absent fuzz target.
- Rust attachment command: `VFR-R2-RUST-ATTACH-001..007` blocked by stale/invalid `--features verus` command.
- Provenance: invocation ledger absent.

## Nearest rerun state

Rerun State 5 proof-writer for `VFR-R2-VERUS-007` plus blocker disposition repair. If only repairing the stale Rust attachment command/provenance ledger, rerun State 6 afterward; full proof gate remains blocked until Flux/Kani/proptest/fuzz are repaired or explicitly approved as waivers/blockers by the owning lane.

STATUS: REJECTED
