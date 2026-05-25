# Proof Writer Report — vb-rpch verus-flux-rust-r3

bead: vb-rpch
state: 5 proof/model/harness repair after State 11 Holzman implementation
workdir: /home/lewis/src/vb-jpq7-jj-fix
date: 2026-05-24

## Disposition

R3 State 5 proof artifacts were repaired for the Verus lane. The seven reviewed Verus artifacts now name and model the State-11 production proof surfaces added by Holzman Rust, and a dedicated `vb_rpch_production_bridge.rs` records the symbol-level bridge. Verus smoke passes for all touched `vb_rpch_*` artifacts.

This is **not** full State 5 closure: Flux remains `BLOCKED_TOOLING`; Kani remains `BLOCK_GLOBAL`; required proptest/fuzz targets are absent and remain `BLOCKED_SCOPE`. TLC round-3 evidence is preserved as bounded TLA/TLC abstraction only.

## Obligations touched

- `VFR-R2-VERUS-001` — repaired `verification/verus/vb_rpch_unsupported_recovery_state.rs` to bridge `UnsupportedRecoveryState::{SUPPORTED,is_fully_supported,union,union_matches_flags}`.
- `VFR-R2-VERUS-002` — repaired `verification/verus/vb_rpch_seed_dimensions.rs` to bridge `recovery_dimension_count_from_index`, `recovery_seed_dimensions_positive`, and `recovery_observed_dimension_is_positive`.
- `VFR-R2-VERUS-003` — repaired `verification/verus/vb_rpch_action_replay_tracker.rs` to bridge `ActionReplayTracker::{has_completed,has_failed,is_resolved,mark_completed,mark_failed}`.
- `VFR-R2-VERUS-004` — repaired `verification/verus/vb_rpch_digest_check.rs` to bridge `DigestCheck::{hierarchy_rank,checks_workflow_source,checks_compiled_ir,checks_full,is_strictly_weaker_than}`.
- `VFR-R2-VERUS-005` — repaired `verification/verus/vb_rpch_hydrate_snapshot_tail.rs` to bridge snapshot-tail run, sequence, evidence, aggregate precondition, and positive-dimension helpers.
- `VFR-R2-VERUS-006` — repaired `verification/verus/vb_rpch_hydrate_events.rs` to bridge events-only non-empty and positive-dimension helpers.
- `VFR-R2-VERUS-007` — repaired `verification/verus/vb_rpch_replay_events.rs` to bridge replay attempt default/current/stale, state-effect filtering, stale-state-effect, and step-order divergence helpers.
- `VFR-R2-TLA-PRESERVE-001`, `VFR-R2-TLA-PRESERVE-002` — preservation command rerun; scope remains bounded TLA/TLC only.

## Artifacts changed

- `verification/verus/vb_rpch_unsupported_recovery_state.rs`
- `verification/verus/vb_rpch_seed_dimensions.rs`
- `verification/verus/vb_rpch_action_replay_tracker.rs`
- `verification/verus/vb_rpch_digest_check.rs`
- `verification/verus/vb_rpch_hydrate_snapshot_tail.rs`
- `verification/verus/vb_rpch_hydrate_events.rs`
- `verification/verus/vb_rpch_replay_events.rs`
- `verification/verus/vb_rpch_production_bridge.rs`
- `.beads/vb-rpch/proof-writer-report-verus-flux-rust-r3.md`
- `.beads/vb-rpch/proof-evidence-verus-flux-rust-r3.md`
- `.beads/vb-rpch/trusted-base-ledger.verus-flux-rust-r3.jsonl`
- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r3.written.jsonl`

## Commands/results summary

- Verus smoke for all eight touched artifacts: PASS, exit 0; per-file counts `4/4/5/2/3/3/6/1 verified, 0 errors`.
- Trust-marker scan over `verification/verus/vb_rpch_*.rs`: only comment occurrences of `trusted` in the bridge file; no executable `assume/admit/external_body/trusted/axiom/opaque` markers found.
- Conclusion-encoding scan: old direct restatement patterns removed except legitimate monotonicity preconditions in `ActionReplayTracker`.
- `cargo flux --version`: blocked, no `cargo flux` subcommand.
- `cargo kani --version`: available, `cargo-kani 0.67.0`; harness run blocked by unrelated global `kani_admission.rs` compile errors.
- `PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests ...`: blocked; no such test target.
- `cargo fuzz list` plus requested target smoke: blocked; requested `vb_rpch_*` fuzz target not present.
- TLC preservation check: PASS, `TLC_R3_EVIDENCE_PRESENT_SCOPE_TLA_ONLY`.

## Blocker classifications

- `BLOCKED_TOOLING`: `VFR-R2-FLUX-001..007`; `cargo flux` unavailable. No Flux pass claimed.
- `BLOCK_GLOBAL`: `VFR-R2-KANI-001..007`; Kani installed, but `vb_storage` global `cfg(kani)` compilation fails in unrelated `kani_admission.rs` (`RuntimePolicy` and `FjallJournal` lack `kani::Arbitrary`). No Kani pass claimed.
- `BLOCKED_SCOPE`: `VFR-R2-PROPTEST-001..007`; planned `recovery_property_tests` target absent.
- `BLOCKED_SCOPE`: `VFR-R2-FUZZ-001..004`; planned `vb_rpch_*` fuzz targets absent.
- `BLOCKED_PLAN_COMMAND`: `VFR-R2-RUST-ATTACH-001..007`; planned `cargo check -p vb_storage --features verus` is invalid because `vb_storage` has no `verus` feature. State 11 did expose production surfaces and passed its own all-features checks, but the planned command itself is stale/invalid.

## Reviewer rerun readiness

Proof-reviewer may rerun for the Verus bridge repair. The rerun must treat Flux/Kani/proptest/fuzz as blockers, not proof passes, unless another owner repairs or waives them.

STATUS: VERUS_R3_REPAIRED_WITH_BLOCKERS
