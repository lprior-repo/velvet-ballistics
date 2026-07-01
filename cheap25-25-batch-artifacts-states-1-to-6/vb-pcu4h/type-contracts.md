# Type Contracts — vb-pcu4h

- bead_id: vb-pcu4h
- artifact_owner: rust-contract
- scope: type-level contracts that the test edits MUST respect.
- canonical_struct: `RecoveredPendingAction` at `crates/vb_storage/src/recovery/types.rs:644-650` (UNCHANGED by this bead).

## Production struct under assertion

```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredPendingAction {
    pub step: StepIdx,
    pub action: ActionId,
}
```

- `step: StepIdx` — durable newtype over `u32` (`vb_core::StepIdx`); `StepIdx::new(N)` is the canonical constructor; `StepIdx::ZERO` exists for the ZERO literal.
- `action: ActionId` — durable newtype over `u32` (`vb_core::ActionId`); `ActionId::new(N)` is the canonical constructor.
- Derives: `Debug, Clone, Copy, PartialEq, Eq` — guarantees that `assert_eq!(entry_a, entry_b)` compares both fields and that `assert_eq!(vec![entry_a], vec![entry_b])` compares length and per-element equality.
- No extra fields (e.g., no `attempt`, no `seq`, no `idempotency_key`, no `output_slot`). Adding any of these would invalidate this contract and require a Verus mirror update.

## Vec container

- `RecoveryFrameSeed::pending_actions: Vec<RecoveredPendingAction>` — public field; the recovery reducer writes it via `recovered_pending_actions` (`derive.rs:287-296`) which sorts ascending by `(step, action)` so canonical-order equality is decidable from `PartialEq` alone.
- `Vec<T>::PartialEq` — length-equal AND element-wise equal; this is the single comparison that the bead's fix uses. No separate `assert_eq!(len, 1)` is required *for the bead's purpose* (the Vec `PartialEq` covers it), but Test A retains the explicit length expectation via the constructed literal vec.

## Smart constructors / type-driven inputs (already in production)

- `StepIdx::new(u32) -> StepIdx` — infallible newtype wrap; cannot fail.
- `ActionId::new(u32) -> ActionId` — infallible newtype wrap; cannot fail.
- `RecoveredPendingAction { step, action }` — public-field struct literal; only legal at the test boundary or in production code that already holds validated `StepIdx` / `ActionId`. No constructor is required at the test boundary.

## Forbidden type shapes (anti-patterns that the bead explicitly removes)

- Anti-1 — `Vec<RecoveredPendingAction>::iter().any(|entry| entry.step == X && entry.action == Y)` — fuzzy predicate; cannot distinguish "expected entry present" from "expected entry present plus a phantom duplicate" or "vec is empty and the unsupported flag covers it". Tests A/B/C replace this anti-pattern.
- Anti-2 — `matches!(seed, Ok(recovered) if <inner>)` — when the outer `assert!` is a bool from `matches!`, an `Err(_)` arm causes the matches! to return false, the outer assert to see a Bool, and the test to PASS without exercising the recovery code. Test A's outer pattern must be replaced by `let recovered = seed.expect("…")` first.
- Anti-3 — `seed.pending_actions.iter().find(|entry| entry.action == action).expect(...)` followed by `assert_eq!(pending.step, StepIdx::new(2))` — splits a single vec-equal assertion into two non-equivalent halves (length checked elsewhere, field split into a find + scalar). SECONDARY targets D and E fall in this shape; contract RECOMMENDS uplifting to Vec-equality in the same patch.

## Required type-level behavior (preserved by replacement)

- TC-1 — PartialEq struct-equality. `assert_eq!(entry_actual, RecoveredPendingAction { step: S, action: A })` compares both fields and panics with a structured diff on mismatch (Rust's default `assert_eq!` formatter prints both sides including fields). This is the audit's "field-level assertions covering every PendingAction field" surface.
- TC-2 — Vec PartialEq. `assert_eq!(vec_actual, vec![RecoveredPendingAction { step: S, action: A }])` first compares length and panics with `assertion `left == right` failed: …left.len() != right.len()`` if length drifts; if lengths match it compares element-wise. The single `assert_eq!` covers all three audit failure modes (drop-all, phantom-duplicate, field-drift).
- TC-3 — `expect(&str)` panic-on-Err. `seed.expect("seed recovery must succeed for single ActionScheduled")` panics with the message + the `Err(_)` Debug payload; the test fails closed instead of passing silently.
- TC-4 — `unsupported.pending_actions` boolean (preserved). The bead does NOT remove the boolean assertion in Test A — only ADDS the Vec-equality assertion alongside it. The unsupported flag exercises a separate derivation (the accumulator's empty-set check) and must remain asserted.

## Type contract for `journal_recover_runtime_frame_seed` (alias)

The SECONDARY targets use `recover_runtime_frame_seed(&journal, run)` (`crates/vb_storage/src/recovery/recover.rs`) which is a journal-backed alias of the same reducer. It returns the same `RecoveryFrameSeed` type and is therefore subject to the same `pending_actions` Vec-equality contract.

## Type contract for `summary.steps_started`

The audit phrase "only checks steps_started count" cannot be mapped literally onto any test currently in the repository (no PRIMARY or SECONDARY target asserts only `summary.steps_started` for the pending-action flow). The hand-built-seed test at `recovery_unit_tests.rs:323` builds `steps_started: 1` but then asserts derived state (`state.pending_actions`, `state.pending_asks`, `state.unsupported_reason()`) — not `steps_started` itself. The contract therefore REJECTS the literal "steps_started-only" reading and maps the audit phrase onto "counter-only / boolean-only / fuzzy-iter-only" assertions over the seed.

## Verus mirror drift-gate contract

- Drift surface: `verification/verus/production_inner/replay_invariants_production.rs:253-256` mirrors `RecoveredPendingAction` byte-for-byte. Adding any field to the production struct requires mirroring it here or the drift gate (`scripts/check-production-inner-drift.sh`) fails.
- Drift surface: `verification/verus/production_inner/recovery_verification_production.rs:25,45` references `RecoveredPendingAction` in provenance comments; no field changes there.
- Binding gate: `scripts/check-verus-production-binding.sh` — STRONG binding via `#[path = "..."]` to production source. No edit needed for this bead; runs as a gate only.

## Replacement contract (binding for test-writer)

For each PRIMARY target the replacement MUST satisfy:

1. `let recovered = <seed expression>.expect("<panic message>");` — fails closed on `Err(_)` with a context message that names the fixture event.
2. `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { step: <StepIdx literal>, action: <ActionId literal> }]);` — single Vec-equality assertion; covers length and per-element fields.
3. (Test A only) `assert!(recovered.unsupported.pending_actions);` — boolean preservation.
4. (Tests B and C only) preserve existing `slot_count` and `step_count` assertions and any step-state assertions untouched.
5. No re-introduction of `.any(...)`, `.find(...)`, or `matches!(Ok(_) if ...)` predicates over `pending_actions`.

For SECONDARY targets D and E (recommended uplift, optional in this patch):

1. Replace `.find(|entry| entry.action == action).expect("…")` + `assert_eq!(pending.step, StepIdx::new(N))` + `assert_eq!(seed.pending_actions.len(), 1)` with a single `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(N), action }]);`.