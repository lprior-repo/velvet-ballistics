# Proof Evidence — vb-rpch verus-flux-rust-r2

## Verus smoke

Command run in `/home/lewis/src/vb-jpq7-jj-fix`:

```sh
verus verification/verus/vb_rpch_unsupported_recovery_state.rs --crate-type lib && verus verification/verus/vb_rpch_seed_dimensions.rs --crate-type lib && verus verification/verus/vb_rpch_action_replay_tracker.rs --crate-type lib && verus verification/verus/vb_rpch_digest_check.rs --crate-type lib && verus verification/verus/vb_rpch_hydrate_snapshot_tail.rs --crate-type lib && verus verification/verus/vb_rpch_hydrate_events.rs --crate-type lib && verus verification/verus/vb_rpch_replay_events.rs --crate-type lib
```

Exit status: 0.

Excerpt:

```text
verification results:: 4 verified, 0 errors
verification results:: 3 verified, 0 errors
verification results:: 4 verified, 0 errors
verification results:: 2 verified, 0 errors
verification results:: 2 verified, 0 errors
verification results:: 3 verified, 0 errors
verification results:: 4 verified, 0 errors
```

The replay-events file emitted Verus auto-trigger notes for quantifiers; they are not verification errors.

## Flux discovery

Command run in `/home/lewis/src/vb-jpq7-jj-fix`:

```sh
cargo flux --version
```

Exit status: non-zero.

Excerpt:

```text
error: no such command: `flux`
help: a command with a similar name exists: `fix`
```

Flux obligations remain `BLOCKED_TOOLING`; no Flux proof pass is claimed.

## Kani blockers

Discovery in `/home/lewis/src/vb-jpq7-jj-fix`:

```sh
cargo kani --version
```

Exit status: 0, excerpt: `cargo-kani 0.67.0`.

Accepted planned command smoke:

```sh
cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani --no-unwind
```

Exit status: non-zero.

Excerpt:

```text
error: unexpected argument '--no-unwind' found
tip: a similar argument exists: '--no-unwinding-checks'
```

Harness availability smoke without the unsupported flag:

```sh
cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani
```

Exit status: non-zero.

Excerpt:

```text
error[E0432]: unresolved import `crate::recovery::replay::summary::recover_runtime_summary_from_events`
error[E0277]: the trait bound `types::EventSeq: kani::Arbitrary` is not satisfied
error[E0063]: missing field `seq` in initializer of `events::JournalEvent`
error[E0308]: mismatched types ... expected `types::EventSeq`, found `vb_core::EventSeq`
error: could not compile `vb_storage` (lib) due to 36 previous errors; 6 warnings emitted
```

State 5 did not edit `crates/vb_storage/src/**`. Kani lane is blocked on production `cfg(kani)` harness repair/wiring by State 11/Holzman.

## TLC preservation

Command run in `/home/lewis/src/vb-jpq7-jj-fix`:

```sh
python3 - <<'PY'
from pathlib import Path
for p in ['.beads/vb-rpch/proof-review-tlc-fix-round3.md','.beads/vb-rpch/formal-verification-report-tlc-fix-round3.md']:
    s=Path(p).read_text()
    assert 'STATUS: APPROVED' in s or 'Model checking completed. No error has been found.' in s
print('TLC_R3_EVIDENCE_PRESENT_SCOPE_TLA_ONLY')
PY
```

Exit status: 0.

Excerpt: `TLC_R3_EVIDENCE_PRESENT_SCOPE_TLA_ONLY`.

## Rust attachment blockers

`VFR-R2-RUST-ATTACH-001` through `VFR-R2-RUST-ATTACH-007` are owner_state 11 production proof-surface obligations. State 5 wrote standalone verification artifacts only and did not mutate production Rust.

## Proptest fuzz blockers

Planned property/fuzz artifacts are absent:

- `crates/vb_storage/tests/recovery_property_tests.rs`
- `fuzz/fuzz_targets/vb_rpch_seed_dimensions_fuzz.rs`
- `fuzz/fuzz_targets/vb_rpch_hydrate_snapshot_tail_fuzz.rs`
- `fuzz/fuzz_targets/vb_rpch_hydrate_events_fuzz.rs`
- `fuzz/fuzz_targets/vb_rpch_replay_events_fuzz.rs`

State 5 did not create broad/vacuous generators. These lanes require production recovery generator/oracle support or explicit test-writer ownership before exact planned commands can be meaningful.
