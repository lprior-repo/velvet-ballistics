bead_id: vb-qi37.4.3
bead_title: runtime/storage: Persist run header before acknowledgement
phase: State 6 - implementation
updated_at: 2026-05-11T00:00:00Z

# Implementation

Holzman references used:
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Changes
- Added RED test `submit_direct_returns_durability_error_before_ack_when_header_cannot_persist` in `crates/vb_runtime/src/runtime.rs`.
- Added `Runtime::persist_run_header_before_ack` and stored runtime policy so `submit_direct` appends run submitted/admission metadata before returning success.

## Command Evidence
- RED before implementation: `rtk cargo test -p vb_runtime submit_direct_returns_durability_error_before_ack_when_header_cannot_persist` failed with `left: Ok(()) right: Err(JournalPoisoned)`.
- GREEN after implementation: same command passed, `1 passed`.

## Risk
- Downstream State 8 must run broader gates because this minimal change may duplicate lifecycle events during later shard processing.
# State 6 Repair Addendum

- Repaired duplicate journal header fallout by routing `Runtime::submit_direct` through `ShardCommand::SubmitPrePersisted` after the runtime shell persists and drains `RunSubmitted` plus `RunAdmission` before acknowledgement.
- Updated queued-journal shutdown expectation for the new contract: post-admission queue drainage now counts execution evidence because the run header is already durably drained before ack.
- Evidence: three targeted `vb_runtime` commands in `moon-report.md` passed.
