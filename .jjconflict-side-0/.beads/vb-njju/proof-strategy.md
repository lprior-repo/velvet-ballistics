# vb-njju State 4 Proof Strategy

## Scope

Plan executable proof obligations for `vb-njju`: BDD mutation/fuzz/property closure scenarios. This plan writes no production, test, proof, harness, model, dependency, or CI code.

Inputs read:

- `.beads/vb-njju/contract.md`
- `.beads/vb-njju/domain-model-review.md`
- `.beads/vb-njju/tla-spec.md`
- `.beads/vb-njju/lean-contract.md`
- `.beads/vb-njju/verification-layers.md`
- `.beads/vb-njju/proof-obligations.jsonl`
- `.beads/vb-njju/traceability-matrix.jsonl`
- `.beads/vb-njju/delivery-scope.jsonl`
- `.beads/vb-njju/codebase-map.md`

## Discovery evidence

Executed from isolated workspace `/home/lewis/src/femdation-vb-njju`:

```bash
pwd -P
test -s ".beads/vb-njju/contract.md"
test -s ".beads/vb-njju/traceability-matrix.jsonl"
test -s ".beads/vb-njju/delivery-scope.jsonl"
rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" \
  "crates/workspace_tests/src/acceptance_catalog.rs" \
  "crates/workspace_tests/tests/vb_c3k9_current_api_mutation_plan.rs" \
  "crates/vb_codegen/src/proptests.rs" \
  "crates/vb_storage/src/proptests.rs" \
  "crates/vb_runtime/src/engine/tests.rs" \
  "fuzz/src/lib.rs" "fuzz/src/bin" ".moon/tasks/all.yml" "docs/current-api-mutation-plan.md"
rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" \
  "crates/workspace_tests/src/acceptance_catalog.rs" \
  "crates/workspace_tests/tests/vb_c3k9_current_api_mutation_plan.rs" \
  "crates/vb_codegen/src/proptests.rs" \
  "crates/vb_storage/src/proptests.rs" \
  "crates/vb_runtime/src/engine/tests.rs" \
  "fuzz/src/lib.rs" "fuzz/src/bin" ".moon/tasks/all.yml" "docs/current-api-mutation-plan.md"
```

Result summary: workspace path confirmed; required input files present; scoped discovery found proptest use in `vb_codegen`, `vb_storage`, and `vb_runtime`, fuzz targets in `fuzz/src/bin`, unsafe-boundary references, Moon gate wiring, mutation plan tests, and existing `unwrap`/`expect` in test/proptest/fuzz code. No `kani::`, `loom::`, Flux, Verus `requires/ensures`, or proof functions were discovered in scoped delivery files.

## Risk classification

| Risk class | Classification | Trigger |
| --- | --- | --- |
| Temporal/state-machine | Not applicable | No scheduler, retry lifecycle, lease, queue, or temporal liveness behavior introduced by this bead. |
| Rust-local invariant | Applicable | Evidence predicates must reject weak catalog, mutation, fuzz, property, and release-gate states. |
| Bounded state | Applicable | Fuzz target set is finite: `yaml_events`, `ipc_frame`, `journal_event`, `compiled_ir`; boundary target set is inventory-derived. |
| Refinement/type-state | Applicable via property tests | Generated Rust vs IR parity must include taint, result, slot/signal, journal/event signature, and typed errors. |
| Concurrency | Not applicable | No spawned task, async runtime, lock, or scheduling scope in delivery artifacts. |
| Unsafe/UB | Applicable as release gate | Unsafe/decoder/binary boundary evidence must be fuzz-covered or explicit blocker/follow-up. Miri is planned as a regression gate, not as exhaustive proof. |
| Untrusted input | Applicable | Fuzz lanes exercise hostile YAML, IPC, journal, compiled IR, and boundary payloads. |
| Dependency/supply-chain | Not applicable | No dependency file edits are planned by this state. |
| Performance | Not applicable | No speed claim or benchmark acceptance criterion. |
| Release-critical gates | Applicable | All five delivery-scope rows are `release_critical:true`. |

## Required verifier lanes

1. BDD/catalog cargo tests: exact workspace acceptance tests.
2. Mutation gates: `moon run :mutants-smoke` plus scoped `cargo mutants` for the vb-njju test.
3. Fuzz gates: `moon run :fuzz-smoke`, `cargo fuzz build --target x86_64-unknown-linux-gnu`, and exact hostile seed/run evidence for the required targets.
4. Property gates: `vb_codegen`, `vb_storage`, and targeted runtime property cargo tests.
5. Unsafe-boundary gate: boundary inventory cargo tests plus fuzz/release-failure assertions.
6. Miri regression gate: `moon run :miri` for release safety optics; not proof of full UB absence.
7. Coverage/release regression gate: `moon run :coverage` and `moon ci` after local obligations are satisfied.
8. Static traceability gate: JSONL validation for planned obligations and traceability.

