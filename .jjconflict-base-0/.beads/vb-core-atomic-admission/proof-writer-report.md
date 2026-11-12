# Proof Writer Report: vb-core-atomic-admission

bead_id: vb-core-atomic-admission
state: 5
attempt: 3-of-7
status: PASS_EXECUTABLE_STATE5_RESTART_REPAIR
updated_at: 2026-05-15T22:41:10Z

## Scope

Proof-writer consumed the repaired State 4 plan and State 6 rejection artifacts from the isolated workspace only.

No production source, tests, dependencies, CI files, or source checkout files were edited.

## Artifacts Written

- `verification/tla/AtomicAcceptedRunAdmission.tla`
- `verification/tla/AtomicAcceptedRunAdmission.cfg`
- `verification/verus/accepted_run_atomic_admission.rs`
- `.beads/vb-core-atomic-admission/proof-writer-report.md`
- `.beads/vb-core-atomic-admission/proof-evidence.md`
- `.beads/vb-core-atomic-admission/STATE.md`

## Repair Delta

- `TLA-ATOM-001`: removed `CHECK_DEADLOCK FALSE`, added `WF_vars(ReadbackAccepted)`, defined `EventuallyReadableAfterCommit`, and added it as a TLC `PROPERTY`.
- `VERUS-IDX-005`: renamed the proof surface to precondition decomposition: `spec_required_index_preconditions` and `proof_index_precondition_decomposition`. It does not claim runtime key derivation.
- `VERUS-ERR-006`: strengthened the pure model with `SpecAdmissionOutcome`, `spec_admission_outcome`, and `spec_outcome_is_err`, proving every modeled failure cause maps to an `Err` outcome in addition to taxonomy existence.

## Obligation Coverage

| Obligation | Artifact | State 5 result |
|---|---|---|
| TLA-ATOM-001 | `verification/tla/AtomicAcceptedRunAdmission.tla`; `verification/tla/AtomicAcceptedRunAdmission.cfg` | PASS: TLC exit 0 after repair; invariants and both temporal properties checked with deadlock checking enabled by absence of `CHECK_DEADLOCK FALSE`. |
| VERUS-PRE-001 | `verification/verus/accepted_run_atomic_admission.rs` | PASS: Verus exit 0 verifies valid pure input has source, artifact, header, runtime policy, and capabilities. |
| VERUS-PRE-002 | `verification/verus/accepted_run_atomic_admission.rs` | PASS: Verus exit 0 verifies coherent pure input references. |
| VERUS-SEQ-003 | `verification/verus/accepted_run_atomic_admission.rs` | PASS: Verus exit 0 verifies accepted sequence is positive and equals `RunAccepted.seq` for same run. |
| VERUS-ART-004 | `verification/verus/accepted_run_atomic_admission.rs` | PASS: Verus exit 0 verifies strict payload discriminator rejects raw, legacy, and malformed tags. |
| VERUS-IDX-005 | `verification/verus/accepted_run_atomic_admission.rs` | PASS_NARROWED: Verus exit 0 verifies pure index precondition decomposition only; deterministic runtime key derivation remains owned by `ERR-INDEX-022` and later integration evidence. |
| VERUS-ERR-006 | `verification/verus/accepted_run_atomic_admission.rs` | PASS_NARROWED: Verus exit 0 verifies each modeled failure cause classifies to an error and maps to an `Err` outcome; production `Result` propagation remains owned by `STATIC-SCAN-011`, `MUT-ERR-010`, and per-variant scenarios. |
| KANI-PROP-007 | none | WAIVED_NOT_RUN per repaired plan: no exact accepted-run sequence-binding Kani harness exists; owner State 8 before State 12. No Kani pass claimed. |
| FUZZ-ART-008 | none | WAIVED_NOT_RUN per repaired plan: no exact malformed `AcceptedArtifact` fuzz target exists; owner State 8 before State 12. No fuzz pass claimed. |
| MIRI-CODEC-009 | none | NOT_RUN: owner State 12 after implementation/codec test existence check; Miri tool available. No Miri pass claimed. |
| MUT-ERR-010 | none | NOT_RUN: owner State 12 after implementation/tests; mutation command not run by proof-writer. |
| STATIC-SCAN-011 | none | NOT_RUN: owner State 12; `moon ci` not run because State 5 scope forbids production/test/CI changes and this is a later machine gate. |
| INTEG-FAIL-012 | none | NOT_RUN: owner State 12 after integration scenarios exist. |
| API-COMPAT-013 | none | NOT_RUN: owner State 12, conditional on public API/CLI diff evidence. |
| PERF-NONGOAL-014 | `.beads/vb-core-atomic-admission/verification-layers.md` | NOT_APPLICABLE per repaired plan: no speed/vectorization/latency claim exists. |
| ERR-INVALID-015 | none | NOT_RUN: owner State 12 after scenario implementation. |
| ERR-INCONSISTENT-016 | none | NOT_RUN: owner State 12 after scenario implementation. |
| ERR-STAGE-017 | none | NOT_RUN: owner State 12 after scenario implementation. |
| ERR-COMMIT-018 | none | NOT_RUN: owner State 12 after scenario implementation. |
| ERR-PARTIAL-019 | none | NOT_RUN: owner State 12 after scenario implementation. |
| ERR-SEQUENCE-020 | none | NOT_RUN: owner State 12 after scenario implementation. |
| ERR-STRICT-RAW-021 | none | NOT_RUN: owner State 12 after scenario implementation. |
| ERR-INDEX-022 | none | NOT_RUN: owner State 12 after scenario implementation. |

