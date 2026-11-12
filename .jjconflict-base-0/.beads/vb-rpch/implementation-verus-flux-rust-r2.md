# Implementation — vb-rpch verus-flux-rust-r2 State 11 Repair

## Status

State 11 production proof-attachment repair completed. This does **not** approve proof artifacts; it exposes production Rust surfaces for State 5 proof-writer rerun.

## Why proof review rejected R2

Proof review classified the Verus files as disconnected mirror models because they defined local `Spec*` enums/predicates and proved algebra over those local definitions, while the planned obligations required binding to the State 11 production proof surface. Several lemmas also required the predicate they ensured, so they were restatement lemmas rather than derivations from production constructors/transitions.

## Source changes mapped to obligations

- `VFR-R2-RUST-ATTACH-001`: `crates/vb_storage/src/recovery/types.rs` now exposes `UnsupportedRecoveryState::is_fully_supported` and `union_matches_flags`; Kani harness name `unsupported_recovery_state_union_kani` added in `crates/vb_storage/src/kani_recovery_hydrate.rs`.
- `VFR-R2-RUST-ATTACH-002`: `crates/vb_storage/src/recovery/replay/summary.rs` now exposes checked dimension helpers `recovery_dimension_count_from_index`, `recovery_seed_dimensions_positive`, and `recovery_observed_dimension_is_positive`; Kani harness `recovery_frame_seed_dimensions_kani` added.
- `VFR-R2-RUST-ATTACH-003`: `ActionReplayTracker` now exposes `has_completed` and `has_failed` in addition to existing `is_resolved`; Kani harness `action_replay_tracker_monotonic_kani` added.
- `VFR-R2-RUST-ATTACH-004`: `DigestCheck` now exposes rank/check/hierarchy predicates; Kani harness `digest_check_hierarchy_kani` added.
- `VFR-R2-RUST-ATTACH-005`: `hydrate.rs` now exposes snapshot-tail run, sequence, evidence, aggregate precondition, and dimension predicates; Kani harness `hydrate_run_frame_precond_kani` added.
- `VFR-R2-RUST-ATTACH-006`: `hydrate.rs` now exposes events-only non-empty and dimension predicates; Kani harness `hydrate_run_frame_from_events_precond_kani` added.
- `VFR-R2-RUST-ATTACH-007`: `replay/core.rs` now exposes attempt default/current/stale, state-effect, stale-state-effect, and step-order divergence predicates. `replay_events` uses the stale and step-order helpers directly. Kani harness `replay_events_kani` added.

## Behavior change statement

Production behavior is intended to be unchanged except `replay_events` now calls equivalent helper predicates for stale attempt filtering and step-order divergence. No TLA files were touched.

## Flux disposition

`cargo flux --version` is blocked: `error: no such command: flux`. No Flux pass is claimed and no Flux annotations were added because tooling is absent.

## Kani disposition

Local recovery Kani harness wiring was repaired to avoid invalid `Vec<T>: Arbitrary`, missing `EventSeq: Arbitrary`, non-arbitrary timestamp/policy/capability, obsolete event shapes, and nonexistent `recover_runtime_summary_from_events`. Required vb-rpch harness names were added.

Kani still does not run because the crate compiles all `#[cfg(kani)]` modules and unrelated `crates/vb_storage/src/kani_admission.rs` fails on `RuntimePolicy: kani::Arbitrary` and `FjallJournal: kani::Arbitrary`. Classified `BLOCK_GLOBAL` for Kani, not proof pass.

## Next rerun

Rerun from State 5 proof-writer, then State 6 proof-review, after either fixing global `kani_admission` compile blockers or accepting a scoped Kani blocker disposition. The proof-writer should consume `.beads/vb-rpch/rust-refinement-obligations.verus-flux-rust-r2.jsonl`.
