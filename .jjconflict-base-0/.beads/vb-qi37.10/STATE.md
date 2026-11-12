# vb-qi37.10 Go-Skill State

Bead: `vb-qi37.10` - codegen: Complete remaining final IR coverage and parity

Source checkout: `/home/lewis/src/velvet-ballistics`

Isolated workspace: `/tmp/opencode/go-skill-vb-qi37-10`

Current state: State 10 blocked (`BLOCK_LOCAL`), repaired test suite approved as fail-closed evidence

Retry attempt: 0
Repair attempts: 3

Claim status: `bd update vb-qi37.10 --claim` succeeded from source checkout.

Path isolation evidence:

- `pwd -P` in isolated workspace returned `/tmp/opencode/go-skill-vb-qi37-10`.
- Path guard command rejected nesting under `/home/lewis/src/velvet-ballistics` with exit 0.
- `jj workspace list` includes `go-skill-vb-qi37-10: xxoyykps 795f4f64 (empty) go-skill vb-qi37.10 final IR coverage`.

Beads context note:

- Plain `bd show` in this jj workspace cannot resolve a Git repo root because jj workspaces here are not colocated with `.git`.
- Bead reality checks are run from the isolated workspace with `bd --db "/home/lewis/src/velvet-ballistics/.beads/dolt" ...` against the source server-mode Dolt database.
- `bash scripts/check-beads-server-mode.sh` passed in the isolated workspace.

Master-doc scope summary:

- P0 core durability/recovery beads are closed in `bd`.
- Remaining master-doc blockers for a fully accepted engine include generated final IR coverage/parity (`vb-qi37.10`), suspension error parity (`vb-qi37.11`), and generated-mode semantic parity evidence (`vb-gvmt`).
- This bead is the first ready blocker because `vb-qi37.11` and `vb-gvmt` depend on it.

Next gate:

- State 10 holzman-rust must implement safe Rust against approved contracts and tests, and write `implementation.md`.

State 2 evidence:

- `explore` specialist wrote `.beads/vb-qi37.10/codebase-map.md`.
- `explore` specialist wrote `.beads/vb-qi37.10/delivery-scope.jsonl`.
- `jq -c . ".beads/vb-qi37.10/delivery-scope.jsonl" >/dev/null` passed.

State 3 evidence:

- `rust-contract` specialist wrote `.beads/vb-qi37.10/contract.md`.
- `rust-contract` specialist wrote `.beads/vb-qi37.10/domain-model-review.md`.
- `rust-contract` specialist wrote `.beads/vb-qi37.10/tla-spec.md`.
- `rust-contract` specialist wrote `.beads/vb-qi37.10/lean-contract.md`.
- `rust-contract` specialist wrote `.beads/vb-qi37.10/verification-layers.md`.
- `jq -c . ".beads/vb-qi37.10/proof-obligations.jsonl" >/dev/null` passed.
- `jq -c . ".beads/vb-qi37.10/traceability-matrix.jsonl" >/dev/null` passed.

State 4 evidence:

- Proof-planner role wrote `.beads/vb-qi37.10/proof-strategy.md`.
- Proof-planner role wrote `.beads/vb-qi37.10/proof-plan-review-input.md`.
- `jq -c . ".beads/vb-qi37.10/proof-obligations.planned.jsonl" >/dev/null` passed.
- Required acceptance proof lanes are executable generated-vs-runtime parity, generated source contract scan, non-empty trybuild compile-fail, journal-signature parity, and final `moon ci`.
- TLA+/Verus/Kani are deferred follow-up lanes because no production-bound non-vacuous proof targets exist in scope.

State 5 evidence:

- Proof-writer role wrote `.beads/vb-qi37.10/proof-writer-report.md`.
- Proof-writer role wrote `.beads/vb-qi37.10/proof-evidence.md`.
- Proof-writer role wrote `.beads/vb-qi37.10/deferred-formal-lanes.md`.
- No TLA+/Verus/Kani formal artifact was created; this avoids vacuous, non-production-bound proof work.
- `pwd -P && jq -c .` over `proof-obligations.planned.jsonl`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, and `delivery-scope.jsonl` passed from the isolated workspace.

