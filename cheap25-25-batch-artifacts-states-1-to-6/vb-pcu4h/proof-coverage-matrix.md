# proof-coverage-matrix.md — vb-pcu4h

- bead_id: vb-pcu4h
- planner_state: 4
- schema_version: proof-coverage-matrix/v1
- produced_by: proof-planner (State 4)
- planner_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`

## 1. Purpose

Maps every contract-clause-bound requirement from `contract.md` and every `proof-seed/v1` row from `proof-seeds.jsonl` to the obligations planned in `proof-obligations.planned.jsonl` and the verifier-lane decisions in `verifier-lane-decisions.jsonl`. The matrix is the single artifact that downstream reviewers and the black-hat reviewer use to confirm "every clause has a planned obligation, every obligation has a required command, every required command has expected evidence."

## 2. Atomic requirement table

| REQ-id | Contract clause | Domain claim (1-line summary) | Atomic-claim-target (path::lines/symbol) | Risk tags |
|--------|-----------------|--------------------------------|-------------------------------------------|-----------|
| REQ-pending-actions-vec-equality-A | POST-001 | After Test A recovery, the recovered `pending_actions` Vec equals exactly `vec![RecoveredPendingAction { step: StepIdx::new(3), action: ActionId::new(9) }]`. | `crates/vb_storage/src/recovery/replay/summary/tests.rs:449-453` :: assertion under repair in `unresolved_action_marks_pending_action_recovery_unsupported` | len-drift; mutation-strength; derived-state-proxy |
| REQ-pending-actions-vec-equality-B | POST-002 | After Test B recovery, the recovered `pending_actions` Vec equals exactly `vec![RecoveredPendingAction { step: StepIdx::new(5), action: ActionId::new(11) }]`. | `crates/vb_storage/src/recovery/replay/summary/tests.rs:666-671` :: assertion under repair in `action_scheduled_ticket_advances_max_slot_and_step_dimensions` | len-drift; mutation-strength |
| REQ-pending-actions-vec-equality-C | POST-003 | After Test C recovery, the recovered `pending_actions` Vec equals exactly `vec![RecoveredPendingAction { step: StepIdx::new(6), action: ActionId::new(17) }]`. | `crates/vb_storage/src/recovery/replay/summary/tests.rs:787-792` :: assertion under repair in `crash_after_schedule_then_recover_hydrates_resume_queue` | len-drift; mutation-strength |
| REQ-pending-actions-secondary-uplift | scope-secondaries | SECONDARY targets in `recovery_hydration_tests.rs:1899-1905` and `:2031-2037` should be uplifted from `.iter().find(...)` to struct-level `assert_eq!` on whole `Vec<RecoveredPendingAction>`. | `crates/vb_runtime/tests/recovery_hydration_tests.rs:1856-1946` (D) and `~:2027-2037` (E) | len-checked-but-fuzzy-entry-match; derived-state-proxy |
| REQ-recovery-err-panic-on-err | error-taxonomy (Layer 1) | Test A's outer `matches!(Ok(_) if …)` is replaced by `let recovered = seed.expect("…")` so any `Err(_)` panics with named context; no silent-pass via a non-panicking Err arm. | `crates/vb_storage/src/recovery/replay/summary/tests.rs:449-453` :: outer pattern at Test A | silent-err-passes |
| REQ-unsupported-flag-preserved-A | INV-002 | Test A retains `assert!(recovered.unsupported.pending_actions)` alongside the new Vec-equality assertion. | `crates/vb_storage/src/recovery/replay/summary/tests.rs:449-453` (Test A, preserved line) | derived-state-proxy |
| REQ-mirror-drift-gate | drift-gates / INV-004, INV-005 | Verus mirror at `verification/verus/production_inner/replay_invariants_production.rs:253-256` matches production; STRONG `#[path]` binding remains. | `verification/verus/production_inner/replay_invariants_production.rs:253-256` :: mirror; `scripts/check-production-inner-drift.sh`, `scripts/check-verus-production-binding.sh` | api-drift |
| REQ-recovery-pure-no-hostile-input | scope | Test fixtures use Rust struct literals; no parser/codec/bytes boundary; fuzz hostile-input is out of scope. | `crates/vb_storage/src/recovery/replay/summary/tests.rs` :: fixture surface | hostile-input-not-applicable |

## 3. Seed × requirement × obligation × verifier table

Format: rows = seeds (1-8); columns = obligation-row-id from `proof-obligations.planned.jsonl`, requested verifiers.

