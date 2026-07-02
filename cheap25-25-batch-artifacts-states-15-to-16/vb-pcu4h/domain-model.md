# Domain Model — vb-pcu4h

- bead_id: vb-pcu4h
- bead_title: "Tests: assert pending-action recovery fields exactly (P1 bug)"
- bead_postcondition: "Pending-action recovery test only checks steps_started count and can pass if pending actions are dropped." — fix or explicit reject with source evidence.
- artifact_owner: rust-contract
- model_kind: assertion-strength uplift (no production-code model change)
- ub_liguistic_origin: Wlaschin / Fowler context, Fowler "Specification by Example" + Munich Re audit phrasing.

## Ubiquitous Language

| Term | Meaning | Type anchor |
|------|---------|-------------|
| Pending action | A scheduled action for which no completion or abandon has been observed in the journal; hydration needs to re-install its boundary on resume. | `RecoveredPendingAction` |
| Pending-action field | One of `step` or `action` of a pending action; both are durable IDs. | `RecoveredPendingAction::{step, action}` |
| Pending-action vec | `Vec<RecoveredPendingAction>` recovered from journal events; sorted by `(step, action)` so equality is canonical across HashSet extraction. | `RecoveryFrameSeed::pending_actions` |
| Recovered seed | The whole `RecoveryFrameSeed` derived by replaying durable journal events. | `RecoveryFrameSeed` |
| Unsupported flag (pending_actions) | Boolean derived from `accumulator.pending_actions.is_empty()` indicating that recovery classified the run as unsupported because some action lifecycle did not terminate. | `UnsupportedRecoveryState::pending_actions` |
| Schedule-only event | A journal event such as `ActionScheduled` or `ActionScheduledTicket` that opens an action boundary but has no completion/abandon sibling. | `JournalEvent::ActionScheduled*` |
| Exact assertion | An `assert_eq!` on the full `Vec<RecoveredPendingAction>` against a constructed literal vec; covers length and per-element fields simultaneously. | derived `PartialEq, Eq` |
| Fuzzy assertion | A boolean predicate such as `.iter().any(...)` or `.iter().find(...)` that returns true on the first match; cannot distinguish drop-all from correct, or phantom-duplicate from correct. | (anti-pattern under repair) |
| Step dimension | The set of steps visible in the recovered seed (`step_count`); orthogonal to pending-actions vec. | `seed.step_count` |
| Slot dimension | The set of slot indices visible in the recovered seed (`slot_count`); orthogonal to pending-actions vec. | `seed.slot_count` |

## Aggregate: RecoveryFrameSeed (test-side view)

The bead's lens is the seed as observed by the test after `recover_runtime_frame_seed_from_events` (or its journal-backed alias). Within the bead scope the seed has one aggregate-level invariant relevant to tests:

- INV-DOM-1 — Exact-pending-vec invariant. After replaying the single-event fixtures shipped in the three PRIMARY tests, the recovered `pending_actions` vec MUST equal exactly `vec![RecoveredPendingAction { step: <S>, action: <A> }]` for the unique `(step=S, action=A)` pair carried by the fixture. No additional entries. No missing entries. The vec's `PartialEq` is the single source of truth.

## Entities / Value Objects

- `RecoveredPendingAction` (production, unchanged by bead) — `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` struct with `pub step: StepIdx` and `pub action: ActionId`. Two public fields; field-level equality is the audit's "exact field-level" surface.
- `StepIdx` / `ActionId` (newtypes from `vb_core`) — durable IDs; `==` is total order over their inner `u32`.
- `RecoveryFrameSeed.pending_actions` — `Vec<RecoveredPendingAction>`; sorted ascending by `(step, action)` (canonical order enforced by `derive.rs:294`).

## Forbidden states (must remain unrepresentable post-fix)

