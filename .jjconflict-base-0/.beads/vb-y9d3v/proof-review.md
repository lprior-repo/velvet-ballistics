# Proof Review — vb-y9d3v ActionTicket Generation Fence

reviewer_skill: proof-reviewer
reviewer_invocation_id: vb-y9d3v-state6-proof-reviewer-attempt1
review_state: 6
proof_writer_invocation_id: vb-y9d3v-state5-proof-writer-attempt1
review_date: 2026-05-29
review_round: Attempt 1
workdir: /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-y9d3v

STATUS: REJECTED

## Review Provenance

| Field | Value |
|---|---|
| reviewer_skill | proof-reviewer |
| reviewer_invocation_id | vb-y9d3v-state6-proof-reviewer-attempt1 |
| review_state | 6 |
| proof_writer_invocation_id | vb-y9d3v-state5-proof-writer-attempt1 |
| review_date | 2026-05-29 |
| review_round | Attempt 1 |
| workdir | /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-y9d3v |

## Reviewed Artifacts

| Artifact | Path | Lines | Reviewer Verdict |
|---|---|---|---|
| Kani harnesses | crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs | 608 | Contains multiple vacuous harnesses; GOD RULE violations |
| Verus proofs | crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs | 465 | Tautological specs; disconnected from production; BLOCKED_TOOLING |
| Flux refinements | crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs | 281 | False invariants on production types; BLOCKED_TOOLING |
| proptest properties | crates/vb_runtime/src/verification/proptest/proptest_attempt_fence.rs | 600 | Hardcoded workflow structure; 14/14 PASS (raw evidence confirmed) |
| cargo-fuzz target | fuzz/fuzz_targets/fuzz_retry_codec.rs | 267 | Not executed (PENDING_FORMAL_EXECUTION) |
| Verifier wiring | crates/vb_runtime/src/verification/mod.rs | 34 | Structurally sound; cfg gating correct |
| Proof-writer report | .beads/vb-y9d3v/proof-writer-report.md | 142 | Inflated claims of production binding |
| Proof evidence | .beads/vb-y9d3v/proof-evidence.md | 79 | Insufficient for 20 of 41 BLOCKED obligations |
| Prior plan review | .beads/vb-y9d3v/proof-plan-review.md | 126 | 4 non-blocking findings from State 4 unresolved |

## Obligation Status Summary

| Obligations | Count | Raw Evidence | Reviewer Assessment |
|---|---|---|---|
| Kani (PO-0001–0037) | 10 | 1 smoke-verified (vacuous); 9 PENDING_FORMAL_EXECUTION | 6 harnesses substantively defective |
| Verus (PO-0002–0038) | 10 | BLOCKED_TOOLING; artifacts would fail even with tools | ALL 10 REJECTED — tautologies and disconnected specs |
| Flux (PO-0003–0039) | 10 | BLOCKED_TOOLING; artifacts contain false invariants | ALL 10 REJECTED — false `invariant(attempt > 0)` |
| proptest (PO-0004–0040) | 10 | 14 tests PASS (`cargo test` raw output confirmed) | Conditional PASS; hardcoded workflow limits coverage |
| cargo-fuzz (PO-0041) | 1 | PENDING_FORMAL_EXECUTION; no raw fuzzer output | Cannot approve without execution |
| **Total** | **41** | | **31+ obligations substantively REJECTED** |

---

## Blocking Findings (CRITICAL)

### F-vb-y9d3v-S6-0001 — Verus Tautology: `spec_action_fence_correctness` Always Returns `true`

