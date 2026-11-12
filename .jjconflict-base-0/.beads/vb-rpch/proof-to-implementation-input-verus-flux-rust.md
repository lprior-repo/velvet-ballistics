# Proof-to-Implementation Input — vb-rpch Verus/Flux/Rust

## State routing

- **State 11 owns production Rust proof-attachment and any behavior repair.**
- **State 5 owns proof/model/harness artifacts only.**
- Inline production annotations, feature gates, pure helper exposure, and behavior fixes are State 11 work. Proof-writer may not edit production behavior.

## Required implementation attachment points

1. `crates/vb_storage/src/recovery/types.rs`
   - `UnsupportedRecoveryState::SUPPORTED` and `union` for `INV-002`.
   - `ActionReplayTracker::{mark_completed, mark_failed, is_resolved}` for `INV-004`.
   - `DigestCheck` explicit rank/inclusion helper for `INV-005`.
2. `crates/vb_storage/src/recovery/replay/summary.rs`
   - `dimension_count` and seed construction paths for `INV-003`.
3. `crates/vb_storage/src/recovery/hydrate.rs`
   - `hydrate_run_frame` accepted/error branches for `PRE-001`.
   - `hydrate_run_frame_from_events` accepted/error branches for `PRE-002`.
4. `crates/vb_storage/src/recovery/replay/core.rs`
   - `compute_max_attempt`, `validate_contiguous_sequences`, `replay_events` for `POST-009`.

## Verus bridge obligations

- Proofs must bind to real code shape or gated production proof surfaces; independent toy definitions are prohibited.
- Required commands are listed in `proof-obligations.verus-flux-rust.planned.jsonl` as `VFR-VERUS-001` through `VFR-VERUS-007`.
- Expected evidence is Verus `0 errors`; planning does not claim that evidence exists.

## Flux bridge obligations

- Recheck `cargo flux --version` first.
- If available, State 11 must add viable Flux annotations or explain non-viability per property with concrete compiler evidence.
- Planned command: `cargo flux --package vb_storage` from `/home/lewis/src/vb-jpq7-jj-fix`.
- Current status: `BLOCKED_TOOLING`; no Flux pass may be claimed.

## Holzman Rust acceptance for State 11

- No unsafe or panic-family constructs.
- All arithmetic remains checked or explicitly saturating with proof/trust-base explanation.
- No public behavior weakening to satisfy proofs.
- If a proof reveals implementation/contract mismatch, repair implementation and then rerun proof planning/review, not the other way around.