| Seed-id | REQ-id | Obligation-row | cargo-test | source-lint | drift-gate | Applicable? (with concrete refs) |
|---------|--------|----------------|------------|-------------|------------|------------------------------------|
| vb-pcu4h-seed-001 | REQ-pending-actions-vec-equality-A | PO-VBPCU4H-001 | required | required | folded | yes (per `proof-obligations.planned.jsonl#PO-VBPCU4H-001`) |
| vb-pcu4h-seed-002 | REQ-pending-actions-vec-equality-B | PO-VBPCU4H-002 | required | required | folded | yes (per `proof-obligations.planned.jsonl#PO-VBPCU4H-002`) |
| vb-pcu4h-seed-003 | REQ-pending-actions-vec-equality-C | PO-VBPCU4H-003 | required | required | folded | yes (per `proof-obligations.planned.jsonl#PO-VBPCU4H-003`) |
| vb-pcu4h-seed-004 | REQ-pending-actions-secondary-uplift | PO-VBPCU4H-004 (optional) | required_if_applied | required_if_applied | folded | optional; if test-planner/holzman-rust decides to apply, second source-lint is run |
| vb-pcu4h-seed-005 | REQ-recovery-err-panic-on-err | (folded into PO-VBPCU4H-001) | folded into PO-001 cargo-test | folded into PO-001 source-lint | folded | covered by PO-VBPCU4H-001 same evidence |
| vb-pcu4h-seed-006 | REQ-unsupported-flag-preserved-A | (folded into PO-VBPCU4H-001) | folded into PO-001 cargo-test | folded into PO-001 source-lint | folded | covered by PO-VBPCU4H-001 same evidence |
| vb-pcu4h-seed-007 | REQ-mirror-drift-gate | (folded as PO-001/2/3 source-lint pre-flight) | — | folded source-lint | required | covered by drift-gate; serves pre-flight for all PRIMARY obligations |
| vb-pcu4h-seed-008 | REQ-recovery-pure-no-hostile-input | (folded as PO-001/2/3 source-lint pre-flight) | — | folded source-lint | — | covered by source-lint; fuzz/loom/Miri all marked not_applicable with concrete refs |

## 4. Forbidden-shape anti-coverage

Forbiddens from `contract.md#Forbidden` and their handling under the planned obligations:

| Anti-shape | Mechanism that catches it | Planned-obligation coverage |
|------------|---------------------------|-----------------------------|
| ANTI-001 (`iter().any`) | Vec `PartialEq` covers length + per-element | PO-VBPCU4H-001/002/003 |
| ANTI-002 (`matches!(Ok(_) if ...)`) | `.expect("...")` panic-on-Err | PO-VBPCU4H-001 fold (seed-005) |
| ANTI-003 (`.find` then split `assert_eq!`) | Whole-vec `assert_eq!` | PO-VBPCU4H-004 (optional) |
| ANTI-004 (assert only `unsupported.pending_actions`) | Vec-equality paired with preserved boolean | PO-VBPCU4H-001 fold (seed-006) |
| ANTI-005 (assert only `summary.steps_started`) | Counter-only tests are out of bead scope; flagged for contract review in `codebase-map.md#UNKNOWN` | Not in-claim; cross-referenced |

Each row in `verifier-lane-decisions.jsonl` carrying `applicability=not_applicable` for default-profile verifiers cites concrete path-or-symbol refs in `non_applicability_evidence_refs` — these satisfy the EARS rule "default-profile verifiers with applicability: not_applicable need non_applicability_evidence_refs with concrete artifact hashes" (the artifact hashes at this State are the path references; SHA-256 will be captured by the formal-verifier at closure).

## 5. Coverage gaps explicitly closed by the test edit

| Audit failure mode | Description | Caught by planned obligation |
|--------------------|-------------|--------------------------------|
| FAIL-1 silent-Err mode | `matches!(Ok(_) if …)` outer pattern allows non-panicking error read | PO-VBPCU4H-001 with `.expect("…")` rewrite |
| FAIL-2 drop-all pending actions | `pending_actions` empty | PO-VBPCU4H-001/002/003 with `Vec::eq` against length-1 literal |
| FAIL-3 phantom-duplicate | `pending_actions` length > 1 | PO-VBPCU4H-001/002/003 with `Vec::eq` |
| FAIL-4 field-drift | `step` or `action` mismatch | PO-VBPCU4H-001/002/003 with element-wise `RecoveredPendingAction::eq` |
| FAIL-5 unsupported-flag drift (Test A) | Boolean false | PO-VBPCU4H-001 with preserved `assert!(unsupported.pending_actions)` |

## 6. Coverage not closed (intentional / scope)

- `recovery_unit_tests.rs:314-351` hand-built seed test — out of bead scope per user prompt's "must NOT modify recovery_unit_tests.rs:314-351 hand-built test." Risk: `derived-state-proxy`. Closure: a flag-of-record in `delivery-scope.jsonl` (kind=risk, name=derived-state-proxy, status=open) for follow-up beads.
- SECONDARY targets in `recovery_hydration_tests.rs` — optional; applied only if test-planner/holzman-rust decides in the same patch.
- `Reproducer`/fuzz coverage for `RecoveredPendingAction` — irrelevant; the struct is a value object with no parser/codec surface.
- Property-based round-trip — folded as not_applicable (circular shape: expected Vec requires the same reducer output).

## 7. State transitions for closure

- State 4 → State 4b: this matrix + `verifier-lane-decisions.jsonl` + `proof-strategy.md` are reviewed by `proof-plan-reviewer`.
- State 4b → State 5: `proof-writer` is invoked only with the test edit (no model/proof/harness).
- State 5 → State 7: `proof-to-implementation` emits a minimal bridge because no proof-side claims cross-impl; the obligation rows' `artifact` paths suffice.
- State 7 → State 9: `holzman-rust` / `test-writer` apply the test edits.
- State 9 → State 12: `formal-verifier` runs the planned obligations and produces the closure ledger.

No claim of coverage closure. State 12 is the only authority for "covered + verified."

## 8. Handoff digest

- **Black-hat reviewer (State 11)** receives: this matrix, `verifier-lane-decisions.jsonl`, `proof-obligations.planned.jsonl`, `proof-strategy.md`, `trusted-base-plan.md`, `waiver-candidates.jsonl` (empty), and the closure-evidence directory `.beads/vb-pcu4h/evidence/`.
- **Trusted base** is captured in `trusted-base-plan.md`.
- **Behavior-affecting** posture: false for every obligation; no waiver candidate.

End of matrix.
