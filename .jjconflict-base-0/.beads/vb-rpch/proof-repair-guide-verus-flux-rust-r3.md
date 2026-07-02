# Proof Repair Guide — vb-rpch verus-flux-rust-r3

## Required repairs

1. Repair `VFR-R2-VERUS-007` in `verification/verus/vb_rpch_replay_events.rs`.
   - Do not require the exact quantified `old_attempts_have_no_state_effect` body and then ensure the wrapper predicate.
   - Add a real transition/filter model that derives stale-attempt no-state-effect from replay filtering semantics, or downgrade the existing lemma to predicate equivalence and add a separate derivation lemma.

2. Preserve the R3 source-correspondence bridge.
   - Keep `TB-VFR-R3-VERUS-PRODUCTION-SYMBOL-BRIDGE` current if production helper refs move or semantics change.
   - Rerun all eight Verus smoke commands.

3. Do not claim full State 5 closure until blockers are repaired or approved as waivers/blockers by the owning lane:
   - Flux cargo subcommand unavailable.
   - Kani global `kani_admission.rs` compile blocker.
   - Missing `recovery_property_tests` proptest target.
   - Missing `vb_rpch_*` fuzz targets.
   - Invalid planned `rtk cargo check -p vb_storage --features verus` command.

4. Add `.beads/vb-rpch/agent-invocation-ledger.jsonl` provenance rows for proof planning, writing, and review.

## Nearest rerun

Rerun State 5 proof-writer for `VFR-R2-VERUS-007`, then State 6 proof-review. Full proof gate remains rejected until required non-Verus lanes are repaired or explicitly waived/blocked.

STATUS: REJECTED
