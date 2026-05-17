# Verification Layers: vb-scxh

## Boundary

- TLA+ temporal model: recovery lifecycle, evidence classification, laundering rejection, safety anchor gate, mutation/scope safety, close/unblock safety.
- Manual/raw evidence: BD, git safety anchor, CI, mutation, scope-control, final decision.
- Verus/Lean/Kani/Flux/Loom/Miri/proptest/fuzz: no current Rust/protocol/codec/concurrency target; waived/deferred in primary ledger.

## Layer assignment

- PRE-SCXH-001 -> path guard and artifact write audit.
- PRE-SCXH-002 -> artifact-presence audit.
- PRE-SCXH-003 -> raw BD command audit.
- POST-SCXH-001 -> BD exact-12 closure audit.
- POST-SCXH-002 / INV-SCXH-001 -> TLA+ non-laundering model plus Truth Serum evidence-classification audit.
- POST-SCXH-003 -> green CI raw evidence audit.
- POST-SCXH-004 / INV-SCXH-005 -> mutation classification audit and TLA invariant.
- POST-SCXH-005 / INV-SCXH-004 -> generated parity deferral/scope-control audit and TLA invariant.
- POST-SCXH-006 / INV-SCXH-002 -> TLA lifecycle model, assurance bundle, Truth Serum report, final evidence decision.
- POST-SCXH-007 -> safety-anchor audit; current bundle-open failure is `BLOCK_LOCAL`.
- INV-SCXH-003 -> State 3 path/diff review.
- INV-SCXH-006 -> TLA path consistency audit.
- All `Error::*` variants -> explicit error trace rows and proof-obligation rows.

## TLA+ scope

- Module/model path: `.beads/vb-scxh/tla/ScxhRecovery.tla`.
- Config path: `.beads/vb-scxh/tla/ScxhRecovery.cfg`.
- Command: `tlc -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla`.
- Required invariants: `NoEngineUnblockBeforeApprovedEvidence`, `FalseClosuresVerifiedBeforeClose`, `NoAcceptanceFromSubagentRequiredEvidence`, `LaunderingAttemptRejected`, `SafetyAnchorRequiredForApproval`, `MutationUnviableNotPass`, `ParityGapOwnershipPreserved`.
- Liveness/fairness: must be configured in State 5 or explicitly waived as non-evidence for closure.

## Raw evidence obligations

- Exact false closures: State 11 must produce `bd-closure-audit.md` with all 12 IDs and per-ID raw reopened/linked/follow-up evidence.
- Safety anchor: State 11 must rerun/capture bundle and bookmark verification. Known bundle-open failure is blocking until repaired.
- Green CI: State 11 must audit or rerun `moon ci` evidence with exact markers.
- Mutation: State 11 must preserve `FAIL_UNVIABLE / DEFERRED`, not PASS.
- Scope: State 11 must keep generated parity gaps deferred to `vb-gvmt` / `vb-qi37.10`.
- Final decision: State 12 must produce `truth-serum-report.md` and `final-evidence-decision.md`; close/unblock is blocked if any required local evidence is missing or blocked.

## Primary waiver ledger

Machine-readable waiver rows are in `proof-obligations.jsonl` for Verus, Lean/Aeneas/Hax, Kani, Flux, Loom, Miri, proptest, fuzz, performance, API compatibility, and release-provenance. These waivers do not waive raw evidence obligations; they only state that no production Rust/proof target exists in this recovery-only bead.