State 6 rejection evidence:

- `proof-review.md` returned `STATUS: REJECTED` because formal obligations were contradictory across canonical/planned ledgers.
- `contract-verification-review.md` returned `STATUS: REJECTED` for the same contradiction and non-executable required formal gates.
- `proof-repair-guide.md` required ledger reconciliation and concrete follow-up ownership.

State 5 repair attempt 1 evidence:

- Created concrete formal follow-up beads: `vb-w20g` (TLA+), `vb-h3fx` (Verus), `vb-mnv0` (Kani).
- Repaired `proof-obligations.jsonl`: original formal lanes are `required:false`, `status:waived`, `mode:deferred-follow-up`, with concrete follow-up bead IDs and no pass claims.
- Repaired `proof-obligations.planned.jsonl` to include canonical fields `target`, `claim`, `layer`, `checker`, and `scope`.
- Repaired `traceability-matrix.jsonl` so acceptance owners are executable/static gates and TLA/Verus/Kani are deferred follow-up only.
- `jq -c .` validation passed for repaired JSONL artifacts.
- Reviewer-targeted unresolved blocked-required-formal query returned no output.

State 6 re-review evidence:

- `proof-review.md` contains `STATUS: APPROVED`.
- `contract-verification-review.md` contains `STATUS: APPROVED`.
- `proof-findings.jsonl` is valid JSONL and records only a non-blocking summary.
- `rg '^STATUS: APPROVED$'` found approvals in both required review artifacts.

State 7 evidence:

- `test-planner` specialist wrote `.beads/vb-qi37.10/test-plan.md`.
- `test-plan.md` covers support matrix, Repeat/Reduce/Together/Collect parity, expression/accessor parity, taint parity, text helper support/rejection, generated source contract, non-empty trybuild, journal-signature parity, and final gates.

State 8 evidence:

- `test-writer` specialist modified `crates/vb_codegen/src/tests.rs` and `crates/vb_codegen/tests/trybuild_tests.rs`.
- `test-writer` specialist wrote `.beads/vb-qi37.10/test-writer-report.md`.
- Focused commands passed after test-harness-only repairs: support matrix totality, text helper support/rejection, generated source contract, repeat fail-closed regression, and trybuild tests.
- `test-writer-report.md` records that full Repeat/Reduce/Together/Collect generated-vs-runtime parity and journal-signature parity remain implementation gaps.

State 9 rejection evidence:

- `test-plan-review.md` contains `STATUS: APPROVED`.
- `test-suite-review.md` contains `STATUS: REJECTED` because required parity filters for Reduce/Together/Collect/expression/taint/journal ran zero tests and Repeat had only fail-closed evidence.
- `test-repair-guide.md` required adding failing-first executable parity tests for every required filter and strengthening owner-map enforcement.

State 8 repair attempt 1 evidence:

- Added failing-first tests for Repeat, Reduce, Together, Collect, expression/accessor, taint, and journal-signature filters.
- Strengthened owner-map enforcement to require owner test function names in `tests.rs`.
- Added raw command logs under `.beads/vb-qi37.10/repair-attempt-1-outputs/`.
- Required filters now select at least one test; parity filters fail as expected on current unsupported generated families.

State 9 re-review rejection evidence:

- `test-suite-review.md` remained `STATUS: REJECTED` after repair attempt 1.
- Remaining blockers: expression/accessor fixture construction used an unused accessor workflow, and parity tests asserted generated source substrings rather than executable generated-vs-runtime behavior.

State 8 repair attempt 2 evidence:

- Repaired expression/accessor tests to use actual generated execution and IR-derived values/taints.
- Replaced required repaired parity source-substring assertions with executable generated/runtime-oracle comparisons.
- Added raw command logs under `.beads/vb-qi37.10/repair-attempt-2-outputs/`.
- Required filters still select tests; Repeat/Reduce/Together/Collect fail at unsupported generated emission, while expression/taint/journal focused commands pass.

