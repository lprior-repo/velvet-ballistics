# State 10 Implementation Repair Attempt 2 — vb-qi37.10

## Reference / contract files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`
- `.beads/vb-qi37.10/test-suite-review.md`
- `.beads/vb-qi37.10/test-repair-guide.md`
- `.beads/vb-qi37.10/contract.md`
- `.beads/vb-qi37.10/test-plan.md`
- `.beads/vb-qi37.10/traceability-matrix.jsonl`

## Files changed in repair attempt 2

- `crates/vb_codegen/src/lib.rs`
- `crates/vb_codegen/src/tests.rs`
- `.beads/vb-qi37.10/implementation.md`

Prior attempt-1 workspace changes remain present in:

- `crates/vb_codegen/src/generated_storage_helpers.rs.txt`
- `crates/vb_codegen/tests/trybuild_tests.rs`

## Behavior repaired

- Restored fail-closed validation for `Together*`, `Reduce*`, `Repeat*`, and `Collect*` by returning exact `CodegenError::UnsupportedIr { feature }` before source emission.
- Removed the support-owner bookkeeping claim for unsupported `Together*` / `Reduce*` / `Repeat*` / `Collect*`; source-owner checks no longer imply semantic parity for those families.
- Removed the forbidden parity laundering path where `not_yet_implemented` from the runtime oracle plus generated stdout starting with `ok:` counted as success.
- Updated the focused family tests to assert exact fail-closed behavior for the first unsupported family node instead of pretending generated-vs-runtime parity exists.

## Power-of-Ten / zero-panic impact

- No production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable!`, unchecked indexing/slicing, unchecked arithmetic, lossy cast, or ignored fallible result was added.
- Fail-closed admission now satisfies INV-001 for unsupported final-IR families, but POST-002 remains incomplete and non-closable because those families are required unless approved scope/blocker decision revises acceptance.

## Commands run

- `rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture` — PASS, 3 passed / 358 filtered.
- `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` — PASS, 2 passed / 359 filtered.
- `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` — PASS, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` — PASS, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` — PASS, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen expression_generated_parity -- --nocapture` — PASS, 2 passed / 359 filtered.
- `rtk cargo test -p vb_codegen generated_taint_parity -- --nocapture` — PASS, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen text_helper_generated_support_or_rejection -- --nocapture` — PASS, 4 passed / 357 filtered.
- `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture` — PASS, 3 passed / 358 filtered.
- `rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture` — PASS, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen --test trybuild_tests` — PASS, 3 passed.
- `rtk cargo fmt --check` — PASS.
- `rtk cargo check -p vb_codegen --all-targets --all-features` — PASS.

## Blocker classification

- `BLOCK_LOCAL`: `POST-002` requires `Together*`, `Reduce*`, `Repeat*`, and `Collect*` to either have executable parity evidence or fail closed with a named blocker. Repair attempt 2 takes the honest fail-closed path. The bead is non-closable as complete until an owning implementation bead provides full runtime-oracle parity for those families or an approved scope/blocker decision revises acceptance.
- Named blocker: implement generated and runtime-oracle parity for `Together*`, `Reduce*`, `Repeat*`, and `Collect*`, including result/error/pc/slots/taints/step states/attempt counters/page state/materialization/capacity/journal scenarios listed in `test-plan.md`.
- Owning blocker bead: `vb-2b4g` (`codegen/runtime: Implement Repeat Reduce Together Collect parity`) was created with `discovered-from:vb-qi37.10` and `blocks:vb-qi37.10`.

## Performance layer

- No performance claim made.
- No benchmark/profiler evidence required or run.
- No second-ring assembly/IR/API/provenance claim made.

## Skipped gates / residual risk

- `moon ci` was not run because the repair request listed focused `vb_codegen` evidence commands and this state is a local implementation repair; run it before landing.
- Attempt-1 generated support code for Repeat/Reduce/Together/Collect remains present but is unreachable through public validation for those families. A future implementation should either complete parity and re-enable support or remove the dead emitters.
