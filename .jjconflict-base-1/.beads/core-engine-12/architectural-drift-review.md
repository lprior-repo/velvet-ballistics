# Architectural Drift Review: core-engine-12

Status: COMPLETE

## Disposition

The architectural-drift rejection is partly valid for changed-surface drift, not only pre-existing debt.

The drift review demands explicit follow-up disposition for every changed oversized file. `vb_codegen/src/lib.rs` and `vb_core/src/replay/step.rs` were already oversized before this branch, but this branch materially increased both files. I applied the smallest safe mitigation available inside the changed surface: the generated storage helper template was extracted from `vb_codegen/src/lib.rs` into an adjacent include file. This reduces the primary oversized production source file without changing generated output semantics.

One additional low-risk test-layout mitigation was applied after the first disposition: the large `vb_core/src/replay/step.rs` inline test module was extracted into `vb_core/src/replay/step_tests.rs` while preserving private child-module access for branch coverage. Production replay dispatch stayed in `step.rs`.

Changed oversized files requiring follow-up disposition:

| File | Drift disposition |
|---|---|
| `crates/vb_codegen/src/lib.rs` | Oversized production codegen surface. Partly mitigated in this branch by extracting the generated storage helper template into `generated_storage_helpers.rs.txt`; remaining emitter split must be follow-up. |
| `crates/vb_codegen/src/proptests.rs` | Oversized generated-mode property-test surface. Follow-up should split by generated runtime concern and keep each property cluster near the behavior it protects. |
| `crates/vb_codegen/src/tests.rs` | Oversized codegen unit-test surface. Follow-up should split helper, expression, control-flow, and suspension tests into focused modules without changing behavior. |
| `crates/vb_codegen/tests/compile-fail/pass/minimal_workflow.rs` | Oversized compile-pass fixture. Follow-up should decompose fixture coverage into smaller pass cases or generated fixture fragments while preserving compile-pass intent. |
| `crates/vb_core/src/replay/mod.rs` | Oversized replay module surface. Follow-up should split replay orchestration from helper types and keep public replay API stable. |
| `crates/vb_core/src/replay/step.rs` | Production replay dispatch surface is now reduced by extracting inline tests into `step_tests.rs`; remaining production helper split should be follow-up only after behavior gates stay green. |
| `crates/vb_core/src/replay/step_tests.rs` | New oversized replay step test module. Intentional extraction artifact to keep production `step.rs` smaller while preserving private branch coverage; follow-up should split by Collect, linear-step, object/list, and suspension families. |
| `crates/vb_core/src/replay/tests.rs` | Oversized replay test surface. Follow-up should split by replay behavior family and keep evidence aligned to the contract under test. |
| `crates/vb_runtime/src/collect_tests.rs` | Oversized runtime collect test surface. Follow-up should split collect variant coverage into focused runtime test modules. |
| `crates/vb_runtime/src/engine/types.rs` | Oversized engine type surface. Follow-up should split stable domain types by concern only after behavior gates are green. |
| `crates/vb_storage/src/recovery/replay/summary.rs` | Oversized storage recovery summary surface. Follow-up should split summary calculation/reporting helpers while preserving recovery evidence. |
| `crates/vb_codegen/src/generated_storage_helpers.rs.txt` | New oversized generated template artifact created as the least-risk mitigation. Follow-up should split template sections once golden-output checks cover the extracted pieces. |

## Evidence

- Baseline `HEAD:crates/vb_codegen/src/lib.rs`: 1811 lines.
- Before mitigation, `crates/vb_codegen/src/lib.rs` had grown to 2937 lines by adding the generated storage helper template and related code.
- After mitigation, `crates/vb_codegen/src/lib.rs`: 2357 lines.
- New extracted template file: `crates/vb_codegen/src/generated_storage_helpers.rs.txt`, 580 lines.
- Baseline `HEAD:crates/vb_core/src/replay/step.rs`: 2256 lines.
- Current `crates/vb_core/src/replay/step.rs`: 602 lines after test extraction.
- New `crates/vb_core/src/replay/step_tests.rs`: 2166 lines, extracted from the previous inline test module plus branch-coverage tests.
- Current diff evidence after mitigation: `crates/vb_codegen/src/lib.rs` remains reduced by template extraction; `crates/vb_core/src/replay/step.rs` now contains production replay dispatch and a small `#[cfg(test)]` child-module declaration.

## Follow-Up Scope

- Split generated codegen emitters by concern: validation, step emission, expression emission, resource contracts, generated runtime templates.
- Move large generated runtime templates into explicit template files with golden-output checks.
- Split `vb_core/src/replay/step_tests.rs` into focused replay test modules while keeping production replay dispatch stable.
- Keep any follow-up separate from behavioral changes; this branch already carries substantial runtime/codegen behavior work.

## Holzmann References Read

- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
