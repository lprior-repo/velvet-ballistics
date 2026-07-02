# Proof Writer Report — vb-rpch verus-flux-rust-r4

## Scope

- Bead: `vb-rpch`
- State: 5 proof repair
- Workdir: `/home/lewis/src/vb-jpq7-jj-fix`
- Rejection repaired: `PF-VFR-R3-001` / `VFR-R2-VERUS-007`
- Secondary cleanup: blocker dispositions for Flux/Kani/proptest/fuzz/Rust-attach and provenance ledger absence.

## Artifacts changed

- `verification/verus/vb_rpch_replay_events.rs`
- `.beads/vb-rpch/proof-evidence-verus-flux-rust-r4.md`
- `.beads/vb-rpch/proof-writer-report-verus-flux-rust-r4.md`
- `.beads/vb-rpch/trusted-base-ledger.verus-flux-rust-r4.jsonl`
- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r4.written.jsonl`
- `.beads/vb-rpch/agent-invocation-ledger.jsonl`

## VFR-R2-VERUS-007 disposition

Status: `REPAIRED_VERUS_SMOKE_PASS` at Verus ghost-proof scope.

The rejected `requires forall ... stale ==> !state_effect` premise was removed. The repaired artifact models `ReplayState`, `replay_step`, and `replay_from`. Stale attempts are now no-ops by construction: `replay_step` returns the unchanged state for stale events before considering any state-effect cases. Sequence preservation is proved by induction over stale-event suffixes.

This proves the intended replay-filter property, not the false claim that stale events cannot be state-effecting event variants.

## Commands and results

1. `verus "verification/verus/vb_rpch_replay_events.rs" --crate-type lib` — exit 0, `9 verified, 0 errors`.
2. Full Verus smoke suite over the seven vb-rpch artifacts plus production bridge — exit 0, `4/4/5/2/3/3/9/1 verified, 0 errors`.
3. Trust-marker scan over `verification/verus/vb_rpch_*.rs` — exit 0, only comment-only `trusted` wording in `vb_rpch_production_bridge.rs`; no executable trust marker found.
4. Requires/conclusion scan — exit 0; `old_attempts_have_no_state_effect` absent; VFR-R2-VERUS-007 no longer requires `!state_effect`.

## Remaining blockers

- `VFR-R2-FLUX-001..007`: `BLOCKED_TOOLING`; `cargo flux` unavailable.
- `VFR-R2-KANI-001..007`: `BLOCKED_GLOBAL_COMPILE`; cfg(kani) compile errors in admission code block harness execution.
- `VFR-R2-PROPTEST-001..007`: `BLOCKED_MISSING_ARTIFACT`; planned test target absent.
- `VFR-R2-FUZZ-001..004`: `BLOCKED_MISSING_ARTIFACT`; planned fuzz target absent.
- `VFR-R2-RUST-ATTACH-001..007`: `BLOCKED_PLAN_COMMAND`; planned `--features verus` command is invalid for `vb_storage`.
- Provenance: current R4 invocation ledger row created; prior invocation provenance remains absent/unknown and must not be treated as reconstructed.

No full State 5 proof closure is claimed.
