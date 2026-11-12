# Proof Writer Report: vb-qi37.2.4

## Scope
- Bead: `vb-qi37.2.4`.
- Workspace: `/home/lewis/src/vb-femdation/vb-qi37-2-4`.
- Work performed: verification artifacts only for bounded nested collect/reduce/repeat/together fanout and admission composition checks.
- Production runtime/test code edited: none.
- Forbidden lifecycle agents invoked: none.

## Changed Verification Artifacts
- `specs/tla/BoundedAdmission.tla`: added explicit `verified_budget` and `rejected_budget` state, guarded `AdmitRun` on verified budget, rejected unverified/over-limit paths before admission, removed completed runs from shard capacity accounting, and added retry of rejected budget proofs to avoid terminal deadlock in the bounded model.
- `specs/tla/BoundedAdmission.cfg`: added invariant checks for reservation, capacity, verified budget, and positive admitted resources.
- `verification/verus/budget_bounded.rs`: added proof-only specs and lemmas for checked sequential composition, finite nested multiplication, unknown-bound rejection, multiplication overflow rejection, conservative branch maximum, bounded together fanout, whole-to-aggregate refinement, and diagnostic-field totality.

## Obligation Status
- `TLA-ADM-001`: PASS. `NoRunAdmittedWithoutReservation`, `ShardCapacityBounded`, and positive admitted resources are checked by TLC.
- `TLA-ADM-002`: PASS. The model now has explicit verified/rejected budget state and `AdmitRun` requires `run \in verified_budget` and `run \notin rejected_budget`.
- `VERUS-BUD-001`: PASS. Sequential checked composition monotonicity verified by `proof_sequential_checked_compose_monotone` plus existing sequential lemmas.
- `VERUS-BUD-002`: PASS. Nested finite multiplication, unknown factor rejection, and overflow rejection verified by `proof_nested_finite_repeat_cost`, `proof_unknown_factor_rejects`, and `proof_nested_overflow_rejects`.
- `VERUS-BUD-003`: PASS. Conservative branch maximum and together fanout bounds verified by `proof_branch_max_conservative`, `proof_together_fanout_bounded`, and `proof_together_fanout_over_limit_rejects`.
- `VERUS-AGG-001`: PASS. Aggregate proof surface verifies direct refinement from verified whole dimensions via `proof_aggregate_refines_verified_whole`.
- `VERUS-DIAG-001`: PASS for proof-visible projection. `proof_diagnostic_projection_total` proves every required diagnostic field is mandatory in the abstract projection. Runtime diagnostic property coverage remains owned by later property-test state.
- `KANI-BUD-001`: BLOCKED_SCOPE. Planned artifact target is `crates/vb_core/src/budget.rs`, but this task forbids production runtime/test-code edits. Owner state: 7. Rerun from: 7.
- `PROP-BUD-001`: BLOCKED_SCOPE. Requires property generators/tests under runtime/test code, forbidden in this proof-writer-only state. Owner state: 7. Rerun from: 7.
- `PROP-DIAG-001`: BLOCKED_SCOPE. Requires observable runtime diagnostic tests/property generators, forbidden in this proof-writer-only state. Owner state: 7. Rerun from: 7.
- `GATE-BUD-001`: BLOCKED_TOOLING. `moon run :verify-proof` fails before proof execution because `scripts/rust-verification-gauntlet.sh` is parsed by bash and contains Rust doc-comment syntax at lines 3-7. Owner state: 12. Rerun from: 12.
- `GATE-BUD-002`: NOT_RUN. Deep lane is later-state defense in depth and depends on forbidden proptest/fuzz/Miri/mutation artifacts. Owner state: 12. Rerun from: 12.
- `GATE-BUD-003`: NOT_RUN. Standard lane is later-state compile/lint/test confidence and was not required to validate the proof-only edits after the proof gate tooling failure. Owner state: 12. Rerun from: 12.

## Commands Run
- `which verus || true`: PASS, resolved `/home/lewis/.local/bin/verus`.
- `which tlc || true`: PASS, resolved `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`.
- `which java || true`: PASS, resolved `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`.
- `which moon || true`: PASS, resolved `/home/lewis/.local/share/mise/installs/npm-moonrepo-cli/2.2.4/bin/moon`.
- `verus verification/verus/budget_bounded.rs`: initial FAIL_LOCAL due invalid zero-body monotonicity postcondition; repaired.
- `verus verification/verus/budget_bounded.rs`: PASS, `verification results:: 15 verified, 0 errors`.
- `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla`: initial FAIL_LOCAL due missing unchanged vars in `AdmitRun`; repaired.
- `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla`: second FAIL_LOCAL due rejecting an already admitted run; repaired.
- `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla`: third FAIL_LOCAL due terminal rejected-budget deadlock; repaired.
- `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla`: PASS, no errors, `108977 states generated`, `9762 distinct states found`, `0 states left on queue`, complete state graph depth `9`.
- `moon run :verify-proof`: BLOCKED_TOOLING, exit code 2, `scripts/rust-verification-gauntlet.sh: line 3: //!: No such file or directory` through syntax error at line 7.

## Assumptions And Bounds
- TLA+ constants: `RunId = {1, 2}`, `ShardId = {1}`, `MaxRunsPerShard = 2`, `MaxSlotsPerRun = 1`, `MaxActionsPerRun = 1`.
- TLA+ resource abstraction keeps `memory_bytes` bounded to `0..10`; admission invariants for this bead require positive slots/actions and verified budget before admission.
- Verus arithmetic abstracts Rust checked arithmetic as mathematical integers with `u64::MAX = 18446744073709551615`.
- Verus proof bounds: `max_steps_per_workflow = 65535`, `max_step_budget = 10000`, `max_parallel_in_flight = 1024`, `max_action_tickets = 1000000`.
- The Verus diagnostic proof is an abstract total projection; production diagnostic rendering remains a later property-test obligation.

## Reviewer Guidance
- Review the TLA+ model as an admission-ordering abstraction, not as a full runtime scheduler.
- Review the Verus file as pure proof surface; it intentionally does not import production Rust.
- Do not treat `GATE-BUD-001` as proof failure. The narrow proof artifacts pass, but the rollup script is currently not executable as bash.

## State 5 Repair Attempt 2: PR-004 Mapping Gap
- Repaired `PR-004` by adding executable State 5 proof-obligation rows for `VERUS-AGG-001` and `VERUS-DIAG-001` to `.beads/vb-qi37.2.4/proof-obligations.jsonl`.
- Added traceability mappings for `VERUS-AGG-001` under `POST-001` and `VERUS-DIAG-001` under `POST-009`/`INV-005` in `.beads/vb-qi37.2.4/traceability-matrix.jsonl`.
- No production runtime/test code edited.
- Existing command evidence remains `verus verification/verus/budget_bounded.rs` => `verification results:: 15 verified, 0 errors`; rerun recorded in proof evidence after this repair.
