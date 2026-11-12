# Proof Writer Report — vb-rpch verus-flux-rust-r6

bead: `vb-rpch`  
state: 5 proof/harness repair continuation  
date: 2026-05-24

## Scope executed

Worked only the r6 continuation scope for `VFR-R2-KANI-005..007` and `VFR-R2-FUZZ-001..004`. No Flux work. No production runtime behavior edits.

## Artifacts changed

- `crates/vb_storage/src/kani_recovery_hydrate.rs` — model-reduced the RPCH Kani event generator for `VFR-R2-KANI-005..007` to contiguous vectors of length `0..=2` and eight finite event shapes; added `kani::cover!` non-vacuity markers to the three pending harnesses.
- `.beads/vb-rpch/proof-writer-report-verus-flux-rust-r6.md` — this report.
- `.beads/vb-rpch/proof-evidence-verus-flux-rust-r6.md` — command evidence and blockers.
- `.beads/vb-rpch/trusted-base-ledger.verus-flux-rust-r6.jsonl` — r6 bounds, timeout, and fuzz trust ledger.
- `.beads/vb-rpch/proof-obligations.verus-flux-rust-r6.written.jsonl` — r6 obligation disposition.

## Closed blockers

- `VFR-R2-FUZZ-001..004`: prior musl/sanitizer blocker is precisely classified and bypassed with the repo-sanctioned GNU sanitizer target from `fuzz/README.md` (`--target x86_64-unknown-linux-gnu`). All four exact RPCH cargo-fuzz targets completed 16-run smoke executions.

## Remaining blockers

- `VFR-R2-KANI-005`: still `BLOCKED_RESOURCE_TIMEOUT_R6`. Exact harness `hydrate_run_frame_precond_kani` times out after 180s even after r6 bounded event model reduction. Timeout remains inside allocator/formatter/string/drop paths reached by full production hydration.
- `VFR-R2-KANI-006`: still `BLOCKED_RESOURCE_TIMEOUT_R6`. Exact harness `hydrate_run_frame_from_events_precond_kani` times out after 180s under the bounded event model.
- `VFR-R2-KANI-007`: still `BLOCKED_RESOURCE_TIMEOUT_R6`. Exact harness `replay_events_kani` times out after 180s under the bounded event model.

## Owner state / rerun_from

- owner_state: `State 5 Kani resource/model-reduction sublane`
- rerun_from: `State 5 after either (a) further proof-surface decomposition that avoids production formatter/String allocation paths, or (b) an approved larger Kani resource budget`

## Proof-reviewer disposition

Proof-reviewer may rerun and review r6 artifacts for the fuzz blocker closure and Kani timeout classification. Proof-reviewer must not approve `VFR-R2-KANI-005..007` as passed from r6 evidence.