- FST-1 — A `RecoveryFrameSeed` whose `pending_actions` vec carries a duplicate `(step, action)` entry. Forbidden by `HashSet` in `accumulator.rs:35,68`; the test must enforce the same uniqueness invariant structurally via `Vec` equality (length 1 expected).
- FST-2 — A test that can pass when `pending_actions` is dropped to empty but `unsupported.pending_actions` is asserted alongside. Forbidden by replacing the boolean-only assertion with a Vec-equality assertion that is structurally independent of the unsupported flag.

## Commands (test-side only)

- TEST-CMD-1 — `recover_runtime_frame_seed_from_events(&events)` (single `ActionScheduled` event for Test A).
- TEST-CMD-2 — `recover_runtime_frame_seed_from_events(&events)` (single `ActionScheduledTicket` event for Test B).
- TEST-CMD-3 — `recover_runtime_frame_seed_from_events(&events)` (event sequence `[RunAccepted, StepStarted, ActionScheduledTicket]` for Test C).

## Events (input to the recovery reducer, not to be asserted)

- `JournalEvent::ActionScheduled { run, seq, step, action, attempt }` — Test A.
- `JournalEvent::ActionScheduledTicket { run, seq, ticket, input, output, action_abi_digest }` — Tests B and C.
- `JournalEvent::RunAccepted { run, seq, workflow }` and `JournalEvent::StepStarted { run, seq, step, attempt }` — Test C preamble only; not the focus of the fix.

## Policies

- POL-1 — Test assertion policy. Pending-action presence MUST be asserted as a full `Vec<RecoveredPendingAction>` equality against a constructed literal; `.iter().any(...)` is forbidden in this codebase for the recovery lane.
- POL-2 — `Err` propagation policy. Test A's `matches!(seed, Ok(recovered) if …)` outer pattern is replaced by `let recovered = seed.expect("…")` so any `Err(_)` panics with context and cannot silently pass.
- POL-3 — Boolean preservation policy. The `unsupported.pending_actions` boolean assertion MUST remain alongside the new Vec-equality assertion in Tests A so the unsupported-flag derivation continues to be exercised.
- POL-4 — Drift-mirror policy. `verification/verus/production_inner/replay_invariants_production.rs:253-256` and `verification/verus/production_inner/recovery_verification_production.rs:25,45` continue to mirror production `RecoveredPendingAction` byte-for-byte; the bead does not edit these mirrors, but `scripts/check-production-inner-drift.sh` runs as a gate.
- POL-5 — Out-of-scope policy. The hand-built-seed test `recovery_cannot_resume_state_classifies_pending_action` at `recovery_unit_tests.rs:314-351` does NOT exercise recovery; it is flagged in `delivery-scope.jsonl` but the contract leaves its handling to the test-planner (not auto-included in this bead).

## Aggregates (test-facing)

- AGG-1 — `pending_actions` (Vec) is the recovery-output aggregate the bead fixes. Its identity is `(length, sorted (step, action) pairs)`; equality via `PartialEq` is exact.

## Audit language mapping

- The audit phrase "only checks steps_started count" maps under this contract to "only checks `summary.steps_started` OR `unsupported.pending_actions` OR `.iter().any(...)` — never asserts the full `pending_actions` vec." The replacement maps each of the three PRIMARY test sites onto INV-DOM-1 (Vec-equality assertion).
- The audit phrase "can pass if pending actions are dropped" maps under this contract to FST-2 (the silent-pass mode of `matches!(Ok(_) if …)`).

## Open domain questions (forwarded to test-planner / holzman-rust)

1. Should the same Vec-equality uplift be applied to the SECONDARY targets in `crates/vb_runtime/tests/recovery_hydration_tests.rs:1899-1905` and `:2031-2037`? Both already check `len() == 1` but use `.find(...)` for the field match. The contract RECOMMENDS uplift for completeness; ownership rests with test-planner.
2. Should the hand-built-seed test `recovery_cannot_resume_state_classifies_pending_action` (`recovery_unit_tests.rs:314-351`) gain a `assert_eq!(seed.pending_actions, vec![…])` line? It already builds a vec by hand, so the assertion would be tautological for the hand-built part; the contract classifies this as `optional-modify` and RECOMMENDS leaving it alone (no incremental coverage gained).