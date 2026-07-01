# Workflow Model — vb-7akm0 Lint-Suppression Audit

| Field | Value |
|---|
| bead_id | vb-7akm0 |
| state | 3 (rust-contract) |
| skill | rust-contract |
| captured_at | 2026-07-01T16:04Z |
| upstream_artifacts | domain-model.md, type-contracts.md |

## 0. Scope

This file models the workflow that takes the workspace from "25 lint suppressions across 25 files" to "0 lint suppressions, all `pub` items reachable". The workflow is the bridge between the type contract (which describes WHAT must be true) and the implementation (which makes it true). It is a workflow SPECIFICATION; the implementation is the responsibility of downstream skills (proof-planner, holzman-rust, formal-verifier, black-hat-reviewer).

The workflow has TWO sequential phases:
1. **Audit phase** — decide the treatment per file (already captured in `delivery-scope.jsonl`).
2. **Execution phase** — apply the treatment and re-run the lint gate.

Because this bead is behavior-preserving (no semantic change to the program), the workflow is shorter than the closure substrate. The terminal state is "lint-src exits 0".

## 1. Actors

| Actor | Role | Privileged commands |
|---|---|---|
| `Auditor` | The owner at State 4 (proof-planner). Categorises each suppression, sets `recommended_treatment`. | `Categorise(suppression, category)`, `RecommendTreatment(suppression, treatment)` |
| `DecisionMaker` | A user/architect (or pre-registered policy). Resolves Category G `DecisionRequired` rows. | `Decide(category, recommendation)` |
| `Implementer` | The owner at State 11 (holzman-rust). Applies the treatment to source files. | `DeleteAllow(suppression)`, `NarrowVisibility(suppression, new_visibility)`, `RegisterOrphanTest(test_path)`, `RetireOrphanTest(test_path)` |
| `Verifier` | A moon ci runner. Re-runs `lint-src` and `cargo test --workspace`. | `RunLintSrc(gate_run_id)`, `RunCargoTest(scope)` |
| `Approver` | The owner at State 13 (black-hat-reviewer). Inspects the diff and confirms no semantic change. | `Approve(suppression)` |

## 2. State Machine

```text
                   ┌────────────────────────────┐
                   │                            │
                   ▼                            │
             ┌─────────────┐                    │
             │  Audited    │                    │
             │  (row in    │                    │
             │ delivery-   │                    │
             │  scope)     │                    │
             └────┬────────┘                    │
                  │ Decide (Category G only)   │
                  ▼                             │
             ┌─────────────┐                    │
             │  Decided    │                    │
             └────┬────────┘                    │
                  │ ApplyTreatment              │
                  ▼                             │
             ┌──────────────┐                   │
             │  Applied     │                   │
             └────┬─────────┘                   │
                  │ RunLintSrc (exit 0)         │
                  ▼                             │
             ┌──────────────┐                   │
             │ LintCleared  │ ──────────┐        │
             └────┬─────────┘           │        │
                  │ RunCargoTest         │        │
                  ▼                      │        │
             ┌──────────────┐           │        │
             │ TestsGreen   │           │        │
             └────┬─────────┘           │        │
                  │ Approve             │        │
                  ▼                      │        │
             ┌──────────────┐           │        │
             │   Closed     │           │        │
             └──────────────┘           │        │
                                        │        │
                  (RunLintSrc exit ≠ 0)─┘        │
                  back to Applied                │
```

## 3. Transitions

Each transition is a tuple `(from_state, command, to_state, guard, postcondition, emits_event)`.

| From | Command | To | Guard | Postcondition | Emits |
|---|---|---|---|---|---|
| (none) | `Categorise` | `Audited` | `Suppression::new` succeeds | All 25 rows present | `Audited` |
| `Audited` | `Decide` | `Decided` | `category == OrphanTestDecision` AND recommendation is set | recommendation recorded in audit log | `Decided` |
| `Audited` | `ApplyTreatment` | `Decided` | `category != OrphanTestDecision` | non-decision rows skip `Decided` | (silent transition) |
| `Decided` | `ApplyTreatment` | `Applied` | source file modified per `treatment`; `suppression_line` no longer exists | source change recorded in `git diff` | `Applied` |
| `Applied` | `RunLintSrc` | `LintCleared` | `cargo clippy --workspace --lib --bins --examples --all-features` exits 0 | `lint-src` exit 0 captured in raw log | `LintCleared` |
| `Applied` | `RunLintSrc` | `Applied` (no-op) | `lint-src` exits non-zero with a new `unreachable_pub` label | raw log captured; `attempt_id` incremented | `LintSurfaced` |
| `LintCleared` | `RunCargoTest` | `TestsGreen` | `cargo test --workspace` exits 0 | test suite green | `TestCompiles` |
| `LintCleared` | `RunCargoTest` | `Applied` (revert) | a previously-passing test now fails because of a visibility narrowing | revert the narrowing OR keep narrowing and fix the test | `TestBreaks` |
| `TestsGreen` | `Approve` | `Closed` | black-hat review confirms no semantic change | bead is closable | `Closed` |

Forbidden transitions:
- `Audited` → `Closed` (must clear the lint first).
- `Applied` → `Closed` (must clear the lint AND re-run tests).
- `Audited` → `Decided` if `category != OrphanTestDecision` (only G requires a decision).
- Any state → `Closed` if any `Suppression.behavior_affecting == true` (this bead is behavior-preserving).

## 4. State Invariants (Behavioral Contracts)