- **Severity**: CRITICAL (GOD RULE 2)
- **Artifact**: `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs:355–374`
- **Obligations**: PO-vb-y9d3v-0002, PO-0006, PO-0010, PO-0014, PO-0018, PO-0022, PO-0026, PO-0030, PO-0034, PO-0038
- **Finding**: The `spec_action_fence_correctness` closed spec function returns `true` in EVERY branch. Lines 361–373 each produce `true` regardless of input values. The proof `proof_action_fence_exhaustive` (line 377–387) uses `by(compute)` to assert this always-true spec — which is vacuously true for ALL `u16` and `bool` inputs. This proves nothing about the ActionTicket fence behavior.
- **Evidence**:
  ```
  // Line 361: if !step_exists { true }
  // Line 365: else if incoming == 0 || capacity == 0 || incoming > capacity { true }
  // Line 368: else if incoming < current { true }
  // Line 372: else { true }
  ```
- **Required fix**: Rewrite `spec_action_fence_correctness` to encode actual behavioral contracts (e.g., stale attempt → `StaleAttempt` error, exact match → `Ok(())`). Bind the spec to the production `validate_ticket_attempt` exec fn via `requires/ensures` on `#[verifier::external_body]` declarations that carry non-trivial pre/postconditions. Replace `requires: true` on external_body declarations with actual bounds.

### F-vb-y9d3v-S6-0002 — Verus Tautology: `spec_single_terminal_event` Always Returns `true`

- **Severity**: CRITICAL (GOD RULE 2)
- **Artifact**: `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs:293–311`
- **Obligations**: PO-vb-y9d3v-0018
- **Finding**: `spec_single_terminal_event(is_terminal)` returns `true` unconditionally for both `true` and `false` inputs. The proof `proof_single_terminal_event_invariant` merely confirms this tautology via `by(compute)`.
- **Required fix**: Model the actual invariant: terminal runs cannot accept further completions. Encode that `validate_action_completion` must return an error for terminal runs.

### F-vb-y9d3v-S6-0003 — Verus Tautology: `spec_stale_completion_no_mutation` Always Returns `true`

- **Severity**: CRITICAL (GOD RULE 2)
- **Artifact**: `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs:260–286`
- **Obligations**: PO-vb-y9d3v-0014
- **Finding**: The spec function at lines 265–271 returns `true` for ALL input branches. Error case returns `true`. Success case returns `false` (inverted logic). The proof `proof_stale_completion_is_noop` confirms the trivial true case.
- **Required fix**: Model the no-mutation invariant properly: stale attempts produce errors AND the attempt counter is unchanged.

### F-vb-y9d3v-S6-0004 — Verus Disconnected from Production Code (GOD RULE 2)

- **Severity**: CRITICAL (GOD RULE 2)
- **Artifact**: `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs:399–463`
- **Obligations**: All 10 Verus obligations
- **Finding**: All three `#[verifier::external_body]` declarations have `requires: true` — accepting ALL inputs with no preconditions. The `ensures` clauses reference `spec_action_fence_correctness` (which always returns `true`). The production functions `validate_ticket_attempt`, `record_retry_attempt`, `normalize_scheduled_ticket` from `crates/vb_runtime/src/shard/helpers.rs` are NOT imported, NOT called, and their actual behavior is NOT modeled. The Verus proofs prove a self-defined model of a model, not the production Rust implementation. This is a textbook GOD RULE 2 violation.
- **Additionally**: BLOCKED_TOOLING — Verus not available. Even if Verus were installed, these artifacts would not constitute production-bound proofs.
- **Required fix**: Import and model the actual production types from `vb_core::action::ActionTicket` (with its `attempt: u16`, `capacity: u16`, `run`, `step` fields), the actual `RunState` from `vb_runtime::shard::types`, and the actual production helper functions. Wire ghost state tracking for the live per-step attempt counter. Use non-trivial `requires/ensures` that encode real behavioral contracts.

### F-vb-y9d3v-S6-0005 — Kani Vacuous Harness: `proof_typed_missing_run_error` Exercises No Production Code

