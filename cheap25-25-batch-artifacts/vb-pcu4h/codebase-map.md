# codebase-map.md — vb-pcu4h

- bead_id: vb-pcu4h
- bead_source: `/home/lewis/src/velvet-ballistics` (coord checkout only, read-only here)
- isolated_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`
- captured_at: 2026-07-01
- audit_finding: "Pending-action recovery test only checks steps_started count and can pass if pending actions are dropped." (2026-06-30 20-agent audit; epic e06)
- bead_title: "Tests: assert pending-action recovery fields exactly (P1 bug)"
- bead_postcondition: "The audited issue is fixed or explicitly rejected with source evidence: Pending-action recovery test only checks steps_started count and can pass if pending actions are dropped."

## Scope summary

The audit identified a class of recovery tests that hold an `ActionScheduledTicket` (or `ActionScheduled`) event set, recover it via `recover_runtime_frame_seed_from_events` (or its journal-backed alias), and then assert only on:

1. The derived `summary.steps_started` / `actions_scheduled` counters; and/or
2. `seed.unsupported.pending_actions` boolean; and/or
3. `seed.pending_actions.iter().any(|entry| entry.step == X && entry.action == Y)`

   without asserting the **length** of `seed.pending_actions` and without comparing the **whole Vec** against a constructed `Vec<RecoveredPendingAction>` via `PartialEq`.

Because `.any()` returns `true` whenever *at least one* entry matches, the assertion will silently pass if recovery:

- Drops every pending action (vec is empty → `.any()` returns false only if NO match; if the expected match is absent the test SHOULD fail, but coverage still misses "drop everything" because the boolean unsupported flag is independent), or
- Spawns a duplicate/phantom pending action (vec has one expected + one bogus entry → `.any()` still true), or
- Fails to render `unsupported.pending_actions` (the `.any()` covers presence independently).

The audit phrase "only checks steps_started count" maps onto tests that assert derived counters as a proxy for pending-actions presence, never asserting on the actual `Vec<RecoveredPendingAction>` field of the recovered `RecoveryFrameSeed`.

The fix surface is `RecoveredPendingAction` (defined at `crates/vb_storage/src/recovery/types.rs:644-650`) — a `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` struct with exactly two public fields:

- `pub step: StepIdx`
- `pub action: ActionId`

Hence "field-level assertions covering every PendingAction field" equals a struct-level `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(N), action: ActionId::new(M) }])` so any drift in either field or any spurious extra/missing entry is caught by the derived `PartialEq`.

## Target symbols (production)

### `RecoveredPendingAction` struct definition

- path: `crates/vb_storage/src/recovery/types.rs`
- lines: 644-650
- contents (verbatim):
  ```rust
  /// One pending action reconstructed from unresolved action lifecycle events.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct RecoveredPendingAction {
      /// Step that scheduled the action.
      pub step: StepIdx,
      /// Durable action identifier.
      pub action: ActionId,
  }
  ```
- evidence: file read on 2026-07-01 at the isolated workdir; `PartialEq, Eq` derives let downstream tests use `assert_eq!` on the whole struct (every field).

### Recovery accumulator build site for `pending_actions`

- path: `crates/vb_storage/src/recovery/replay/summary/accumulator.rs`
- lines: 35 (field declaration on `FrameSeedAccumulator`):
  ```rust
  pub(super) pending_actions: HashSet<(ActionId, StepIdx)>,
  ```
- lines: 68 (initial value):
  ```rust
  pending_actions: HashSet::new(),
  ```
- evidence: a `HashSet<(ActionId, StepIdx)>` collects every unresolved action; the set is then mapped to `Vec<RecoveredPendingAction>` in `derive.rs:287-296`.

### Vec assembly + sort contract

- path: `crates/vb_storage/src/recovery/replay/summary/derive.rs`
- lines: 287-296
  ```rust
  fn recovered_pending_actions(
      pending_actions: HashSet<(ActionId, StepIdx)>,
  ) -> Vec<RecoveredPendingAction> {
      let mut entries: Vec<RecoveredPendingAction> = pending_actions
          .into_iter()
          .map(|(action, step)| RecoveredPendingAction { step, action })
          .collect();
      entries.sort_by_key(|entry| (entry.step, entry.action));
      entries
  }
  ```
- evidence: sort key is `(entry.step, entry.action)`. Therefore, for any single-event test scenario, the expected output is exactly one entry whose `(step, action)` matches the input event. Tests must `assert_eq!` the whole vec (length 1, sorted fields) rather than `.any(...)`.

### Recovery entry points to be covered by the tests

- `recover_runtime_frame_seed_from_events` at `derive.rs:69-73`
- `recover_runtime_frame_seed_from_events_with_workflow` at `derive.rs:77-83`
- Re-export path: `crates/vb_storage/src/recovery/mod.rs:53-57` (also re-exported as `replay::recover_runtime_frame_seed_from_events`).
- Higher-level journal entry: `recover_runtime_frame_seed` at `crates/vb_storage/src/recovery/recover.rs` (used by `crates/vb_runtime/tests/recovery_hydration_tests.rs`).

## Failing / fuzzy test sites (PRIMARY targets for replacement)

All three tests are in the same file, `crates/vb_storage/src/recovery/replay/summary/tests.rs`, and each uses the fuzzy `pending_actions.iter().any(|entry| entry.step == X && entry.action == Y)` pattern with no `len()` check and no struct-level `assert_eq!`.

### Test A — `unresolved_action_marks_pending_action_recovery_unsupported`

- path: `crates/vb_storage/src/recovery/replay/summary/tests.rs`
- lines: 437-454 (assertion at 449-453)
- assertion under repair (verbatim):
  ```rust
  assert!(
      matches!(seed, Ok(recovered) if recovered.pending_actions.iter().any(|entry|
          entry.step == StepIdx::new(3) && entry.action == ActionId::new(9)
      ) && recovered.unsupported.pending_actions)
  );
  ```
- bug shape:
  - Fuzzy `.any(...)` — passes if any phantom/duplicate `(step=3, action=9)` entry is appended.
  - No length check — passes if zero pending entries (because then `unsupported.pending_actions` is also independent of the vec; `accumulator.pending_actions.is_empty()` is what flips `unsupported.pending_actions` — verify alongside length-0 case).
  - Wrapped in `matches!(seed, Ok(recovered) if ...)` which evaluates a Bool; outer `assert!` only fires when matches! is true. If `seed == Err(_)`, the test silently passes (count as one likely false-positive mode).
- expected_events: a single `JournalEvent::ActionScheduled { run: RunId::new(61), seq: EventSeq::new(0), step: StepIdx::new(3), action: ActionId::new(9), attempt: 1 }`.
- expected_recovered_vec: exactly `vec![RecoveredPendingAction { step: StepIdx::new(3), action: ActionId::new(9) }]` (len 1, sorted — single element).
- replacement contract: derive `seed` via `.expect("seed recovery must succeed for single ActionScheduled")`, then
  - `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(3), action: ActionId::new(9) }]);` (exact vec)
  - `assert!(seed.unsupported.pending_actions);` (unchanged)
  - optional: assert `steps_started` is unaffected (0) and `actions_scheduled == 1`.

### Test B — `action_scheduled_ticket_advances_max_slot_and_step_dimensions`

- path: `crates/vb_storage/src/recovery/replay/summary/tests.rs`
- lines: 621-672 (assertion at 666-671)
- assertion under repair (verbatim):
  ```rust
  assert!(
      seed.pending_actions
          .iter()
          .any(|entry| { entry.step == StepIdx::new(5) && entry.action == ActionId::new(11) }),
      "ActionScheduledTicket must remain pending until completion/abandon",
  );
  ```
- bug shape: same `.any(...)` fuzzy match; no length check.
- context: ticket has `step: StepIdx::new(5)`, `action: ActionId::new(11)`, output slot 9, so `slot_count == 10`, `step_count == 6` are also asserted alongside.
- expected_recovered_vec: exactly `vec![RecoveredPendingAction { step: StepIdx::new(5), action: ActionId::new(11) }]`.
- replacement contract: `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(5), action: ActionId::new(11) }]);` plus the existing `slot_count == 10` and `step_count == 6` assertions.

### Test C — `crash_after_schedule_then_recover_hydrates_resume_queue`

- path: `crates/vb_storage/src/recovery/replay/summary/tests.rs`
- lines: 743-809 (assertion at 787-792)
- assertion under repair (verbatim):
  ```rust
  assert!(
      seed.pending_actions
          .iter()
          .any(|entry| { entry.step == StepIdx::new(6) && entry.action == ActionId::new(17) }),
      "crashed-while-pending action must surface in the resume queue",
  );
  ```
- bug shape: same `.any(...)` pattern; no length check.
- context: ticket has `step: StepIdx::new(6)`, `action: ActionId::new(17)`, output slot 8 → `slot_count == 9`, `step_count == 7` already asserted.
- expected_recovered_vec: exactly `vec![RecoveredPendingAction { step: StepIdx::new(6), action: ActionId::new(17) }]`.
- replacement contract: `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(6), action: ActionId::new(17) }]);`.

## Length-checked `.find()` secondary targets (LENGTH IS CHECKED, but field match is still fuzzy)

These tests *do* `assert_eq!(seed.pending_actions.len(), 1)`, so the "drop all pending actions" failure mode is contained — but the field assertion still uses `.iter().find(|entry| ...)` with `entry.action == action` and then a separate `assert_eq!(pending.step, StepIdx::new(2))`. They should be uplifted to struct-level `assert_eq!` for completeness under the bead language ("covering every PendingAction field" + "exact field-level assertions"). Lower priority than PRIMARY targets.

### Secondary target D — `pending_action_persisted_restart_via_appends_with_syncall`

- path: `crates/vb_runtime/tests/recovery_hydration_tests.rs`
- lines: 1856-1946; assertion block at 1895-1905.
- current state:
  ```rust
  let pending = seed
      .pending_actions
      .iter()
      .find(|entry| entry.action == action)
      .expect("pending action must survive journal reopen");
  assert_eq!(pending.step, StepIdx::new(2));
  assert_eq!(seed.pending_actions.len(), 1);
  ```
- uplift: `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(2), action }]);` and drop the `.find` plumbing.

### Secondary target E — second persisted-restart / crash test

- path: `crates/vb_runtime/tests/recovery_hydration_tests.rs`
- lines: ~2027-2037 (`recover_runtime_frame_seed(&journal, run)` after the child crash path)
- current state: same `.find(|entry| entry.action == action)` plus `assert_eq!(pending.step, StepIdx::new(2))` plus `assert_eq!(seed.pending_actions.len(), 1)`.
- uplift: identical to D.

## Field-by-field type-test already present (NOT a target)

- path: `crates/vb_storage/src/recovery_type_tests.rs`
- lines: 118-126
- existing test `recovered_pending_action_carries_step_and_action` already asserts `assert_eq!(pending.step, StepIdx::new(7))` and `assert_eq!(pending.action, ActionId::new(99))` directly on a hand-constructed `RecoveredPendingAction`. This test is already exhaustive over the two fields (`step`, `action`).
- decision: leave as-is; an additional case using full struct-level `assert_eq!` against a constructed literal is appropriate but does not change the existing test's coverage.

## Derived-state test that hand-builds the seed (NOT a target of the bead)

- path: `crates/vb_storage/src/recovery/recovery_unit_tests.rs`
- lines: 314-351 (function `recovery_cannot_resume_state_classifies_pending_action`)
- notes: this test does NOT exercise recovery at all — it hand-constructs a `RecoveryFrameSeed` and asserts only the *derived* `RecoveryCannotResumeState` booleans (`state.pending_actions`, `state.pending_asks`, `state.unsupported_reason() == "pending_actions"`). The audit phrase "only checks steps_started count" loosely matches *this shape* of test (proxies over derived state, no recovery event flow), but the literal text "steps_started count" is not present here. Downstream contract agent must decide whether to widen scope to this test.
- recommendation: out-of-scope for the bead's fix (the bead is about event-driven recovery tests; this test is a hand-build), but flag in delivery-scope as `risk_tag: derived-state-proxy` for review.

## Mirror / verification artifacts (producers and consumers)

### Verus production mirrors

- `verification/verus/production_inner/replay_invariants_production.rs:253-256` mirrors `RecoveredPendingAction` with the same `(pub step: StepIdx, pub action: ActionId)` shape (drift gate at `scripts/check-production-inner-drift.sh`).
- `verification/verus/extern_vb_rpch_replay_invariants.rs:191` re-exports `prod_src::RecoveredPendingAction`.
- `verification/verus/production_inner/recovery_verification_production.rs:25,45` references the same struct.
- evidence: these are STRONG-binding mirrors; they do NOT need to be edited for the test-strength change, but must remain drift-free if `RecoveredPendingAction` ever gained a field.

### Existing proof/verifier harnesses for `RecoveredPendingAction`

- The struct is not exercised by a standalone Verus/Flux proof; verifier coverage is provided indirectly via the recovery-state invariants in `_production_strong_bind_recovery.rs` (lines 234-308) and `vb_rpch_flux_r8.rs` / `vb_rpch_flux_r9.rs` (refined-by of `pending_actions: bool` is on `UnsupportedRecoveryState`, not on `RecoveredPendingAction`).
- implication: changing the assertion style in the unit test does NOT require Verus/Flux updates; the bead lane is "Test: assertion strength" only.

## Risks / classification

| Risk tag | Detail |
|----------|--------|
| derived-state-proxy | The three PRIMARY tests assert on `seed.unsupported.pending_actions` AND `.any(...)` rather than the full Vec; replacement must keep BOTH the boolean and the new struct-level vec equality so the unsupported-flag chain continues to be exercised. |
| len-drift | `.any(...)` would still pass if a phantom duplicate is appended; replacement `assert_eq!` on the whole vec catches both length and field drift simultaneously (the `Vec` `PartialEq` checks length and per-element equality). |
| silent-err-passes | Test A wraps the assertion in `matches!(Ok(recovered) if ...)`; if `seed` is `Err(_)`, the outer `assert!` only fires if the matches! returns true. After replacement, replace with `let recovered = seed.expect("…")` first so `Err` panics with context (NOT a silent pass). |
| mutation-strength | The fuzzy assertion is a target for `cargo-mutants` (find by value / swap order) and would mask a deletion-of-fields mutation; the replacement `assert_eq!` on the full Vec bounds the mutation-resistance contract. |
| already-coverage | Test `recovered_pending_action_carries_step_and_action` (`recovery_type_tests.rs:118-126`) already exhaustively walks both fields; replacement must NOT be redundant with that, but at minimum must hold a new spec-level `assert_eq!(recovered, expected)` to strengthen the *event-driven* recovery lane. |

## Targeted closure commands (smallest trustworthy gates before `moon ci`)

- Targeted test compile/run for the three PRIMARY tests:
  ```bash
  cargo test -p vb_storage --lib -- --nocapture \
    unresolved_action_marks_pending_action_recovery_unsupported \
    action_scheduled_ticket_advances_max_slot_and_step_dimensions \
    crash_after_schedule_then_recover_hydrates_resume_queue
  ```
- Targeted secondary runtime tests:
  ```bash
  cargo test -p vb_runtime --test recovery_hydration_tests -- --nocapture \
    pending_action_persisted_restart_via_appends_with_syncall
  ```
- Lint-source gate (zero-tolerance):
  ```bash
  moon run :lint-src
  ```
- Format gate:
  ```bash
  cargo fmt --all -- --check
  ```
- Canonical gate:
  ```bash
  moon ci
  ```

## Open questions for downstream agents

1. Should the bead's fix also touch the derived-state hand-build test `recovery_cannot_resume_state_classifies_pending_action` at `recovery_unit_tests.rs:314-351` to add a `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { … }])` line? (Out-of-scope vs in-scope decision rests with the contract agent.)
2. Should the secondary `.find()` tests (`recovery_hydration_tests.rs:1899-1905` and `:2031-2037`) be uplifted to struct-level `assert_eq!` in the same patch, or queued as a follow-up?
3. Are the existing `assert_eq!(seed.pending_actions.len(), 1)` calls in `recovery_hydration_tests.rs` sufficient to satisfy the bead's "exact field-level" requirement when the `.find()` callback already checks `entry.action == action`?
4. Is there any Kani/Flux harness that would benefit from being pointed at this changed test? (No current `RecoveredPendingAction`-specific Kani harness observed.)

## Excluded paths (explicitly out-of-scope)

- All `vb_compile`, `vb_cli`, `vb_dispatch` paths: the bead is bounded to recovery tests.
- `verification/verus/**` mirrors: the struct definition is unchanged; no Verus edit needed.
- `crates/fuzz/**`: no fuzz harness directly covers `RecoveredPendingAction` shape (grep `RecoveredPendingAction` in `fuzz/` returns no matches; UNKNOWN until probed).
- `scripts/check-production-inner-drift.sh`, `scripts/check-verus-production-binding.sh`, `scripts/check-nightly-features.sh`: not edited by the bead; will run as gates only.

## Files mapped

- `crates/vb_storage/src/recovery/types.rs` (RecoveredPendingAction at 644-650)
- `crates/vb_storage/src/recovery/replay/summary/derive.rs` (recovered_pending_actions at 287-296; recovery entry points at 69-83)
- `crates/vb_storage/src/recovery/replay/summary/accumulator.rs` (HashSet at 35, init at 68)
- `crates/vb_storage/src/recovery/replay/summary/mod.rs` (test wiring at 33-34)
- `crates/vb_storage/src/recovery/replay/summary/tests.rs` (PRIMARY targets at 437-454, 621-672, 743-809)
- `crates/vb_storage/src/recovery/mod.rs` (test wiring at 26-33; re-exports at 39-67)
- `crates/vb_storage/src/recovery_type_tests.rs` (field-by-field coverage at 118-126)
- `crates/vb_storage/src/recovery/recovery_unit_tests.rs` (derived-state hand-build at 314-351)
- `crates/vb_runtime/tests/recovery_hydration_tests.rs` (secondary `.find()` at 1899-1905, 2031-2037)
- `verification/verus/production_inner/replay_invariants_production.rs` (mirror at 253-256)
- `verification/verus/extern_vb_rpch_replay_invariants.rs` (re-export at 191)
- `verification/verus/production_inner/recovery_verification_production.rs` (references at 25, 45)

## UNKNOWN / MISSING items

- UNKNOWN: whether the bead's audit "steps_started count" wording literally maps to `summary.steps_started` in any single test or is a generic phrasing about counter-only assertions. No test in the repo asserts `summary.steps_started` for the pending-action flow without also asserting `pending_actions` vec; closest analogue is `recovery_unit_tests.rs:323` (`steps_started: 1` as part of the hand-built seed but the assertion afterwards does NOT check pending_actions field — flagged for contract review).
- MISSING: there is no per-bead evidence directory at `.beads/vb-pcu4h/evidence/`; downstream agents must create one before black-hat review.