| State | Invariant |
|---|---|
| `Audited` | All 25 rows parse via `parse_suppression_from_delivery_scope`; `category` and `kind` are consistent; `behavior_affecting == false` for every row. |
| `Decided` | Every Category G row has a `recommendation` set. |
| `Applied` | For each `Suppression`, the source file at `file_path` no longer contains `#[allow(unreachable_pub)]` at `suppression_line`; OR (for Category B/C/D) the `pub_items_at_file_scope` have been narrowed per `treatment`. |
| `LintCleared` | `moon run :lint-src` exits 0; raw log captured in `.evidence/lint-src/<run_id>/exit-code.txt`. |
| `TestsGreen` | `cargo test --workspace` exits 0; specifically `cargo test -p vb_validate --lib`, `cargo test -p vb_cli --lib`, `cargo test --workspace --tests`. |
| `Closed` | All of the above, plus black-hat review approval. |

## 5. Category G Decision Path

Category G contains the most decision-laden rows: `commands_diff.rs`, `commands_incident.rs`. The decision workflow:

```text
                  ┌─────────────────────┐
                  │ OrphanTestDecision  │
                  └──────────┬──────────┘
                             │ Decide
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        RetireOrphan    RegisterOrphan   NarrowModule
              │              │              │
              ▼              ▼              ▼
       retire test,   register test,  change pub mod
       narrow CLI     delete allow    to pub(crate)
       items to                          (breaks public
       private/                          CLI surface;
       pub(crate)                         NOT recommended)
```

Per `domain-model.md §5`, the default recommendation is `RetireOrphan`. The decision MUST be recorded in the audit log before `ApplyTreatment` is allowed to fire on a Category G row.

## 6. Per-Category Workflow Notes (Bead-Specific)

| Category | File count | Expected trajectory | Notes |
|---|---|---|---|
| A (Vestigial) | 4 (xtask/main.rs, diag_tests.rs, schema_tests.rs, fact_table.rs) | `Audited → Applied → LintCleared → TestsGreen → Closed` | One-line deletion per file; `fact_table.rs` contains only `pub(crate)` items so the lint does not actually fire there. |
| B (Gate internal) | 6 (gate_07..13) + 2 (taint_prop.rs, type_check.rs treated as B) | `Audited → Applied → LintCleared → TestsGreen → Closed` | Convert `pub fn` → `fn`; consumers via `crate::gate_XX::name` and `use super::*` still resolve. |
| C (Taint/type/secret_leak) | 3 (taint_prop, type_check, secret_leak) | same as B | Same fix as B; canonical exports in `type_taint.rs` remain `pub`. |
| D (Schema support) | 4 (type_sigs, schema_doc, schema_id, schema_fields) | `Audited → Applied → LintCleared → TestsGreen → Closed` | Convert `pub` → `pub(crate)`; cross-test-module access preserved. |
| E (Diag module) | 3 (diag_codes, diag_convert, diag_render) | `Audited → Decided (per-file) → Applied → LintCleared → TestsGreen → Closed` | Mixed: `diag_convert.rs` and `diag_render.rs` get DeleteAllow; `diag_codes.rs` may be DeleteAllow (if keeping `pub`) or PubToPubCrate (if narrowing). |
| F (Diagnostic re-export) | 1 (diagnostic.rs) | `Audited → Applied → LintCleared → TestsGreen → Closed` | One-line DeleteAllow; the two re-exports are externally reachable. |
| G (Orphan test) | 3 (commands_diff, commands_incident, lifecycle) | `Audited → Decided → Applied → LintCleared → TestsGreen → Closed` | Default: retire orphan + narrow CLI items. `lifecycle.rs` is DeleteAllow (create_run_header is externally reachable). |

## 7. Concurrency and Ordering

- The 25 rows are independent; `ApplyTreatment` may execute on each row independently.
- Per-file ordering: `ApplyTreatment(file)` MUST NOT run before `Decide(file)` for Category G rows.
- Verification ordering: `RunLintSrc` MUST NOT run until all 25 `ApplyTreatment` calls have completed.
- Test ordering: `RunCargoTest` MUST NOT run until `RunLintSrc` has exited 0 (otherwise we might miss the lint failure).

## 8. Timeout and Stall Detection

The implementation MUST NOT stall on any single file. If `RunLintSrc` exits non-zero after 3 attempts, escalate to the bead's `priority = 0` queue and surface the new `unreachable_pub` label.

## 9. Audit Trail

Every transition produces an event in `.beads/vb-7akm0/audit-log.jsonl`:

| Field | Type | Constraint |
|---|---|---|
| `event_type` | `EventType` | one of `Audited`, `Decided`, `Applied`, `LintSurfaced`, `LintCleared`, `TestCompiles`, `TestBreaks`, `Closed` |
| `suppression_id` | string | `file_path:suppression_line` |
| `at` | `Iso8601` | UTC `Z` |
| `details` | string | event-specific payload |

The audit trail is append-only. No event is ever deleted or rewritten.

## 10. Open Workflow Questions

1. Whether to execute the 25 changes in one commit or 6 commits (one per category). Recommendation: 6 commits, one per category, to make black-hat review tractable.
2. Whether to run `RunLintSrc` after each category's commit or only after all 6 commits. Recommendation: after each category, so a failure on Category B doesn't mask a failure on Category G.
3. Whether Category G requires a separate human approval step before `Decide` is allowed. Recommendation: yes; the default recommendation is retire, but a human MUST ack before retire is executed.
4. Whether the production-binding audit (Question 4 in domain-model §5) is a hard pre-condition for `ApplyTreatment` on `commands_incident.rs`. Recommendation: yes; holzman-rust must run `grep IncidentReport verification/verus/production_inner/` and confirm no direct import.

End of workflow-model.md.