# proof-strategy.md — vb-pcu4h

- bead_id: vb-pcu4h
- planner_state: 4
- planner_invocation_id: p4-proof-planner
- captured_at: 2026-07-01 (UTC)
- planner_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`
- schema_version: proof-strategy/v1
- producer: proof-planner (read-only on production code; planning only)

## 1. Bead thesis

vb-pcu4h closes audit finding P1 (`"Pending-action recovery test only checks steps_started count and can pass if pending actions are dropped."`) by replacing three `Vec<RecoveredPendingAction>::iter().any(...)` fuzzy predicates and one `matches!(seed, Ok(_) if ...)` silent-err outer pattern in `crates/vb_storage/src/recovery/replay/summary/tests.rs` with `assert_eq!` on the whole `Vec<RecoveredPendingAction>` (derived `PartialEq, Eq` from `crates/vb_storage/src/recovery/types.rs:644-650`).

The fix is a test-assertion-strength uplift. **No production code is edited.** The recovery reducer at `crates/vb_storage/src/recovery/replay/summary/derive.rs:69-83`, the accumulator at `crates/vb_storage/src/recovery/replay/summary/accumulator.rs:35,68`, the sort-and-assemble site at `derive.rs:287-296`, the production struct definition at `types.rs:644-650`, the journal-backed alias at `recover.rs`, and every Verus production mirror under `verification/verus/production_inner/**` are read-only for this bead.

## 2. Risk-classification summary

Classified once per seed in `verifier-lane-decisions.jsonl`. Top-level shape:

| Risk class | Applies? | Backing seed | Closure mechanism |
|------------|----------|--------------|--------------------|
| temporal/state-machine | No | — | Recovery reducer is synchronous, deterministic, idempotent. No async, no cancellation, no timer. |
| Rust-local invariant | Yes | seed-001, seed-002, seed-003, seed-006 | `assert_eq!` on the whole `Vec<RecoveredPendingAction>` against constructed literal vec; derived `PartialEq, Eq` covers length + per-element field equality. |
| Bounded state | Yes | seed-001, seed-002, seed-003 | Single-event fixtures → single-element expected vec; bounded by reducer's deterministic HashSet→Vec pipeline. |
| Refinement / type-state | No | — | Type already refined via `#[derive(... PartialEq, Eq)]`; no refinement ladder in this bead. |
| Concurrency | No | — | Single-threaded test, no shared state, no atomics. |
| Unsafe / UB | No | — | No `unsafe`, no raw pointers, no FFI. |
| Untrusted input / hostile input | No | seed-008 (fuzz) | Test fixtures use Rust struct literals (`JournalEvent::ActionScheduled { ... }`); no parser, codec, or string-decoding boundary. |
| Dependency / supply chain | No | — | No new dependency; existing `cargo` and `moon` setups only. |
| Performance | No | — | Vec equality on length-1 is O(1) per element; prior `.any(...)` was also O(n) per element. No benchmark regression possible. |
| Release-critical gates | Yes | seed-007 | `scripts/check-production-inner-drift.sh` and `scripts/check-verus-production-binding.sh` run as closure gates (production-inner and STRONG `#[path]` mirror unchanged). |

### Subrisk tags (per `codebase-map.md` risk table)

- `derived-state-proxy` (Test A only): preserved by keeping `assert!(recovered.unsupported.pending_actions)` alongside the new vec-equality assertion.
- `len-drift`: closed by `Vec::eq`.
- `silent-err-passes` (Test A only): closed by replacing `matches!(Ok(_) if ...)` with `let recovered = seed.expect("…")`.
- `mutation-strength`: closed by element-wise `RecoveredPendingAction::eq` (both fields).
- `api-drift`: closed by drift gate.
- `hostile-input-not-applicable` (seed-008): closed by absence of parser/codec/bytes boundary.

## 3. Verifier profile decision

Per the bead's hard lanes (user prompt + contract):

| Verifier | Applicability | Rationale |
|----------|---------------|-----------|
| `cargo-test` (`proptest` excluded; `cargo test --lib` only) | required for seed-001, seed-002, seed-003, seed-005, seed-006; required for seed-004 only if SECONDARY uplift applied | Targeted `--nocapture` runs of the three PRIMARY tests prove the rewritten assertions; secondary uplift gated to `recovery_hydration_tests::pending_action_persisted_restart_via_appends_with_syncall`. |
| `source-lint` (`moon run :lint-src`) | required for every seed (and as a project-wide gate) | Zero-tolerance source lint is the project's non-negotiable gate. `cargo fmt --all -- --check` is folded into source-lint. |
| `proptest` (round-trip) | not_applicable | Single-element expected vecs are deterministic; proptest would generate synthetic `(ActionScheduled*)` event sequences whose Vec equality requires constructing the same expected vec that the reducer would produce — circular. The three fixtures already exhaust all single-event shapes (`ActionScheduled`, `ActionScheduledTicket`, multi-event preamble). |
| `verus` | not_applicable | No production-code edit; the existing STRONG `#[path]` mirror at `verification/verus/production_inner/replay_invariants_production.rs:253-256` is unchanged. Adding a new Verus obligation would require a new production-bound claim; the bead does not have one (drift gate covers the surface that already exists). |
| `kani` | not_applicable | No `#[kani::proof]` exists for `RecoveredPendingAction`; the bead's forbidden list forbids adding one. The struct is a synchronous `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` value object that the test exercises exhaustively via the three single-event fixtures. |
| `flux` | not_applicable | Existing Flux refinements target `UnsupportedRecoveryState::pending_actions` (a Bool field), not `RecoveredPendingAction`. The bead is forbidden from adding new Flux refinements. Drift gate covers mirror parity. |
| `fuzz` (hostile-input) | not_applicable | Test fixtures use Rust struct literals; no parser / codec / bytes boundary at the test surface. `fuzz/` has no `RecoveredPendingAction` target. |
| `loom` / `shuttle` | not_applicable | Synchronous reducer, no concurrency surface. |
| `miri` | not_applicable | No `unsafe`; no raw pointers; nothing for Miri to check. |

### Why no Verus/Flux/Kani additions

GOD RULE 4 (formal verification mandates) prohibits altering the mathematical contract or proof harness just to make a test turn green. The bead's audit finding is closed by a test-assertion edit; the **production contract is unchanged**. Adding a new Verus/Flux/Kani row would:

1. Require inventing a new production-bound claim (none exists for this bead).
2. Require either binding to a mirror or holding `ALLOWED_EXCEPTIONS`; both are unnecessary.
3. Drift the verifier-lane profile away from the "test-only" classification that the contract enforces (`delivery-scope.jsonl:17-19` lists verus/kani/flux as `not-required` and the master `AGENTS.md` forbids ad-hoc verifier mutations outside the bead's blast radius).

Therefore Verus/Flux/Kani are explicitly `not_applicable` and the closure surface is `cargo-test + source-lint + drift gates`.

## 4. Closure evidence commands

Captured per obligation in `proof-obligations.planned.jsonl`. Closure gates in addition:

- `cargo test -p vb_storage --lib -- --nocapture unresolved_action_marks_pending_action_recovery_unsupported action_scheduled_ticket_advances_max_slot_and_step_dimensions crash_after_schedule_then_recover_hydrates_resume_queue`
- (Optional) `cargo test -p vb_runtime --test recovery_hydration_tests -- --nocapture pending_action_persisted_restart_via_appends_with_syncall`
- `moon run :lint-src` (source lint zero tolerance)
- `cargo fmt --all -- --check` (style)
- `bash scripts/check-production-inner-drift.sh` (mirror drift)
- `bash scripts/check-verus-production-binding.sh` (binding gate unchanged)
- `moon ci` (canonical)

## 5. Forbidden shapes (binding on test-writer)

Per `contract.md#Forbidden-(anti-pattern)-shapes`:

- ANTI-001 — `Vec<RecoveredPendingAction>::iter().any(|entry| entry.step == X && entry.action == Y)` (the audit's fuzzy predicate).
- ANTI-002 — `matches!(seed, Ok(recovered) if <bool>)` outer pattern (silent-pass-prone).
- ANTI-003 — `seed.pending_actions.iter().find(|entry| entry.action == action).expect(...)` followed by split `assert_eq!(pending.step, ...)` (SECONDARY targets).
- ANTI-004 — Asserting only `unsupported.pending_actions` (single boolean).
- ANTI-005 — Asserting only `summary.steps_started` or `summary.actions_scheduled` (counter-only proxy).

The PLAN, not proof-writer or implementation, is forbidden from removing any of these forbidden shapes silently. The plan's obligations all derive from `assert_eq!(<vec>, vec![RecoveredPendingAction { step, action }])`, never from a forbidden anti-pattern.

## 6. Type-binding at the test boundary (binding on test-writer)

The struct literal `RecoveredPendingAction { step: ..., action: ... }` must be reachable inside `crates/vb_storage/src/recovery/replay/summary/tests.rs`. The current import block at `tests.rs:1-9` includes `use crate::recovery::replay::summary::*;` but `summary/mod.rs` does **not** `pub use RecoveredPendingAction` (the glob pulls in `apply::*` and `derive::*` re-exports only — see `summary/mod.rs:36-42`). The type is reachable via `crate::recovery::RecoveredPendingAction` (re-exported at `recovery/mod.rs:42`) or via a new `use crate::recovery::RecoveredPendingAction;` line. **Modification of `summary/mod.rs` is out-of-bead** (`delivery-scope.jsonl:4`); the fix is bounded to the `tests.rs` test file only. The plan therefore *recommends* adding `use crate::recovery::RecoveredPendingAction;` to the test file's import block, before the existing `use vb_core::{...}` line.

## 7. State-4 obligations count

The contract mandates **2-3 obligations**. The plan emits **3 obligations** (one per PRIMARY test), plus one optional FOURTH obligation that activates only if the SECONDARY uplift patch is applied by `test-planner`/`holzman-rust`. Drift and binding gates are folded into the third obligation's `expected_evidence` rather than bloating into separate obligations.

| Obligation | Seed | Required? | Behavior-affecting? |
|------------|------|-----------|---------------------|
| PO-VBPCU4H-001 | seed-001 (Test A) | yes | false |
| PO-VBPCU4H-002 | seed-002 (Test B) | yes | false |
| PO-VBPCU4H-003 | seed-003 (Test C) | yes | false |
| PO-VBPCU4H-004 | seed-004 (SECONDARY) | optional | false |

## 8. Waiver posture

`waiver-candidates.jsonl` is empty. The bead is closed by `cargo-test` + `source-lint` evidence; there is no behavior-affecting waiver candidate (which would be fatal per the proof-planner EARS rules).

## 9. Hand-off

- **State 4b** — `proof-plan-reviewer` dispositions each lane decision; their `verifier-lane-review.jsonl` and `proof-plan-review.md` are owned by that agent.
- **State 5** — `proof-writer` authors the test edits. No Verus/Flux/Kani artifacts (none required).
- **State 7** — `proof-to-implementation` produces the proof-to-impl bridge; this planner provides `proof-to-implementation-input.md` is **out of scope for this bead's 7-artifact contract** and is therefore omitted (the user explicitly enumerated 7 artifacts).
- **State 9** — `test-writer`/`holzman-rust` apply the test edits.
- **State 12** — `formal-verifier` runs the obligations and closes the ledger.

No claim of PASS, APPROVED, or VERIFIED. Plan only.
