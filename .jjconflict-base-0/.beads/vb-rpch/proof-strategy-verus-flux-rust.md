# Proof Strategy — vb-rpch Verus/Flux/Rust Repair Plan

Scope: remaining non-TLC work only. The approved TLC round-3 evidence in `.beads/vb-rpch/proof-review-tlc-fix-round3.md` is preserved and not replaced.

Discovery evidence run from `/home/lewis/src/vb-jpq7-jj-fix`:
- `command -v verus && verus --version` → `/home/lewis/.local/bin/verus`, Verus `0.2026.05.05.d03e906` available.
- `cargo flux --version` → `error: no such command: flux`; Flux lane is `BLOCKED_TOOLING` until tool is installed and source annotations are viable.
- `cargo --version && rustc --version` → cargo `1.97.0-nightly`, rustc `1.97.0-nightly`.
- Grep discovery found no existing Verus or Flux annotations in `crates/vb_storage/src/recovery/*.rs`.

## Required remaining clauses

Verus must cover: `INV-002`, `INV-003`, `INV-004`, `INV-005`, `PRE-001`, `PRE-002`, `POST-009`.

Flux RS must have an explicit lane decision for every applicable refinement property. Current status is `BLOCKED_TOOLING`, not a proof pass and not a behavior waiver.

Production Rust proof-attachment work belongs to State 11. Proof-writer must not edit production behavior. If inline annotations are required, State 11 owns the production file edits; State 5 may only write proof/model/harness artifacts and run verifier commands against the State 11 attachment surface.

## Risk classification

- Temporal/state-machine: `POST-009` replay latest-attempt filtering and non-idempotent resolution; TLC already approved bounded abstraction, Rust refinement remains.
- Rust-local invariant: `INV-002`, `INV-004`, `INV-005`.
- Bounded state/arithmetic: `INV-003`, `PRE-001`, `PRE-002` dimension counts and `u16` boundaries.
- Refinement/type-state: Flux candidates for supported-state flags, positive dimensions, digest hierarchy, resolved-tracker monotonicity, tail sequencing.
- Unsafe/UB: no unsafe in target files due `#![forbid(unsafe_code)]`; Miri not primary for this planning suffix.
- Untrusted input: event slices, snapshot bytes, journal-derived indexes.

## Execution sequence

1. **State 11 — Rust proof-attachment changes**: attach real-code proof surfaces without changing behavior: expose tiny pure helpers if needed, add gated Verus/Flux annotation modules/features, remove proof blockers such as private-only internals where proof requires stable specifications. Holzman constraints: no unsafe, no unwrap/expect/panic/todo/unimplemented/dbg, checked arithmetic, bounded loops, no behavior-affecting shortcuts.
2. **State 5 — Proof-writer**: write/repair Verus proof artifacts and commands against actual Rust implementation surfaces. No production behavior edits.
3. **Flux recheck before State 11/5**: run `cargo flux --version`. If available, write Flux refinement obligations against actual source annotations; if still unavailable, keep `BLOCKED_TOOLING` with fresh evidence and expiry.
4. **State 6 — proof-plan-reviewer/proof-reviewer**: review plan/proof evidence. Do not cite Flux as passed unless a real `cargo flux` command runs successfully.

## Verus plan

Use Verus as the Rust-core spine. Required proof targets:
- `types.rs::UnsupportedRecoveryState::{SUPPORTED, union}`: boolean algebra and all-false supported state.
- `types.rs::ActionReplayTracker`: monotonic resolution after completed/failed insertion.
- `types.rs::DigestCheck`: strict hierarchy modeled by an explicit pure rank/check function, not enum discriminant assumptions.
- `summary.rs::recover_runtime_frame_seed_from_events` / `dimension_count`: non-empty event paths with observed step/slot evidence derive positive counts or typed errors.
- `hydrate.rs::{hydrate_run_frame, hydrate_run_frame_from_events}`: entry guards and dimension checks bind to real implementation behavior.
- `replay/core.rs::replay_events`: max-attempt filtering, step ordering, and non-idempotent resolved guard.

Exact planned commands are in `proof-obligations.verus-flux-rust.planned.jsonl`.

## Flux plan

Applicable refinement properties:
- `UnsupportedRecoveryState::SUPPORTED` has all flags false.
- `UnsupportedRecoveryState::union` is flag-wise OR and monotone.
- `RecoveryFrameSeed.step_count > 0` and `slot_count > 0` on successful non-empty/evidence-bearing recovery paths.
- `DigestCheck` level rank preserves strict hierarchy.
- `ActionReplayTracker::is_resolved` remains true after mark-completed/mark-failed.
- Hydration tail events have `event.seq > snapshot.seq` on accepted paths.

Current decision: `BLOCKED_TOOLING` because `cargo flux --version` fails with `no such command: flux`. Expiry: before any State 11 production annotation work and before State 5 claims Flux evidence.

## TLC preservation

The approved TLC round-3 lane remains authoritative for bounded finite temporal abstraction. This plan does not weaken, rerun, overwrite, or replace `.beads/vb-rpch/proof-review-tlc-fix-round3.md`.