## Commands Run

- `pwd -P`: exit 0; `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission`.
- `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`: exit nonzero before syntax repair; TLC reported TLA+ precedence conflict at `EventuallyReadableAfterCommit`.
- `verus verification/verus/accepted_run_atomic_admission.rs`: exit 0; `verification results:: 6 verified, 0 errors`.
- `/usr/bin/rg -n "EventuallyReadableAfterCommit|CHECK_DEADLOCK|WF_vars\\(Readback|PROPERTY" verification/tla/AtomicAcceptedRunAdmission.*`: exit 0; showed `WF_vars(ReadbackAccepted)` and both `PROPERTY` rows, with no `CHECK_DEADLOCK` match.
- `which java`: exit 0; `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`.
- `which tlc`: exit 0; `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`.
- `which verus`: exit 0; `/home/lewis/.local/bin/verus`.
- `cargo kani --version`: exit 0; `cargo-kani 0.67.0`.
- `cargo fuzz --version`: exit 0; `cargo-fuzz 0.13.1`.
- `cargo flux --version`: exit 101; no such cargo command `flux`.
- `cargo +nightly miri --version`: exit 0; `miri 0.1.0 (e0e95a7187 2026-04-04)`.
- `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`: exit 0; model checking completed with no error, 6,828 states generated, 1,080 distinct states found, temporal properties checked.

## Assumptions And Bounds

- TLA+ constants: `Runs = {r1, r2}` and `Workflows = {w1, w2}`.
- TLA+ record families: source, artifact, header, `RunAccepted`, status index, workflow index, and action index.
- TLA+ models Fjall commit as one atomic durable primitive; staged records are erased on injected failure and are not durable before commit.
- TLA+ fairness is limited to `StageRecord`, `CommitOrFail`, `Acknowledge`, and `ReadbackAccepted`. Injected failures are not fair progress after commit.
- Verus is a pure model. Runtime record conversion, byte codecs, storage I/O, CLI formatting, hash/proof byte validation, scheduling, and production struct conversion are trusted shell boundaries for later states.
- No Kani, fuzz, Miri, integration, mutation, static scan, API compatibility, or performance pass is claimed by State 5.

## Reviewer Guidance

Review should focus on whether the TLA+ liveness/deadlock repair satisfies `TLA-ATOM-001`, whether the narrowed Verus index claim matches the repaired plan, and whether the strengthened error-outcome model is still limited to pure modeled failure causes.

## State 5 Attempt 3 Repair After State 6 Rejection

State 6 attempt 3 rejected `TLA-ATOM-001` because restart/readback determinism was claimed but not modeled, and because the abstract `RecordKinds` family set lacked an explicit mapping to planned per-family variables/actions.

### Repair Delta

