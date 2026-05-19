# vb-njju Proof Plan Review Input

## Review target

Review `.beads/vb-njju/proof-obligations.planned.jsonl` for contract parity with State 3 artifacts and executable adequacy for State 5 proof/test/fuzz/mutation work.

## Contract clauses covered

- PRE-001/PRE-002/POST-005/INV-001: acceptance catalog rows must be complete, public-surface driven, isolated, and traceable.
- PRE-003/POST-001/INV-003: admission-branch mutation closure must reject unrelated diagnostic smoke evidence.
- PRE-004/POST-002/INV-002: fuzz smoke must run or invoke hostile seeds for `yaml_events`, `ipc_frame`, `journal_event`, and `compiled_ir`; build-only evidence is weak.
- PRE-005/POST-003/INV-004: generated-vs-IR parity must include taint.
- PRE-006/POST-004/INV-005: missing unsafe-boundary fuzz evidence must fail release unless represented as approved blocker/follow-up.
- POST-006/INV-006: clauses must be machine-traceable to executable obligations or explicit non-applicability rows.

## Planned lanes

- Cargo tests: acceptance catalog, vb-njju BDD closure, current API mutation plan, boundary inventory, property tests.
- Mutation: Moon mutants smoke plus repaired scoped `cargo mutants` against `crates/vb_runtime/src/admission.rs` using `--test-workspace true`; unrelated `diagnostic.rs` smoke remains regression-only.
- Fuzz: Moon fuzz smoke, cargo fuzz build, and target-specific hostile seed/run evidence on `x86_64-unknown-linux-gnu`.
- Property: `vb_codegen`, `vb_storage`, and targeted runtime property regression.
- Release gates: Miri, coverage, and full `moon ci` as regression closure.
- Static validation: planned JSONL plus traceability JSONL parse check.
- Waivers/non-applicability: TLA+, Lean, Verus, Kani, Flux, Loom.

## State 6 rejection repair notes

1. PO-004 repair-2 was rejected because `--test-package velvet-ballastics-workspace-tests` still baselined `vb_runtime` and tried to run the workspace-test target in the mutated package. Review the repair-3 command instead: `TMPDIR=/tmp/opencode cargo mutants --package vb_runtime --test-workspace true --file crates/vb_runtime/src/admission.rs --timeout 60 --jobs 2 --output target/test-output/po-004-mutants -- --test vb_ssei_verification_admission_acceptance`. Discovery evidence: `rtk cargo test --workspace --test vb_ssei_verification_admission_acceptance --no-run` exited 0, and `cargo mutants --package vb_runtime --test-workspace true --file crates/vb_runtime/src/admission.rs --list` listed 56 admission mutants. State 5 evidence must show a workspace-test baseline, nonzero mutants tested, and admission-branch/evidence-classification mutants killed. If it yields zero mutants or tool failure, the plan requires `BLOCK_LOCAL_RELEASE` with raw logs or the approved fallback of adding a `vb_runtime` package-local integration oracle mirroring `vb_ssei`; it does not permit `diagnostic.rs` smoke substitution.
2. PO-008 now requires `for target in yaml_events ipc_frame journal_event compiled_ir; do cargo fuzz run "${target}" --target x86_64-unknown-linux-gnu -- -runs=1; done` with raw per-target `target/fuzz-smoke/*` logs.
3. PO-010 remains State 5-owned because no exact renamed working filter is known from the artifacts. The expected evidence now rejects zero selected tests and requires State 5 to repair registration/filter or return `BLOCK_LOCAL` with raw list/filter output.
4. PO-001..PO-017 expected evidence now requires persisted raw logs under `target/test-output` or `target/fuzz-smoke`; summary-only PASS evidence is a rejection trigger.

## Reviewer questions

1. Does `--test-workspace true` satisfy the cargo-mutants workspace oracle routing requirement for mutating `vb_runtime` while running `velvet-ballastics-workspace-tests`, or must State 5 use the fallback package-local `vb_runtime` admission oracle harness?
2. Is `moon run :fuzz-smoke` expected to be upgraded to run hostile seeds, or should State 5 create a separate exact seed-run script and evidence path?
3. If generated-vs-IR taint parity requires new pure classifier code, should Verus become required before implementation acceptance?
4. Does the boundary inventory expose a machine-readable public list of required unsafe/decoder/binary boundary fuzz targets, or must State 5 add a public test-facing evidence API?

## Rejection triggers

- Any release-critical obligation marked optional without waiver.
- Any fuzz obligation that accepts `cargo fuzz build` as run evidence.
- Any mutation obligation that accepts unrelated `vb_core/src/diagnostic.rs` smoke as admission-branch closure, reports zero mutants for PO-004, or lacks raw cargo-mutants output.
- Any PO-004 cargo-mutants evidence whose baseline still invokes only `--package=vb_runtime` with `--test vb_ssei_verification_admission_acceptance` instead of routing the oracle through workspace tests or an approved package-local runtime oracle.
- Any fuzz obligation that omits `--target x86_64-unknown-linux-gnu` for PO-008 in this workspace.
- Any State 5 PO-010 rerun that exits 0 while selecting zero deterministic replay tests.
- Any executable PO-001..PO-017 claim that lacks persisted raw stdout/stderr and exit-code evidence.
- Any generated-vs-IR property obligation that omits taint parity.
- Any unsafe-boundary obligation that allows release pass with missing fuzz/blocker evidence.
- Any formal lane omitted silently instead of `not_applicable`, `waived`, or `blocked_tooling`.