- **Severity**: CRITICAL
- **Artifact**: `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs:313–333`
- **Obligations**: PO-vb-y9d3v-0021
- **Finding**: This harness creates two `RuntimeError` enum variants (`RuntimeError::RunNotFound`, `RuntimeError::InvalidActionCompletion`) and matches on them. It calls NO production function, NO `use crate::shard::helpers::*` function, and exercises NO runtime behavior. This is the ONLY harness that was smoke-verified (`VERIFICATION:- SUCCESSFUL`), giving false confidence. The harness proves only that the Rust compiler generated an enum correctly.
- **Evidence**: Raw Kani output shows 478 checks on standard library drop/pointer safety, ZERO checks on any `vb_runtime` or `vb_core` behavioral assertion.
- **Required fix**: Rewrite to construct a `RunState` and call `validate_action_completion` (or the production code path that checks for RunNotFound), then assert the exact error variant returned.

### F-vb-y9d3v-S6-0006 — Kani Vacuous Harness: `proof_single_terminal_event_invariant` Tests Rust Borrow Checker

- **Severity**: CRITICAL
- **Artifact**: `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs:284–307`
- **Obligations**: PO-vb-y9d3v-0017
- **Finding**: The harness clones `state.frame`, calls `validate_action_completion(&state, ticket)`, then asserts `state.frame == frame_before` on the error path. But `validate_action_completion` takes `&self` — the Rust type system GUARANTEES it cannot mutate `state`. This harness proves nothing about terminal events; it proves the Rust borrow checker works.
- **Required fix**: Test that after a completion/failure marks a run terminal, subsequent completions are rejected. This requires mutable state manipulation and successive calls.

### F-vb-y9d3v-S6-0007 — Kani Harness Tests Wrong Function for Stale Attempt Rejection

- **Severity**: CRITICAL
- **Artifact**: `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs:119–165`
- **Obligations**: PO-vb-y9d3v-0001
- **Finding**: The harness explicitly acknowledges at line 155 that `validate_ticket_attempt` is private and cannot be called. It tests `normalize_scheduled_ticket` instead — which, per the production code at `helpers.rs:106`, PROMOTES stale attempts (`let attempt = current.max(ticket.attempt).max(1)`) rather than rejecting them. Comment at line 150: "a lower attempt gets promoted, not rejected." The harness ends with `kani::cover!(true, "validate_ticket_attempt rejects stale attempts")` — a vacuous label with no assertion.
- **Required fix**: Make `validate_ticket_attempt` accessible to Kani harnesses (e.g., `#[cfg(kani)] pub` or a test-only re-export), then test the actual rejection logic. The production function at `helpers.rs:87–92` returns `StaleAttempt` when `ticket.attempt < current` — harness MUST verify this path.

### F-vb-y9d3v-S6-0008 — `kani::cover!(true, ...)` Misused as Proof Evidence

- **Severity**: CRITICAL (GOD RULE / Non-Vacuity)
- **Artifact**: `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs`, lines 164, 198, 320, 329, 380, 422, 524, 574, 604
- **Obligations**: All 10 Kani obligations
- **Finding**: Nine locations use `kani::cover!(true, "some label")`. This proves ONLY that the code path is reachable — it places no constraint on program behavior. The proof-reviewer skill explicitly rejects `cover!` used as proof. Per `references/non-vacuity-checks.md`: "Demand evidence that the verifier could fail."
- **Required fix**: Replace `kani::cover!` with actual `kani::assert` invariants that would fail if the production behavior were incorrect. Use `kani::cover!` only for reachability documentation, not as satisfaction of proof obligations.

### F-vb-y9d3v-S6-0009 — GOD RULE 1: Hardcoded Workflow Shapes in Kani and proptest

- **Severity**: CRITICAL (GOD RULE 1)
- **Artifact**: 
  - `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs:58–105` (`any_do_run_state`)
  - `crates/vb_runtime/src/verification/proptest/proptest_attempt_fence.rs:85–124` (`make_do_run_state`)
