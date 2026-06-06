# Routing Decision: vb-63st6.2 — Worktree Loom Obligation

bead_id: vb-63st6.2
parent: vb-63st6
decider_skill: proof-plan-reviewer (controller-approved sublane)
decision_date: 2026-06-06
decision_status: APPROVED — route to state6-test-writer

## Decision (Option a)

Route `PO-vb-63st6-WORKTREE-LOOM` to `state6-test-writer` as the canonical
owner of the approved crate test artifact path. Reject option (b) (lane
removal) and option (c) (production helper exposure) for the reasons below.

## Reason For Rejection Of Other Options

- **Option (b) — remove the Loom lane.** Rejected.
  The obligation binds `REQ-WORKTREE-001`, which carries the
  `cross-agent-interference`, `worktree-ownership`, and `global-git-state`
  risk tags. The parent bead (vb-63st6) was specifically opened because
  local worktree/stash/branch state was being mutated without owner
  approval. Dropping the schedule-exploration lane would leave the only
  behavior-affecting coverage for that risk to a property test, and would
  forfeit the only deterministic exploration of concurrent interference
  patterns across agents. Residual risk is unacceptable for a
  global-git-state seed.

- **Option (c) — expose a production interference helper.** Rejected for
  this routing cycle.
  vb-63st6.1 (BLOCK_LOCAL_PRODUCTION_BINDING) already closed as blocked
  because every production parser/core/disposition/approval/stash/worktree
  helper required by the approved obligations is absent. Adding a
  worktree-only production helper in isolation does not close
  vb-63st6.1, and would still leave the Loom obligation owner-ambiguous
  until a follow-up state7-implementation bead lands. That is a separate
  implementation track, not a routing fix.

## Approved Route (Option a — test-writer ownership)

| Field                    | Value                                                                  |
| ------------------------ | ---------------------------------------------------------------------- |
| Obligation ID            | `PO-vb-63st6-WORKTREE-LOOM`                                            |
| Requirement              | `REQ-WORKTREE-001`                                                     |
| Verifier                 | `loom`                                                                 |
| Artifact path (file)     | `crates/vb_core/tests/vb_63st6_worktree_interference.rs`               |
| Owner state              | `state6-test-writer` (was `state5-proof-writer`)                       |
| Rerun from               | approved test lane (State 6 test-writer)                               |
| Mode                     | `schedule-exploration`                                                 |
| Sibling obligation       | `PO-vb-63st6-WORKTREE-PROPTEST` already owned by `state6-test-writer`  |
| Proof plan owner action  | `state4-proof-plan-reviewer` updates `owner_state` field on the row    |

The `crates/vb_core/tests/` directory is the canonical home for crate
integration tests in this repository (see
`crates/vb_core/tests/aggregate_resource_budget_red.rs` and siblings
in the same directory). `test-writer` is the only approved owner
permitted to author and edit files in that path. Routing the Loom
obligation to `test-writer` removes the State 6 ownership ambiguity
without changing the approved artifact location, command, or expected
evidence shape.

## Ownership Boundary

- `proof-writer`: continues to author Verus, Kani, Flux, and cargo-fuzz
  artifacts under their approved production-source locations
  (e.g. `verification/verus/*.rs`, `crates/*/src/verification/*.rs`,
  `fuzz/fuzz_targets/*.rs`). The proof-writer dispatch forbids behavior
  test edits, so proof-writer must not author
  `crates/vb_core/tests/vb_63st6_worktree_interference.rs`.
- `test-writer`: owns the file
  `crates/vb_core/tests/vb_63st6_worktree_interference.rs`, the
  `loom_worktree_contract` test, and the execution of the approved
  `cargo test` command under `RUSTFLAGS="--cfg loom"`.

## Executable Command Evidence

The approved Loom command (unchanged from
`proof-obligations.planned.jsonl:31`) is the executable evidence
test-writer must run and capture raw successful output for:

```text
RUSTFLAGS="--cfg loom" cargo test -p vb_core --test vb_63st6_worktree_interference loom_worktree_contract -- --exact
```

Pre-conditions the test must satisfy:

1. The file `crates/vb_core/tests/vb_63st6_worktree_interference.rs`
   exists and is owned by test-writer.
2. The file declares a `#[test] fn loom_worktree_contract()` whose
   `loom::model` body is allowed to reference pure core helpers for
   `REQ-WORKTREE-001`; if no production helper exists yet, the test
   must use a documented seam (a `#[cfg(test)]` or `#[cfg(loom)]`
   shim) and the harness must be the source of truth — not a copied
   model.
3. The command exits 0 with raw loom-explored interleavings recorded
   in stderr/stdout. The exact log, command text, exit status, and
   `sha256sum` of the test file must be appended to
   `.beads/vb-63st6/proof-evidence.md` under the `## Loom` section.

## Residual Risk Assessment

After this routing decision, the Loom obligation is no longer
owner-ambiguous. The remaining residual risk is **not** a routing
risk; it is a **production-binding risk** that already lives in
vb-63st6.1:

- The pure core helpers for worktree interference/approval may not
  exist. test-writer should not invent helpers in a behavior test;
  the helpers must be exposed by an implementation owner first.
  If helpers are absent at execution time, test-writer records the
  blocker and the obligation remains `BLOCK_LOCAL_PRODUCTION_BINDING`
  on its lane, not `BLOCK_LOCAL_ARCHITECTURE` on ownership.

This residual risk is documented and tracked in
`PF-vb-63st6-R2-001` (see
`.beads/vb-63st6/proof-review.md` finding 1). It does not block the
acceptance of this routing decision.

## State 6 Expectation

After proof-plan-reviewer updates the `owner_state` of
`PO-vb-63st6-WORKTREE-LOOM` to `state6-test-writer`, State 6 will no
longer report `PF-vb-63st6-R2-002` as an open ownership blocker.
The remaining Loom evidence requirement is the same raw successful
log that the obligation has always specified, owned and executed by
test-writer.

## Acceptance Criteria Mapping

Bead acceptance criteria:

1. The Loom/worktree interference obligation has an approved owner
   lane — satisfied by routing to `state6-test-writer` (Option a).
2. Executable command evidence — `RUSTFLAGS="--cfg loom" cargo test -p
   vb_core --test vb_63st6_worktree_interference loom_worktree_contract
   -- --exact` is the documented evidence path.
3. State 6 no longer reports ownership ambiguity — the
   `owner_state` field is changed from `state5-proof-writer` to
   `state6-test-writer`; `PF-vb-63st6-R2-002` becomes closed.