## Explicit waived or non-applicable lanes

- TLA+: `not_applicable`; finite fail-closed evidence predicates, no temporal behavior. Waiver requires independent review acceptance.
- Lean/Aeneas/Hax: `not_applicable`; no theorem-kernel algebraic proof target.
- Verus: `waived unless State 5 adds non-trivial pure classifiers`; if added, create Verus obligations before acceptance.
- Kani: `not_applicable` for current evidence-planning scope; no bounded model harness target is demanded unless State 5 introduces non-trivial parser/classifier state with structural input risk.
- Flux: `not_applicable`; no refinement annotation scope in delivery files.
- Loom: `not_applicable`; no concurrency or memory-ordering scope.

## State 5 execution rule

State 5 must not claim pass until the exact command listed per planned row executes and captures the stated evidence. Blocked tooling must be recorded as `blocked_tooling` with raw command output and a follow-up bead/blocker.

## State 4 repair-2 updates after State 6 rejection

- **PO-004 mutation command repair-2, superseded by repair-3 below.** The rejected command used unsupported `cargo mutants --test`; repair-2 moved to `crates/vb_runtime/src/admission.rs` with `--test-package velvet-ballistics-workspace-tests`, but State 6 later proved that command still baselined `vb_runtime` and failed before testing mutants. The current authoritative PO-004 command is in the repair-3 section and JSONL row.
- **PO-008 fuzz command normalized.** The planned exact command now includes `--target x86_64-unknown-linux-gnu` for each required fuzz target to avoid local musl + ASan static libc incompatibility: `for target in yaml_events ipc_frame journal_event compiled_ir; do cargo fuzz run "${target}" --target x86_64-unknown-linux-gnu -- -runs=1; done`.
- **PO-010 evidence tightened.** The command is preserved because no exact renamed working filter is known from State 4 artifacts, but expected evidence now rejects `0 tests` output. State 5 must repair the property registration/filter or return `BLOCK_LOCAL` with raw `--list`/filter output.
- **Raw evidence required for PO-001..PO-017.** Every executable obligation now names a raw log path under `target/test-output` or `target/fuzz-smoke`; summary-only PASS claims remain invalid.

## State 4 repair-3 updates after State 6 rejection

- **PO-004 mutation command repaired again.** Raw State 5 evidence showed the repair-2 command discovered 56 `crates/vb_runtime/src/admission.rs` mutants but failed before mutation execution because cargo-mutants baselined `vb_runtime` while the oracle test target lives in `velvet-ballistics-workspace-tests`. The planned command now uses installed-tool-supported workspace-test routing: `TMPDIR=/tmp/opencode cargo mutants --package vb_runtime --test-workspace true --file crates/vb_runtime/src/admission.rs --timeout 60 --jobs 2 --output target/test-output/po-004-mutants -- --test vb_ssei_verification_admission_acceptance`.
- **PO-004 discovery evidence for the repaired route.** `rtk cargo test --workspace --test vb_ssei_verification_admission_acceptance --no-run` exited 0 in the isolated workspace, proving the admission acceptance oracle is addressable as a workspace test target. `cargo mutants --package vb_runtime --test-workspace true --file crates/vb_runtime/src/admission.rs --list` listed 56 admission mutants, including `admit_artifact_run`, `admit_run`, `validate_accepted_artifact_envelope`, `map_artifact_envelope_error`, `first_missing_idempotency_attestation`, and `check_capability` mutants.
- **PO-004 evidence standard tightened.** State 5 must persist raw cargo-mutants stdout/stderr at `target/test-output/PO-004-cargo-mutants-admission.log` and reports under `target/test-output/po-004-mutants`. The log must show a workspace-test baseline for `vb_ssei_verification_admission_acceptance`, nonzero mutants tested, and admission-branch/evidence-classification mutants killed. `diagnostic.rs` smoke is forbidden as substitution.
- **PO-004 fallback if cargo-mutants cannot route workspace tests.** If `--test-workspace true` still cannot execute the workspace oracle, State 5 must return `BLOCK_LOCAL_RELEASE` with raw output or add an approved package-local `vb_runtime` integration oracle that mirrors the public `vb_ssei_verification_admission_acceptance` assertions, then rerun cargo-mutants against `crates/vb_runtime/src/admission.rs`. No pass may be claimed until nonzero mutants are tested and required mutants are killed.
- **Preserved prior repairs.** PO-008 remains the GNU-target fuzz command: `for target in yaml_events ipc_frame journal_event compiled_ir; do cargo fuzz run "${target}" --target x86_64-unknown-linux-gnu -- -runs=1; done`. PO-010 still rejects zero selected tests and requires nonzero deterministic replay property execution.