- **Obligations**: All 10 Kani + all 10 proptest obligations
- **Finding**: Both harness builders construct a single hardcoded Do-node workflow at `StepIdx::ZERO` with `ActionId::new(0)`, fixed `SlotIdx::new(0)`, and a one-element nodes array. GOD RULE 1: "Kani verification harnesses MUST NOT hardcode structural inputs like WorkflowParts or RunFrame with fixed dummy data." While individual ticket fields use `kani::any()`/strategy combinators, the workflow graph structure is entirely fixed. This means the harnesses never explore multi-step workflows, error handlers, WaitUntil/Ask nodes, or non-zero action IDs.
- **Required fix**: Implement `kani::Arbitrary` for `WorkflowParts` (or bounded structural generators) to explore valid workflow graphs with varying node counts, node kinds, and action IDs. For proptest, use strategy combinators that generate variable workflow structures.

### F-vb-y9d3v-S6-0010 — Flux False Invariant: `#[invariant(self.attempt > 0)]` on `ActionTicket`

- **Severity**: CRITICAL (GOD RULE 2)
- **Artifact**: `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs:29`
- **Obligations**: PO-vb-y9d3v-0003, PO-0007, PO-0011, PO-0015, PO-0019, PO-0023, PO-0027, PO-0031, PO-0035, PO-0039
- **Finding**: The `#[extern_spec]` for `ActionTicket` at line 29 declares `#[invariant(self.attempt > 0)]`. The production `vb_core::action::ActionTicket` struct carries `pub attempt: u16` — a public field with NO invariant. Zero-attempt tickets are valid in the production type (rejected at the boundary by `validate_ticket_attempt`, not by construction). This invariant would produce FALSE POSITIVES for any code that constructs or carries zero-attempt tickets. This is a GOD RULE 2 violation: the Flux refinement must be TRUE for the production type.
- **Additionally**: BLOCKED_TOOLING — `cargo-flux` not available. Even if installed, this invariant would trigger spurious violations.
- **Required fix**: Remove the false invariant. Instead, refine the VALIDATION FUNCTIONS (`validate_ticket_attempt`, `normalize_scheduled_ticket`) to ensure they reject zero-attempt tickets. The invariant belongs on the function postcondition, not the struct type.

---

## High Severity Findings

### F-vb-y9d3v-S6-0011 — 20 of 41 Obligations BLOCKED_TOOLING with No Compensating Evidence

- **Severity**: HIGH
- **Artifacts**: Verus (10 obligations), Flux (10 obligations)
- **Finding**: Both Verus and Flux are BLOCKED_TOOLING. The proof-writer report acknowledges this. No raw verifier output exists for ANY of these 20 obligations. The proof-reviewer skill states: "Approve only when every required proof obligation has non-vacuous artifact evidence or an explicit blocker." While the blocker is acknowledged, the ARTIFACTS THEMSELVES are so vacuous/incorrect (see findings S6-0001 through S6-0004, S6-0010) that they would not pass verification even with tools installed.
- **Required fix**: Either install Verus and Flux tooling AND fix the substantive defects, or file formal waivers with compensating evidence from the remaining verifier lanes (Kani + proptest + fuzz) that provide equivalent coverage.

### F-vb-y9d3v-S6-0012 — Non-Blocking Findings from State 4 Unresolved

- **Severity**: HIGH
- **Finding**: The proof-plan-review identified 4 non-blocking findings that should have been resolved in State 5:
  - F-vb-y9d3v-0006: Kani command still `bash scripts/kani-list.sh` (inventory) vs actual `cargo kani` verification
  - F-vb-y9d3v-0007: Feature flag `vb-8mdp-5` (old bead reference)
  - F-vb-y9d3v-0008: Coverage matrix TMR clause mappings stale
  - F-vb-y9d3v-0009: TBP-008 stale PO-028 reference
- **Required fix**: Resolve all four findings. F-vb-y9d3v-0006 is particularly important — the command field in `proof-obligations.planned.jsonl` still specifies `kani-list.sh` which is an inventory tool, not a verification command.

### F-vb-y9d3v-S6-0013 — Verus `external_body` Declarations Have `requires: true`

