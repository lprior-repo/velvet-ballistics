# Rust Contract: vb-r8t6e fixture schema + dead-code compile repair

**Bead:** vb-r8t6e
**Type:** Bounded test/bench/kani fixture compile repair
**Date:** 2026-10-21
**Authoritative failures:** canonical-frozen-baseline (vb-2lyiw) `verify-loom`,
`benchmark-proof`, `verify-kani-vb-validate`, and `check` (-Dwarnings dead-code).

## Authoritative field contracts

- `ActionTicket.run_attempt: u16` (1-indexed durable run attempt),
  `crates/vb_core/src/action/ticket.rs:16`; field order
  `run, step, seq, action, run_attempt, attempt, idempotency_key, capacity`.
  Canonical key fn `compute_action_idempotency_key(run, run_attempt, seq, action)`.
- `WorkflowParts.input_slots: Box<[CompiledInputSlot]>`,
  `crates/vb_core/src/workflow/mod.rs:369` (`#[serde(default)]`).

## Fixes (schema sites in exact callgraph)

1. `crates/vb_runtime/src/models/loom/idempotency_retry_eviction.rs:34` — add
   `run_attempt: 1` to `ActionTicket` literal (neutral canonical first run; the
   loom model keys on the passed `idempotency_key`, not `run_attempt`).
2. `crates/workspace_tests/benches/action_dispatch.rs:55` — add `run_attempt: 1`.
3. `crates/vb_validate/src/verification/kani_gate_08_accessor.rs:184` — add
   `input_slots: Box::new([])` to `workflow_parts_with_accessors`. The property
   under test (`validate_gate_08_accessor_path_segments`) reads only
   `accessors`/`slot_count`/`symbols_count`; empty `input_slots` preserves
   symbolic accessor coverage and existing assertions (not a hardcoded proof shape).

## Coordinator-directed bounded dead-code fixture cleanup (same `check` gate)

No `allow(dead_code)`/`expect`/warning relaxation; no removed `#[test]` or assertions.

- `crates/workspace_tests/tests/runtime_fjall_pending_action_recovery.rs` — remove
  dead `assert_tick_cannot_resume` (redundant with used `assert_cannot_resume`).
- `crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs` — remove dead
  `FixedArtifactStore`+impl, `ticket`, `valid_artifact`, `count_scheduled`,
  `count_completed`; trim unused imports (`AdmissionError`, `ActionReplayTracker`,
  `replay_journal`, plus imports orphaned by the removals).
- `crates/workspace_tests/tests/vb_lp2v_proof_admission_bdd.rs` — trim 5
  unused-import lines (test bodies untouched).

## Verification

- Loom: `RUSTFLAGS="-Dwarnings --cfg loom" cargo test -p vb_runtime --lib --all-features models::loom`
  → 13 passed, 0 failed.
- Bench: `cargo bench -p velvet-ballistics-workspace-tests --bench action_dispatch --no-run` → exit 0.
- Check gate: `RUSTFLAGS="-Dwarnings" cargo check -p velvet-ballistics-workspace-tests --tests --all-features` → exit 0.