State 9 final review evidence:

- `test-plan-review.md` contains `STATUS: APPROVED`.
- `test-suite-review.md` contains `STATUS: APPROVED`.
- Test reviewer approved failing-first tests: unsupported final IR families now fail on real generated-emission gaps, not zero-test filters or fixture construction mistakes.

State 10 implementation attempt 1 evidence:

- `holzman-rust` specialist wrote `.beads/vb-qi37.10/implementation.md` and changed `crates/vb_codegen/src/lib.rs`, `crates/vb_codegen/src/generated_storage_helpers.rs.txt`, `crates/vb_codegen/src/tests.rs`, and `crates/vb_codegen/tests/trybuild_tests.rs`.
- `implementation.md` reports focused codegen gates passing, but also records `DEFERRED_GLOBAL` full runtime-oracle parity for Repeat/Reduce/Together/Collect and first-page/minimal Collect support only.

Post-State-10 invalidation evidence:

- State 10 changed approved tests, invalidating the previous State 9 test-suite approval under the go-skill code-change invalidation rule.
- `test-reviewer` rerun wrote `.beads/vb-qi37.10/test-suite-review.md` with `STATUS: REJECTED`.
- Blocking reasons: `crates/vb_codegen/src/tests.rs` treats `vb_core` `not_yet_implemented` oracle failures as pass when generated output starts with `ok:`, Collect is marked supported while only first-page/minimal behavior exists, required Repeat/Reduce/Together/Collect edge scenario coverage is missing, and owner-map source-substring checks do not prove executable parity.
- Required repair target: State 10 `holzman-rust` implementation repair.

State 10 implementation repair attempt 2 evidence:

- `holzman-rust` repaired the implementation to restore fail-closed validation for `Together*`, `Reduce*`, `Repeat*`, and `Collect*`.
- `crates/vb_codegen/src/tests.rs` no longer treats `not_yet_implemented` runtime-oracle failures as passing parity; unsupported-family focused tests assert exact pre-emission rejection.
- `crates/vb_codegen/src/lib.rs` rejects those unsupported families in `unsupported_node_feature` before emission.
- `.beads/vb-qi37.10/implementation.md` now classifies the bead as `BLOCK_LOCAL`: `POST-002` remains unsatisfied and the bead is non-closable until those families have executable generated-vs-runtime parity or an approved scope/blocker decision revises acceptance.

Controller verification after repair attempt 2:

- `rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture` passed, 3 passed / 358 filtered.
- `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` passed, 2 passed / 359 filtered.
- `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` passed, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` passed, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` passed, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen expression_generated_parity -- --nocapture` passed, 2 passed / 359 filtered.
- `rtk cargo test -p vb_codegen generated_taint_parity -- --nocapture` passed, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen text_helper_generated_support_or_rejection -- --nocapture` passed, 4 passed / 357 filtered.
- `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture` passed, 3 passed / 358 filtered.
- `rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture` passed, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen --test trybuild_tests` passed, 3 passed.
- `rtk cargo fmt --check` passed.
- `rtk cargo check -p vb_codegen --all-targets --all-features` passed.

Post-repair State 9 rerun evidence:

- `test-reviewer` rerun approved the repaired test suite as honest fail-closed evidence only.
- `test-suite-review.md` records `STATUS: APPROVED` for the repaired suite, with explicit completion blocker: `POST-002` still blocks bead completion.
- Required repair target remains State 10 implementation bead/follow-up for full Repeat/Reduce/Together/Collect generated-vs-runtime parity.

Beads blocker update:

- Created follow-up/blocker bead `vb-2b4g`: `codegen/runtime: Implement Repeat Reduce Together Collect parity`.
- Dependency recorded with `discovered-from:vb-qi37.10` and `blocks:vb-qi37.10`.
- Updated `vb-qi37.10` status to `blocked` with notes pointing to `vb-2b4g` and the State 10 repair evidence.