- **Severity**: HIGH
- **Artifact**: `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs:399–463`
- **Finding**: All three `#[verifier::external_body]` declarations accept ALL inputs (`requires: true`). This means the spec is vacuously true — any implementation, no matter how broken, satisfies these specs. For example, `production_validate_ticket_attempt_spec` accepts any `u16` values including `attempt: 0, capacity: 0` which production code rejects.
- **Required fix**: Add preconditions that bound inputs to valid ranges, then ensure the `ensures` clause encodes the actual postcondition (correct error types for invalid inputs, `Ok(())` only for valid inputs).

---

## Medium Severity Findings

### F-vb-y9d3v-S6-0014 — Verus `proof_action_fence_exhaustive` Uses `by(compute)` Without Induction

- **Severity**: MEDIUM
- **Artifact**: `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs:377–387`
- **Finding**: The proof computes the always-true spec function, which succeeds trivially. No inductive reasoning, no invariant, no loop — the proof is vacuous because the spec is vacuous. If the spec were non-trivial, `by(compute)` on raw `u16`/`bool` would not scale to meaningful properties.
- **Required fix**: After fixing the tautological spec, replace `by(compute)` with real proof steps that reason about the relationship between preconditions and postconditions.

### F-vb-y9d3v-S6-0015 — Fuzz Target PENDING_FORMAL_EXECUTION

- **Severity**: MEDIUM
- **Artifact**: `fuzz/fuzz_targets/fuzz_retry_codec.rs`
- **Obligation**: PO-vb-y9d3v-0041
- **Finding**: No `cargo fuzz run` evidence exists. The proof-reviewer skill allows PENDING_FORMAL_EXECUTION with "cheap smoke/typecheck evidence" — but no typecheck evidence was provided for the fuzz target. The target references `vb_runtime::shard::helpers::normalize_scheduled_ticket`, `vb_core::action::ActionTicket`, and `postcard::to_allocvec`/`from_bytes`, which must be verified to compile.
- **Required fix**: Run `cargo fuzz build fuzz_retry_codec` to prove compilation, then execute the planned `cargo fuzz run` campaign.

---

## Accepted Evidence

| Evidence | Obligations | Assessment |
|---|---|---|
| proptest 14/14 PASS (`cargo test -p vb_runtime -- proptest_attempt_fence --nocapture`) | PO-0004, PO-0008, PO-0012, PO-0016, PO-0020, PO-0024, PO-0028, PO-0032, PO-0036, PO-0040 | Raw evidence confirmed. Tests exercise production `use crate::shard::helpers::*` functions. However, hardcoded workflow structure (F-vb-y9d3v-S6-0009) limits input space coverage. |
| Kani smoke `proof_typed_missing_run_error` (VERIFICATION:- SUCCESSFUL, 0/478 failed) | PO-0021 (partial) | Raw Kani output confirmed. However, the harness is substantively vacuous (F-vb-y9d3v-S6-0005). |
| Build check `cargo check -p vb_runtime` PASS | All artifacts | Module wiring, imports, and type resolution confirmed. |
| Test compilation `cargo test -p vb_runtime --no-run` PASS | proptest | Test binary builds successfully. |

---

## GOD RULE Compliance Matrix

| GOD RULE | Verdict | Evidence |
|---|---|---|
| 1: No hardcoded Kani shapes | VIOLATED | `any_do_run_state` / `make_do_run_state` hardcode WorkflowParts (F-vb-y9d3v-S6-0009) |
| 2: No vacuum Verus proofs | VIOLATED | Verus specs return `true` for all inputs; disconnected from production (F-vb-y9d3v-S6-0001 through S6-0004) |
| 3: No unbounded TLA+ math | NOT APPLICABLE | TLA+ globally removed from verifier whitelist |
| 4: Fix implementation, not proof | NOT YET EVALUATED | No implementation fixes attempted |
| 5: No blind verification mutations | NOT APPLICABLE | No mutations run |

---

## Trusted Base Ledger Review