- `TLA-ATOM-001`: added explicit `restarted` state variable, `Restart` action, `WF_vars(Restart)`, `RestartReadbackDeterministic` invariant, and `EventuallyRestartReadbackAfterCommit` temporal property to `verification/tla/AtomicAcceptedRunAdmission.tla` and `.cfg`.
- `TLA-ATOM-001`: removed unused `EXTENDS Naturals, FiniteSets` because TLC was blocked by temp quota extracting unused standard modules; the model uses only built-in set/action syntax.
- `TLA-ATOM-001`: no upstream invalidation classified. The contract/obligation claim is valid and now has executable restart/readback coverage.
- Scope: proof/model artifacts and bead evidence only. No production source, tests, dependencies, CI files, or source-checkout files were edited.

### Record-Family Refinement Mapping

- `RecordKinds == {"source", "artifact", "header", "run_accepted", "status_index", "workflow_index", "action_index"}` maps one-to-one to the planned source record, accepted artifact record, admission header record, `RunAccepted` event record, status index, workflow index, and action index.
- `staged` is the abstract pre-commit batch membership set for those seven families. It represents records added to the Fjall `OwnedWriteBatch` before the atomic durable boundary.
- `durable` is the abstract post-commit visible family set. `Commit` moves from full `staged = RecordKinds` to `durable' = RecordKinds`; failures before/during commit erase both sets.
- `AllRecordsOrNoAcceptedRun` preserves the planned all-or-nothing visibility clause: an accepted readback can only occur when every mapped family is durable.
- `IndexesOnlyCommitted` preserves index-family consistency: any durable status/workflow/action index implies the full committed family set, not a standalone index.
- `NoPartialAfterFailure` preserves fail-closed visibility: failed pre-commit paths expose neither partial records nor ack/runtime allocation.
- `ReadbackOnlyCommitted` and `RestartReadbackDeterministic` preserve restart/readback determinism: after restart, full durable records read `accepted`; non-durable/partial paths are collapsed to absent and never read accepted.

### Attempt 3 Commands Run

- `pwd -P`: exit 0; `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission`.
- `test -s ... STATE.md proof-review.md proof-findings.jsonl proof-repair-guide.md contract-verification-review.md proof-obligations.planned.jsonl AtomicAcceptedRunAdmission.tla AtomicAcceptedRunAdmission.cfg`: exit 0.
- `jq -c . .beads/vb-core-atomic-admission/proof-findings.jsonl >/dev/null && jq -c . .beads/vb-core-atomic-admission/proof-obligations.planned.jsonl >/dev/null`: exit 0.
- `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`: exit nonzero; environment/tooling failure `java.io.IOException: Disk quota exceeded` while writing temp standard module/state metadata.
- `java -Djava.io.tmpdir=/tmp/opencode/vb-core-atomic-admission-tlc/tmp -cp /home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tla2tools.jar tlc2.TLC -metadir /tmp/opencode/vb-core-atomic-admission-tlc/states -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`: exit nonzero; same disk quota failure in `/tmp/opencode` metadata.
- `java -Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission/verification/tla/.tlc-states/tmp -cp /home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tla2tools.jar tlc2.TLC -metadir /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission/verification/tla/.tlc-states/states -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`: exit 0; TLC completed with no errors, 7,964 states generated, 1,100 distinct states found, temporal properties checked, depth 12.
- `verus verification/verus/accepted_run_atomic_admission.rs`: exit 0; `verification results:: 6 verified, 0 errors`.
- `rg "CHECK_DEADLOCK|Restart ==|RestartReadbackDeterministic|EventuallyRestartReadbackAfterCommit|WF_vars\\(Restart\\)|PROPERTY" verification/tla/AtomicAcceptedRunAdmission.*`: exit 0; showed restart invariant/property/fairness and no `CHECK_DEADLOCK` match.
- `rm -rf verification/tla/.tlc-states && test ! -e verification/tla/.tlc-states`: exit 0; generated TLC metadata removed.
- `rm -f accepted_run_atomic_admission && test ! -e accepted_run_atomic_admission`: exit 0; generated TLC byproduct removed.

### Attempt 3 Result

- `TLA-ATOM-001`: PASS for executable State 5 repair. TLC checked the repaired restart/readback model under the official config with deadlock checking enabled by default.
- `VERUS-PRE-001` through `VERUS-ERR-006`: PASS regression; no Verus artifact changes in attempt 3.
- Later Kani/fuzz/Miri/mutation/static/integration/API obligations remain unchanged from attempt 2: not claimed as passed by State 5.
