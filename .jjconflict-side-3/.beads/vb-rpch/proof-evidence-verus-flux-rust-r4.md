# Proof Evidence — vb-rpch verus-flux-rust-r4

## VFR-R2-VERUS-007 repair

Artifact: `verification/verus/vb_rpch_replay_events.rs`

Change: removed the rejected stale-attempt `requires forall ... ==> !state_effect` conclusion premise. The model now proves stale attempts are replay no-ops by construction: `replay_step` returns the prior `ReplayState` whenever `production_replay_attempt_is_stale(...)` is true, including stale events whose `state_effect` flag is true. The sequence lemma requires only that remaining events are stale, not that they lack state effects.

## Verus smoke

Command:

```sh
verus "verification/verus/vb_rpch_unsupported_recovery_state.rs" --crate-type lib && verus "verification/verus/vb_rpch_seed_dimensions.rs" --crate-type lib && verus "verification/verus/vb_rpch_action_replay_tracker.rs" --crate-type lib && verus "verification/verus/vb_rpch_digest_check.rs" --crate-type lib && verus "verification/verus/vb_rpch_hydrate_snapshot_tail.rs" --crate-type lib && verus "verification/verus/vb_rpch_hydrate_events.rs" --crate-type lib && verus "verification/verus/vb_rpch_replay_events.rs" --crate-type lib && verus "verification/verus/vb_rpch_production_bridge.rs" --crate-type lib
```

Result: exit 0.

Output summary:

```text
verification results:: 4 verified, 0 errors
verification results:: 4 verified, 0 errors
verification results:: 5 verified, 0 errors
verification results:: 2 verified, 0 errors
verification results:: 3 verified, 0 errors
verification results:: 3 verified, 0 errors
verification results:: 9 verified, 0 errors
verification results:: 1 verified, 0 errors
```

Verus also emitted automatic trigger notes for quantified replay predicates in `vb_rpch_replay_events.rs`; no verification error was emitted.

## Trust-marker scan

Command:

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

Result: exit 0.

Output:

```text
verification/verus/vb_rpch_production_bridge.rs:11:/// the per-obligation ghost models.  The trusted part is limited to the source
verification/verus/vb_rpch_production_bridge.rs:12:/// reference correspondence ledgered in trusted-base-ledger.verus-flux-rust-r3.jsonl.
```

Classification: comment-only trusted wording in the production-symbol bridge; executable trust markers were not found in scanned `verification/verus/vb_rpch_*.rs` files.

## Requires / conclusion-encoding scan

Command:

```sh
python3 - <<'PY'
from pathlib import Path
import re
for p in sorted(Path('verification/verus').glob('vb_rpch_*.rs')):
    for i,l in enumerate(p.read_text().splitlines(),1):
        if re.search(r'\brequires\b|\bensures\b', l):
            print(f'{p}:{i}:{l.strip()}')
PY
python3 - <<'PY'
from pathlib import Path
needle='old_attempts_have_no_state_effect'
for p in sorted(Path('verification/verus').glob('vb_rpch_*.rs')):
    for i,l in enumerate(p.read_text().splitlines(),1):
        if needle in l:
            print(f'{p}:{i}:{l.strip()}')
PY
```

Result: exit 0.

Relevant `VFR-R2-VERUS-007` output:

```text
verification/verus/vb_rpch_replay_events.rs:102:requires production_replay_attempt_is_stale(event.has_attempt, event.attempt, max_attempt),
verification/verus/vb_rpch_replay_events.rs:103:ensures replay_step(state, event, max_attempt) == state,
verification/verus/vb_rpch_replay_events.rs:107:requires
verification/verus/vb_rpch_replay_events.rs:110:ensures replay_from(events, max_attempt, state, index) == state,
verification/verus/vb_rpch_replay_events.rs:115:requires forall|i: int| 0 <= i < events.len() ==> production_replay_attempt_is_stale(events[i].has_attempt, events[i].attempt, max_attempt),
verification/verus/vb_rpch_replay_events.rs:116:ensures replay_from(events, max_attempt, state, 0) == state,
```

The `old_attempts_have_no_state_effect` scan produced no output. The remaining VFR-R2-VERUS-007 preconditions describe stale input cases only; they do not require `!state_effect` and do not assert the no-op conclusion as a premise.

## Flux disposition

Prior discovery command: `cargo flux --version`.

Prior output:

```text
error: no such command: `flux`
help: a command with a similar name exists: `fix`
```

Disposition: `BLOCKED_TOOLING` for `VFR-R2-FLUX-001..007`. Owner/rerun: tooling owner installs Flux, then rerun State 5/appropriate Flux sublane. No Flux pass claimed.

## Kani disposition

Prior discovery command: `cargo kani --version; cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani`.

Prior output excerpt:

```text
cargo-kani 0.67.0
error[E0277]: the trait bound `vb_core::RuntimePolicy: kani::Arbitrary` is not satisfied
error[E0277]: the trait bound `journal::core::FjallJournal: kani::Arbitrary` is not satisfied
error: Failed to execute cargo (exit status: 101). Found 5 compilation errors.
```

Disposition: `BLOCKED_GLOBAL_COMPILE` for `VFR-R2-KANI-001..007`. Owner/rerun: Kani harness/State 11 owner repairs cfg(kani) admission or harness construction, then rerun State 5 Kani sublane. No Kani pass claimed.

## Proptest disposition

Prior command: `PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_unsupported_recovery_state_union --exact --nocapture`.

Prior output:

```text
error: no test target named `recovery_property_tests` in `vb_storage` package
```

Disposition: `BLOCKED_MISSING_ARTIFACT` for `VFR-R2-PROPTEST-001..007`. Owner/rerun: property-test owner creates/renames the planned test target, then rerun State 5 proptest sublane. No proptest pass claimed.

## Fuzz disposition

Prior commands: `cargo fuzz --version; cargo fuzz list; cargo fuzz run vb_rpch_seed_dimensions_fuzz -- -runs=1 -max_len=16`.

Prior output excerpt:

```text
cargo-fuzz 0.13.1
error: no bin target named `vb_rpch_seed_dimensions_fuzz` in default-run packages
```

Disposition: `BLOCKED_MISSING_ARTIFACT` for `VFR-R2-FUZZ-001..004`. Owner/rerun: fuzz-target owner creates/renames the planned fuzz target, then rerun State 5 fuzz sublane. No fuzz pass claimed.

## Rust attachment planned-command blocker

Prior command: `rtk cargo check -p vb_storage --features verus`.

Prior output:

```text
error: the package 'vb_storage' does not contain this feature: verus
```

Disposition: `BLOCKED_PLAN_COMMAND` for `VFR-R2-RUST-ATTACH-001..007`. Owner/rerun: proof planner / State 11 command owner must replace the invalid command or add a justified feature gate, then rerun from State 4/11 as appropriate. No Rust attachment pass claimed.

## Provenance ledger

`.beads/vb-rpch/agent-invocation-ledger.jsonl` was absent in R3 review. R4 creates it with the current proof-writer invocation only. Prior proof planning/writing/review invocations were not reconstructed and are recorded as absent/unknown rather than fabricated.
