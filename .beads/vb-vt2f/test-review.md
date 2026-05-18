# Test Review — vb-vt2f Public Surface Audit Evidence Repair

STATUS: APPROVED
PUBLIC_SURFACE_AUDIT: PASS

## Startup Evidence

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: lines 63-67 require contract parity and exact error assertions; lines 161-166 require integration tests to use public API only; lines 149-150 allow loops/helpers/local mutability unless they hide assertions or nondeterminism.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same content observed; this agents copy wins on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`: lines 13-20 require traceable behavior evidence; lines 32-48 allow bounded generated/table/helper coverage when assertions stay exact; lines 94-110 require explicit Given preconditions.

## Scope

- Bead: `vb-vt2f` only.
- Sublane: State 9 public-surface audit evidence repair after State 11 rejection.
- Formal gap repaired: `.beads/vb-vt2f/formal-verification-report.md:36-39` required `.beads/vb-vt2f/test-review.md` with exact `PUBLIC_SURFACE_AUDIT: PASS` for `PRE-001` and `INV-002`.
- Reviewed executable evidence:
  - `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`
  - `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs`

## Public Surface Audit Findings

- PASS: `contract.md:30` (`PRE-001`) requires scenarios to drive exported `vb_runtime`/`vb_core` APIs, not private modules or crate-local helpers. `vb_vt2f_direct_runtime_api_acceptance.rs:6-21` imports only `vb_core::*`, `vb_runtime::*`, and standard library items.
- PASS: `contract.md:54` (`INV-002`) requires evidence not to pass by observing or mutating private shard internals. The direct API scenarios construct public `Runtime` values and assert public facade/trace/journal/counter/snapshot outcomes through public calls such as `Runtime::submit_direct`, `tick_all`, `snapshot_run`, `list_active_runs`, `complete_action_with_output`, `fail_action`, `answer_ask`, `list_events`, `drain_trace`, `collect_metrics`, and `shutdown_graceful`.
- PASS: `vb_hxm0_acceptance_catalog.rs:3-5` imports the public workspace test catalog module; `vb_hxm0_acceptance_catalog.rs:141-166` proves `VB-BDD-CATALOG-004` points at the executable vt2f direct API target and is not deferred.
- PASS: `vb_hxm0_acceptance_catalog.rs:331-347` includes an explicit catalog rejection oracle for a private/helper-coupled public surface (`CatalogValidationError::PrivateSurface`).
- PASS: no `use crate::`, `crate::`, `super::`, local `mod`, `include!`, `#[path]`, `pub(crate)`, or `pub(super)` usage was found in either evidence file.

## Commands / Raw Evidence

Workdir for every command: `/home/lewis/src/bd-vb-vt2f-bdd`.

```text
pwd -P && RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp rtk grep -n "use crate::|crate::|super::|#[[:space:]]*path|include!|mod [A-Za-z0-9_]+;|pub\(crate\)|pub\(super\)" "crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs" "crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs"; rc=$?; if [ "$rc" -eq 1 ]; then printf 'PRIVATE_SURFACE_SCAN: PASS_NO_MATCH\n'; else exit "$rc"; fi
/home/lewis/src/bd-vb-vt2f-bdd
0 matches for 'use crate::|crate::|super::|#[[:space:]]*path|include!|mod [A-Za-z0-9_]+;|pub\(crate\)|pub\(super\)'
PRIVATE_SURFACE_SCAN: PASS_NO_MATCH
```

```text
pwd -P && rtk grep -n "^use |^const DIRECT_API_TARGET|Runtime::|vb_runtime::|vb_core::|velvet_ballastics_workspace_tests::acceptance_catalog|VB-BDD-CATALOG-004|PrivateSurface" "crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs" "crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs"
/home/lewis/src/bd-vb-vt2f-bdd
36 matches in 2 files, including:
crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs:3: use velvet_ballastics_workspace_tests::acceptance_catalog::{...}
crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs:22: "VB-BDD-CATALOG-004"
crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs:149: .filter(|scenario| scenario.id == "VB-BDD-CATALOG-004")
crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs:344: Err(CatalogValidationError::PrivateSurface { ... })
crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:6-21: imports from vb_core and vb_runtime public crates
crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:23: const DIRECT_API_TARGET
```

```text
TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballastics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance
Summary [   0.003s] 13 tests run: 13 passed, 0 skipped
```

```text
TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog
Summary [   0.003s] 13 tests run: 13 passed, 0 skipped
```

## Decision

The required public-surface audit evidence for `PRE-001` and `INV-002` is approved. The direct API and catalog scenarios use public system surfaces only and do not rely on private helpers as primary evidence.

Next route: return to State 11 formal-verifier for the remaining required evidence check.
