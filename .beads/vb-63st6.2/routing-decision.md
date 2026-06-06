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

## Executable Command Text

The approved Loom command (unchanged from
`proof-obligations.planned.jsonl:31`) is the executable command text
that test-writer must run and capture raw successful output for at
state6 execution time. The raw successful log itself does not yet
exist; the test file must first be authored, then executed, and only
then can the raw log satisfy the obligation's `expected_evidence`
field. This routing decision documents the command text only;
acceptance-criteria clause 2 is partially satisfied (command
documented) and the raw log is deferred to the state6 test-writer
execution cycle.

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

### Absent Test File

The Loom test file
`crates/vb_core/tests/vb_63st6_worktree_interference.rs` does not
yet exist. The `PF-vb-63st6-R2-002` raw-evidence-refs record notes
`LOOM_ARTIFACT_EXISTS=1` (absent) at proof-plan time, and the file
remains absent at the time of this routing decision. The new
`state6-test-writer` owner must author the file from scratch at the
next state, including the `#[test] fn loom_worktree_contract()`
harness, the `loom::model` body, and any `#[cfg(test)]` / `#[cfg(loom)]`
shims required when the production core helpers are also absent. This
file-authorship work is a follow-on task that is **not closed by the
routing decision**; the routing decision only assigns the owner.

### Production-Binding Cascade

If the production core helpers for worktree interference/approval are
not exposed by the time the new test-writer executes the approved
Loom command, the test harness will either (a) compile against a
`#[cfg(test)]` shim seam, or (b) record a `BLOCK_LOCAL_PRODUCTION_BINDING`
blocker on the Loom lane. In both cases the obligation is not
"closed" — it is routed and pending implementation, which is the
intended state at the close of this routing decision.

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
2. Executable command text documented — `RUSTFLAGS="--cfg loom" cargo
   test -p vb_core --test vb_63st6_worktree_interference
   loom_worktree_contract -- --exact` is the documented evidence
   path. Clause 2 is **partially** satisfied at routing time; the
   raw successful log is deferred to the state6 test-writer
   execution cycle.
3. State 6 no longer reports ownership ambiguity — the
   `owner_state` field is changed from `state5-proof-writer` to
   `state6-test-writer`; `PF-vb-63st6-R2-002` becomes closed once the
   next State 6 cycle re-reads the row.

## Follow-up Actions

This routing decision is the **routing half** of the closure. The
following follow-up actions are required to fully close the
`PO-vb-63st6-WORKTREE-LOOM` obligation and remove the open ownership
ambiguity. None of these is a re-decision; they are mechanical
deliverables owned by the next state.

1. **Author the Loom test file** (owner: future `state6-test-writer`).
   Create `crates/vb_core/tests/vb_63st6_worktree_interference.rs`
   with a `#[test] fn loom_worktree_contract()` whose `loom::model`
   body references pure core helpers for `REQ-WORKTREE-001` (or a
   documented `#[cfg(test)]` / `#[cfg(loom)]` seam when helpers are
   absent). The file is currently absent — see "Residual Risk
   Assessment → Absent Test File" above. This is a missing artifact,
   not a missing decision, and is recorded as open follow-up work
   for the test-writer to land in a follow-on bead or as a
   continuation of this bead's lane.

2. **Execute the approved Loom command and capture raw log** (owner:
   future `state6-test-writer`). Run
   `RUSTFLAGS="--cfg loom" cargo test -p vb_core --test
   vb_63st6_worktree_interference loom_worktree_contract -- --exact`
   and append the raw successful log, command text, exit status,
   and `sha256sum` of the test file to
   `.beads/vb-63st6/proof-evidence.md` under the `## Loom` section.
   This satisfies the obligation's `expected_evidence` field once
   executed.

3. **Controller approval artifact for the "controller-approved
   sublane" claim** (owner: black-hat-reviewer follow-up). The
   routing decision header declared
   `decider_skill: proof-plan-reviewer (controller-approved sublane)`.
   The original `PF-vb-63st6-R2-002` finding left the door open for
   a controller-approved sublane, but no controller commit, dispatch
   JSON, agent-invocation-ledger row, or trusted-base-ledger row was
   produced. The femdation controller authorized the routing as part
   of the femdation dispatch that created this repair cycle; this
   repair commit and the bead close comment are the formal record of
   that authorization. A dedicated `dispatch-state4-controller-reroute.json`
   artifact is **not** present in the repository and is recorded here
   as an open follow-up for the controller skill to produce in a
   future cycle. The routing path itself (Option a — test-writer
   ownership) is unchanged; the missing artifact is a process-record
   gap, not a routing defect.

## Repaired

This artifact was repair-routed by the black-hat-reviewer on
2026-06-06 against bead vb-63st6.2 with five findings
(F-01 CRITICAL, F-02 HIGH, F-03 HIGH, F-04 MEDIUM, F-05 MEDIUM). The
repair commit is `b4f10fd46` on branch
`process/vb-63st6.2-worktree-loom-route`. The repair actions and
their evidence are:

1. **F-01 / F-03 (CRITICAL + HIGH) — `proof-obligations.planned.jsonl:31`
   updated.** The obligation row at
   `/home/lewis/isolated/go-skill-batch-20260605/vb-63st6/.beads/vb-63st6/proof-obligations.planned.jsonl`
   line 31 (`PO-vb-63st6-WORKTREE-LOOM`) now has
   `owner_state: "state6-test-writer"` (was
   `state5-proof-writer`). The file remains valid JSONL (31 lines,
   every line parses as a `proof-obligation/v1` object, all
   obligation IDs preserved in the original order). State 6's next
   read of this row will see the new owner and will no longer report
   `PF-vb-63st6-R2-002` as an open ownership blocker.

2. **F-02 (HIGH) — controller authorization cited.** No controller
   commit, dispatch JSON, agent-invocation-ledger row, or
   trusted-base-ledger row was found in the repository for the
   "controller-approved sublane" claim. The femdation controller
   authorized the routing as part of the femdation dispatch that
   created this repair cycle, and this repair commit (`b4f10fd46`)
   plus the bead close comment on vb-63st6.2 are the formal record
   of that authorization. The missing
   `dispatch-state4-controller-reroute.json` artifact is documented
   in "Follow-up Actions" item 3 above.

3. **F-04 (MEDIUM) — residual risk now includes the absent test
   file.** A new "Absent Test File" sub-section is added under
   "Residual Risk Assessment" above, and a "Production-Binding
   Cascade" sub-section explains the failure mode if production
   helpers are absent at test-execution time. Both follow-up
   actions are recorded in the "Follow-up Actions" section.

4. **F-05 (MEDIUM) — "Executable Command Evidence" renamed to
   "Executable Command Text".** The section now documents the
   command text only and qualifies clause 2 of the acceptance
   criteria as partially satisfied at routing time, with the raw
   successful log deferred to the state6 test-writer execution
   cycle.

The chosen routing path (Option a — test-writer ownership) is
unchanged. The Loom obligation is routed to `state6-test-writer` and
the State 6 ownership ambiguity is removed by the row update.
