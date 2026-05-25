# Proof Evidence — vb-rpch verus-flux-rust-r3

## Verus smoke

Command:

```sh
verus verification/verus/vb_rpch_unsupported_recovery_state.rs --crate-type lib && verus verification/verus/vb_rpch_seed_dimensions.rs --crate-type lib && verus verification/verus/vb_rpch_action_replay_tracker.rs --crate-type lib && verus verification/verus/vb_rpch_digest_check.rs --crate-type lib && verus verification/verus/vb_rpch_hydrate_snapshot_tail.rs --crate-type lib && verus verification/verus/vb_rpch_hydrate_events.rs --crate-type lib && verus verification/verus/vb_rpch_replay_events.rs --crate-type lib && verus verification/verus/vb_rpch_production_bridge.rs --crate-type lib
```

Result: exit 0.

Output:

```text
verification results:: 4 verified, 0 errors
verification results:: 4 verified, 0 errors
verification results:: 5 verified, 0 errors
verification results:: 2 verified, 0 errors
verification results:: 3 verified, 0 errors
verification results:: 3 verified, 0 errors
note: automatically chose triggers for quantified replay predicates in vb_rpch_replay_events.rs
verification results:: 6 verified, 0 errors
verification results:: 1 verified, 0 errors
```

## Trust-marker scan

Pattern:

```text
\b(assume|admit|external_body|trusted|axiom|opaque|reveal_with_fuel|unimplemented|todo|panic|unwrap|expect)\b
```

Scope: `verification/verus/vb_rpch_*.rs`

Result: only comment text in `verification/verus/vb_rpch_production_bridge.rs:11-12` containing `trusted`; no executable trust markers found.

## Conclusion-encoding scan

Pattern:

```text
requires\s+(successful_seed_dimensions|valid_hydrate_snapshot_tail_preconditions|valid_hydrate_events_preconditions|old_attempts_have_no_state_effect|is_resolved|production_is_resolved)
```

Result: two remaining `production_is_resolved` preconditions in `vb_rpch_action_replay_tracker.rs` monotonicity lemmas. These are not conclusion-encoding constructor lemmas; they prove that a previously resolved key remains resolved after inserting another key.

## Production bridge source refs

Read evidence:

- `crates/vb_storage/src/recovery/types.rs:275-297` — `UnsupportedRecoveryState::union`, `is_fully_supported`, `union_matches_flags`.
- `crates/vb_storage/src/recovery/types.rs:367-383` — `ActionReplayTracker::{has_completed,has_failed,is_resolved}`.
- `crates/vb_storage/src/recovery/types.rs:405-437` — `DigestCheck` rank/check/strict weaker predicates.
- `crates/vb_storage/src/recovery/hydrate.rs:18-68` — hydrate snapshot-tail/events/dimension proof surfaces.
- `crates/vb_storage/src/recovery/replay/core.rs:27-74` — replay attempt, state-effect, stale-state-effect, and step-order proof surfaces.
- `crates/vb_storage/src/recovery/replay/summary.rs:481-508` — seed dimension proof surfaces.

## Flux discovery

Command:

```sh
cargo flux --version
```

Result: non-zero.

Output:

```text
error: no such command: `flux`
help: a command with a similar name exists: `fix`
```

Classification: `BLOCKED_TOOLING` for `VFR-R2-FLUX-001..007`. No Flux pass claimed.

## Kani discovery/blocker

Command:

```sh
cargo kani --version
cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani
```

Result: version command passed; harness command failed before verification.

Key output:

```text
cargo-kani 0.67.0
error[E0277]: the trait bound `vb_core::RuntimePolicy: kani::Arbitrary` is not satisfied
  --> crates/vb_storage/src/kani_admission.rs:86:30
error[E0277]: the trait bound `journal::core::FjallJournal: kani::Arbitrary` is not satisfied
  --> crates/vb_storage/src/kani_admission.rs:92:42
error: could not compile `vb_storage` (lib) due to 5 previous errors; 5 warnings emitted
error: Failed to execute cargo (exit status: 101). Found 5 compilation errors.
```

Classification: `BLOCK_GLOBAL` for `VFR-R2-KANI-001..007`. No Kani pass claimed.

## Proptest blocker

Command:

```sh
PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_unsupported_recovery_state_union --exact --nocapture
```

Result: non-zero.

Output:

```text
error: no test target named `recovery_property_tests` in `vb_storage` package
help: available test targets:
    accepted_artifact_red_phase
    manual_qa_smoke
    vb_core_atomic_admission_red
```

Classification: `BLOCKED_SCOPE` for `VFR-R2-PROPTEST-001..007`.

## Fuzz blocker

Commands:

```sh
cargo fuzz --version
cargo fuzz list
cargo fuzz run vb_rpch_seed_dimensions_fuzz -- -runs=1 -max_len=16
```

Result: `cargo-fuzz 0.13.1` is available; requested target is absent.

Output excerpt:

```text
cargo-fuzz 0.13.1
capability_contract_schema
capability_name_schema
collect_page
compiled_ir
expression
generated_compare
ipc_frame
journal_event
vb_f04l_yaml_compiler_compile
vb_qi37_12_persisted_payload_decode
yaml_events
error: no bin target named `vb_rpch_seed_dimensions_fuzz` in default-run packages
```

Classification: `BLOCKED_SCOPE` for `VFR-R2-FUZZ-001..004`.

## Rust attachment planned-command blocker

Command:

```sh
rtk cargo check -p vb_storage --features verus
```

Result: non-zero.

Output:

```text
error: the package 'vb_storage' does not contain this feature: verus
help: packages with the missing feature: vb_validate, velvet-ballastics-workspace-tests
```

Classification: `BLOCKED_PLAN_COMMAND` for `VFR-R2-RUST-ATTACH-001..007`; State 11 reports production surfaces exist and all-features checks passed, but this planned command is invalid.

## TLC preservation

Command:

```sh
python3 - <<'PY'
from pathlib import Path
for p in ['.beads/vb-rpch/proof-review-tlc-fix-round3.md','.beads/vb-rpch/formal-verification-report-tlc-fix-round3.md']:
    s=Path(p).read_text()
    assert 'STATUS: APPROVED' in s or 'Model checking completed. No error has been found.' in s
print('TLC_R3_EVIDENCE_PRESENT_SCOPE_TLA_ONLY')
PY
```

Result: exit 0.

Output:

```text
TLC_R3_EVIDENCE_PRESENT_SCOPE_TLA_ONLY
```

Scope statement: bounded TLA/TLC abstraction only; no Rust/Flux/Kani conclusion inferred.
