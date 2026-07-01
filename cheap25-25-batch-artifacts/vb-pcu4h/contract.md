# Contract — vb-pcu4h

- bead_id: vb-pcu4h
- bead_title: "Tests: assert pending-action recovery fields exactly (P1 bug)"
- artifact_owner: rust-contract
- contract_kind: test-only mutation-strength uplift; no production-code contract change.
- mirror_drift_gate: `scripts/check-production-inner-drift.sh` (passes unchanged).
- mirror_binding_gate: `scripts/check-verus-production-binding.sh` (passes unchanged; STRONG `#[path]` already in place).

## Authority

- Mandatory startup read: `/home/lewis/.agents/skills/rust-contract/SKILL.md` (this file wins on conflict per AGENTS).
- Parent contract: `.beads/master/contract.md` (master document, narrowed here).
- Sibling contracts: `.beads/vb-pcu4h/{domain-model.md, type-contracts.md, workflow-model.md, error-taxonomy.md, boundary-map.md, hazard-analysis.md}` are binding on this bead.
- Source-of-truth: `crates/vb_storage/src/recovery/types.rs:644-650` defines `RecoveredPendingAction` with `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`; `crates/vb_storage/src/recovery/replay/summary/derive.rs:287-296` defines the canonical sort order.

## Scope

In scope:

- Rewrite the assert region of `unresolved_action_marks_pending_action_recovery_unsupported` at `crates/vb_storage/src/recovery/replay/summary/tests.rs:437-454`.
- Rewrite the assert region of `action_scheduled_ticket_advances_max_slot_and_step_dimensions` at `crates/vb_storage/src/recovery/replay/summary/tests.rs:621-672`.
- Rewrite the assert region of `crash_after_schedule_then_recover_hydrates_resume_queue` at `crates/vb_storage/src/recovery/replay/summary/tests.rs:743-809`.
- Recommended (optional) uplift: rewrite the assert region of `pending_action_persisted_restart_via_appends_with_syncall` and the second persisted-restart test in `crates/vb_runtime/tests/recovery_hydration_tests.rs:1899-1905, 2031-2037`.

Out of scope:

- All production code (`crates/vb_storage/src/recovery/**`, `crates/vb_runtime/**`, `crates/vb_codegen/**`, etc.).
- All Verus / Flux / Kani / proptest / fuzz artifacts (the bead's mutation is test-only).
- All mirrors under `verification/verus/production_inner/**` (drift gate runs as gate only).
- All scripts under `scripts/check-*.sh` (gate runners only).

## Oracle

- Recovery reducer: `recover_runtime_frame_seed_from_events` at `crates/vb_storage/src/recovery/replay/summary/derive.rs:69-73`.
- Constructed expected vec: `vec![RecoveredPendingAction { step: <StepIdx literal>, action: <ActionId literal> }]` — derived from the input `JournalEvent` in the test fixture; no oracle lookup required.

## Preconditions

- PRE-001 — `RecoveredPendingAction` retains `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` at `crates/vb_storage/src/recovery/types.rs:644-650`. (Bead does not edit; verified by drift gate.)
- PRE-002 — `recover_runtime_frame_seed_from_events` retains its signature `fn(&[JournalEvent]) -> RecoveryResult<RecoveryFrameSeed>` and returns `Ok(_)` for the three PRIMARY fixtures. (Bead does not edit; verified by test compile.)
- PRE-003 — `recovered_pending_actions` (`derive.rs:287-296`) sorts ascending by `(step, action)`. Single-element vec; sort order is canonical. (Bead does not edit; verified by drift gate.)
- PRE-004 — Verus mirror at `verification/verus/production_inner/replay_invariants_production.rs:253-256` matches production byte-for-byte. (Bead does not edit; verified by `scripts/check-production-inner-drift.sh`.)
- PRE-005 — The three PRIMARY test fixtures (Test A: single `ActionScheduled`; Test B: single `ActionScheduledTicket`; Test C: `[RunAccepted, StepStarted, ActionScheduledTicket]`) are byte-identical pre- and post-fix; only the assert region is rewritten.

## Postconditions

- POST-001 — Test A: `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(3), action: ActionId::new(9) }]);` AND `assert!(recovered.unsupported.pending_actions);`. The `.expect("…")` panic message is `"seed recovery must succeed for single ActionScheduled"` (or equivalent diagnostic text).
- POST-002 — Test B: `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(5), action: ActionId::new(11) }]);`. The existing `.expect("schedule-only event must produce a seed")` is retained. The existing `slot_count == 10`, `step_count == 6`, `seed.steps.iter().any(...)`, and `summary.actions_scheduled == 1` assertions are preserved.
- POST-003 — Test C: `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(6), action: ActionId::new(17) }]);`. The existing `.expect("post-schedule crash must produce a recoverable seed")` is retained. The existing `slot_count == 9`, `step_count == 7`, and the redundant `let _ = frame_recovery;` recovery call are preserved.
- POST-004 — The `use crate::recovery::replay::summary::*;` import line at `tests.rs:2` is preserved. The struct literal `RecoveredPendingAction { step, action }` is reachable via the existing `summary::*` glob import (re-exported from `summary/mod.rs`).
- POST-005 — No `.iter().any(|entry| entry.step == X && entry.action == Y)` pattern remains in the three PRIMARY test bodies after the patch.
- POST-006 — No `matches!(seed, Ok(recovered) if <inner>)` outer pattern remains in the three PRIMARY test bodies after the patch.

## Invariants

- INV-001 — Vec equality covers length drift AND per-element field drift in a single `assert_eq!` call. The audit's three failure modes (drop-all, phantom-duplicate, field-drift) all panic.
- INV-002 — The unsupported-flag derivation (`accumulator.pending_actions.is_empty() → unsupported.pending_actions`) continues to be exercised in Test A by the preserved `assert!(recovered.unsupported.pending_actions);` line.
- INV-003 — Sort canonicality. Single-element literal vec; sort order is trivial. Multi-element tests (not in this bead) must construct literal vecs in sorted order to match the reducer's canonical order.
- INV-004 — Drift-free production mirror. Verus mirror at `replay_invariants_production.rs:253-256` matches production `RecoveredPendingAction` byte-for-byte. Drift gate passes.
- INV-005 — Production-binding gate. STRONG `#[path = "..."]` binding on the Verus mirror is preserved; binding gate passes.

## Error taxonomy (test-side)

- ET-001 — `expect("…")` panic-on-Err. The reducer returning `Err(_)` panics with the named message + the `Err(_)` Debug payload.
- ET-002 — `assert_eq!` Vec-mismatch panic. Length or per-element drift panics with the `Debug` of both `Vec` sides.
- ET-003 — `assert!(... unsupported.pending_actions)` panic (Test A only). Boolean false panics with the message.
- ET-004 — Compilation failure if `RecoveredPendingAction` ever requires a new field. Drift gate precedes compile.

## Forbidden (anti-pattern) shapes

- ANTI-001 — `Vec<RecoveredPendingAction>::iter().any(|entry| entry.step == X && entry.action == Y)` (audit's fuzzy predicate).
- ANTI-002 — `matches!(seed, Ok(recovered) if <bool>)` outer pattern (silent-pass risk).
- ANTI-003 — `seed.pending_actions.iter().find(|entry| entry.action == action)` followed by `assert_eq!(pending.step, StepIdx::new(N))` (split assertion; SECONDARY targets).
- ANTI-004 — Asserting only `unsupported.pending_actions` (single boolean; cannot catch Vec drift).
- ANTI-005 — Asserting only `summary.steps_started` or `summary.actions_scheduled` (counter-only proxy; out of bead scope but flagged).

## Acceptance commands

Required closure gates (run from the isolated workdir):

```bash
# Compile + run the three PRIMARY tests
cargo test -p vb_storage --lib -- --nocapture \
    unresolved_action_marks_pending_action_recovery_unsupported \
    action_scheduled_ticket_advances_max_slot_and_step_dimensions \
    crash_after_schedule_then_recover_hydrates_resume_queue

# Optional SECONDARY uplift gate
cargo test -p vb_runtime --test recovery_hydration_tests -- --nocapture \
    pending_action_persisted_restart_via_appends_with_syncall

# Source lint gate (zero tolerance)
moon run :lint-src

# Format gate
cargo fmt --all -- --check

# Canonical closure gate
moon ci
```

Additionally, expected to pass as part of `moon ci`:

```bash
bash scripts/check-production-inner-drift.sh
bash scripts/check-verus-production-binding.sh
```

## Mapping to seed/proof obligations

- Seed PROOF-SEED-001 maps to `tests.rs:437-454` (Test A) with verifier lane profile `Rust-local` (proptest sufficient; no Kani/Verus/Flux needed).
- Seed PROOF-SEED-002 maps to `tests.rs:621-672` (Test B) with the same Rust-local profile.
- Seed PROOF-SEED-003 maps to `tests.rs:743-809` (Test C) with the same Rust-local profile.
- Seed PROOF-SEED-004 maps to SECONDARY targets in `crates/vb_runtime/tests/recovery_hydration_tests.rs` (optional).
- Seed PROOF-SEED-005 maps to the audit's silent-pass risk; covered by PROOF-SEED-001's `expect("…")` panic-on-Err.

## Open questions for downstream agents

1. Should the SECONDARY targets (delivery-scope `optional-modify`) be uplifted in the same patch, or queued as a follow-up bead? (Recommended: same patch; ownership with test-planner.)
2. Should the hand-built-seed test `recovery_cannot_resume_state_classifies_pending_action` (`recovery_unit_tests.rs:314-351`) gain a `assert_eq!(seed.pending_actions, vec![…])` line? (Recommended: NO; the assertion would be tautological against a hand-built vec.)
3. Is a proptest added for `RecoveredPendingAction` Round-Trip coverage (any pair of fields yields the same Vec after sorting)? (Optional; not required for the audit's P1 fix.)

## Verification obligations

- Behavior-affecting: NO (production code unchanged; tests remain tests; only the assertion strength changes).
- Proof obligations: none required for this bead; the existing recovery tests are sufficient. PROOF-SEED rows are emitted to `proof-seeds.jsonl` for the planner's consideration but the planner may mark them `not_applicable` if the audit's P1 fix is closed by the test edit alone.
- Drift gates: `check-production-inner-drift.sh`, `check-verus-production-binding.sh`, `check-nightly-features.sh` — run as closure gates; expected to pass unchanged.

## Hand-off

- Owner downstream: `holzman-rust` (test edits) and `test-planner` (acceptance plan).
- Reviewer downstream: `test-reviewer` (assertion-strength review).
- Evidence: `.beads/vb-pcu4h/evidence/` directory must be created by the implementation agent before black-hat review (per delivery-scope UNKNOWN item).