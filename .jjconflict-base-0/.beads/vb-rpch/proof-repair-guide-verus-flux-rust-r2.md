# Proof Repair Guide — vb-rpch verus-flux-rust-r2

## Required repairs

1. Add or expose a production proof/equivalence bridge for the seven Verus obligations.
   - The bridge must connect `UnsupportedRecoveryState`, `ActionReplayTracker`, `DigestCheck`, `hydrate_run_frame`, `hydrate_run_frame_from_events`, and `replay_events` to the Verus spec predicates/types.
   - The bridge must be field/function precise; do not rely on comments saying the model mirrors production.
   - Rerun all seven `verus verification/verus/vb_rpch_*.rs --crate-type lib` commands and record raw output.

2. Strengthen Verus non-vacuity.
   - Avoid lemmas that `requires P(...)` and only `ensures` P's conjuncts.
   - Add negative/rejection lemmas for weakened predicates and mismatched production inputs.
   - For replay, prove transition-level facts: old attempts cannot create state effects, completed/failed events update resolution, resolved non-idempotent actions are blocked, and step-order divergence is detected from modeled replay transitions.

3. Repair required non-Verus lanes or route explicit waivers.
   - Kani: replace invalid `--no-unwind` with the correct cargo-kani 0.67.0 flag or update the plan, repair harness compile errors, and rerun exact harnesses.
   - Proptest/fuzz: create meaningful generator/oracle artifacts or obtain approved blocker disposition. Missing files are not proof evidence.
   - Flux: keep `BLOCKED_TOOLING` unless `cargo flux --version` and `cargo flux --package vb_storage` succeed; do not claim a Flux pass.

4. Restore review provenance.
   - Add `.beads/vb-rpch/agent-invocation-ledger.jsonl` entries sufficient to prove the proof writer did not approve its own artifacts.

## Nearest rerun

Rerun from State 5 after State 11/approved bridge work is available. Then rerun State 6 proof review.

STATUS: REJECTED