| TBP ID | Marker | Current Disposition | Reviewer Action |
|---|---|---|---|
| TBP-009 | `external_body` (Verus) | `pending-proof-reviewer` | **REJECTED** — `requires: true` makes this trust boundary unbounded. Fix requires real preconditions. |
| TBP-010 | `extern_spec` (Flux) | `pending-proof-reviewer` | **REJECTED** — false invariant `attempt > 0` on ActionTicket. |
| TBP-011 | `assume` (Kani bounds) | `pending-proof-reviewer` | **ACCEPTED** — bounded `step<16`, `attempt>0`, `capacity<=255` are reasonable for u16 values. |
| TBP-012 | `trusted` (fuzz scaffold) | `pending-proof-reviewer` | **ACCEPTED** — inline workflow construction acceptable for fuzz targets. |
| TBP-013 | `external_body` (future-attempt gap) | `pending-proof-reviewer` | **NOTED** — gap acknowledged; spec documents both current and desired behavior. |
| TBP-014 | `blocked` (Verus tooling) | `accepted-blocker` | **NOTED** — valid blocker; but artifacts would not pass even with tooling. |
| TBP-015 | `blocked` (Flux tooling) | `accepted-blocker` | **NOTED** — valid blocker; but artifacts contain false invariants. |

---

## Disposition

**STATUS: REJECTED**

The State 5 proof-writer artifacts fail adversarial review on multiple critical grounds:

1. **GOD RULE 2 violated twice**: Verus specs are disconnected from production code (all `by(compute)` on self-defined models). Flux refinements assert false invariants on production types.

2. **GOD RULE 1 violated**: Both Kani and proptest harnesses use hardcoded `WorkflowParts` with fixed Do-node structure, violating the mandate for `kani::Arbitrary` structural inputs.

3. **Systemic vacuity**: At least 4 Kani harnesses test the wrong function, the Rust borrow checker, or enum variant matching rather than the ActionTicket fence. The only Kani smoke-verified harness exercises zero production functions.

4. **Tautological Verus proofs**: Three spec functions return `true` unconditionally. The `requires: true` on external_body declarations accepts all inputs.

5. **20 of 41 obligations**: Have zero raw verifier evidence. The existing artifacts contain defects that would cause failures even with tooling installed.

### Required Remediation

1. **Kani**: Rewrite all harnesses to test actual production functions. Add `#[cfg(kani)]` visibility for private `validate_ticket_attempt`. Replace `kani::cover!(true, ...)` with `kani::assert`. Implement `kani::Arbitrary` for `WorkflowParts` or write bounded structural generators with variable node kinds/action IDs. Execute `cargo kani` for all 10 harnesses.

2. **Verus**: Rewrite all specs to model real production types and functions. Replace `true`-returning spec functions with behavioral contracts. Add non-trivial `requires/ensures` on `external_body` declarations. Execute `bash scripts/verify-verus.sh`.

3. **Flux**: Remove false `#[invariant(self.attempt > 0)]`. Bind refinements to actual production type definitions. Refine the validation functions' postconditions instead of the struct invariant. Execute `cargo flux -p vb_runtime`.

4. **proptest**: Add variable workflow structure generation. Expand hostile-input coverage per contract acceptance-invariant 2 (lower stale, exact current, future within capacity, zero attempt, zero capacity, over-capacity).

5. **cargo-fuzz**: Execute `cargo fuzz run fuzz_retry_codec` with documented iteration count and collect raw output.

6. **Plan findings**: Resolve F-vb-y9d3v-0006 through F-vb-y9d3v-0009 from State 4.

### Approvable Threshold

This review can be reopened as APPROVED when:
- At least the Kani AND proptest lanes have non-vacuous evidence from corrected harnesses
- Verus/Flux lanes have either valid tooling output OR formal waivers with compensating evidence from Kani+proptest+fuzz
- All blocking findings (S6-0001 through S6-0010) are resolved
- All HIGH findings (S6-0011 through S6-0013) are resolved
- Raw command evidence is provided for every obligation
