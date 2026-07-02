# Proof Coverage Matrix — vb-815l8

Maps each contract clause from `.beads/vb-815l8/contract.md` to proof obligations and verifier lanes.

## Contract: C-1 — Runtime frame hydration rejects every seed

| Contract Clause | Proof Obligation | Verifier Lane | Status | Source Refs |
|---|---|---|---|---|
| `hydrate_run_frame` returns `Err(InvalidRecoveryHydration)` for any seed | PO-001, PO-002 | cargo-test | planned | `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs::recovery_from_corrupt_snapshot_sequence_is_detected` |
| Precondition: `RuntimeError::InvalidRecoveryHydration` is a unit variant (PartialEq exact) | (subsumed) | (covered by `vb_runtime::error::equality` tests) | existing | `crates/vb_runtime/src/error/equality.rs:3-28` |
| Precondition: `from_seed` marks all 13 `*_missing` flags true | (subsumed) | (covered by 8 canonical unit tests) | existing | `crates/vb_storage/src/recovery/types.rs:949-957` |
| Precondition: `RunFrame::new` rejects `step_count==0` (second gate) | (subsumed) | (covered by canonical unit tests) | existing | `crates/vb_core/src/frame/parts/impl_001_construct.rs:10-14` |

## Contract: C-2 — Boundary seed validation is invariant, not permissive

| Contract Clause | Proof Obligation | Verifier Lane | Status | Source Refs |
|---|---|---|---|---|
| Comments at lines 75-78 must not falsely claim permissiveness | PO-003 (source-lint doc-check), PO-001 (cargo-test compile) | cargo-test + source-lint | planned | `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs::lines 75-78` |
| `RecoveryResumeStatus` enum has no `Resumable` variant by design | (subsumed) | (covered by existing production code) | existing | `crates/vb_runtime/src/recovery.rs:41-57` |
| Source-length exception preserved (359 → 364 lines, still under 400-line test cap) | PO-004 | source-lint (sub-gate) | planned | `.config/source-length-exceptions.txt::200` |

## Contract: C-3 — Test uses typed assertion, not tautological `is_ok() \|\| is_err()`

| Contract Clause | Proof Obligation | Verifier Lane | Status | Source Refs |
|---|---|---|---|---|
| Line 79 contains typed `assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...")` (Pattern A) | PO-001, PO-002 | cargo-test | planned | `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:79` |
| `PartialEq for RuntimeError` is exact by unit tag 10 | (subsumed) | (covered by `vb_runtime` equality tests) | existing | `crates/vb_runtime/src/error/equality.rs:3-28` |
| Pattern A matches canonical style at `crates/vb_runtime/src/recovery/tests.rs:55-57` | PO-001 (cargo-test), PO-003 (source-lint style) | cargo-test + source-lint | planned | `crates/vb_runtime/src/recovery/tests.rs:55-57` |

## Contract: C-4 — Import is added at lines 7-13

| Contract Clause | Proof Obligation | Verifier Lane | Status | Source Refs |
|---|---|---|---|---|
| `use vb_runtime::RuntimeError;` is added to the import block | PO-001, PO-003 | cargo-test + source-lint | planned | `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs::lines 7-13` |
| `vb_runtime` is already a dev-dependency | (subsumed) | (covered by Cargo.toml lockfile) | existing | `crates/workspace_tests/Cargo.toml:43` |
| Import style matches precedent | PO-001, PO-003 | cargo-test + source-lint | planned | `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:13` |

## Coverage Summary

| Category | Total Contract Clauses | Active Obligations | Subsumed (existing) |
|---|---|---|---|
| C-1 (typed-error contract) | 1 | 2 (PO-001, PO-002) | 3 (equality, from_seed, RunFrame::new) |
| C-2 (comment cleanup) | 1 | 2 (PO-003, PO-004) | 1 (RecoveryResumeStatus enum) |
| C-3 (typed assertion pattern) | 1 | 2 (PO-001, PO-003) | 1 (PartialEq) |
| C-4 (import) | 1 | 2 (PO-001, PO-003) | 1 (dev-dep) |
| **Total** | **4** | **4** (with cross-clause sharing) | **6** |

## Lane-to-Clause Map

| Lane | Active Obligations | C-1 | C-2 | C-3 | C-4 |
|---|---|---|---|---|---|
| cargo-test | 2 (PO-001, PO-002) | ✅ | ✅ | ✅ | ✅ |
| source-lint | 2 (PO-003, PO-004) | ✅ | ✅ | ✅ | ✅ |

## Behavior-Affecting Map

| Obligation | Behavior-affecting |
|---|---|
| PO-001 | **false** (test fix; production code unchanged; test must pass before AND after) |
| PO-002 | **false** (package-wide sanity; no production mutation) |
| PO-003 | **false** (source-lint; no production mutation) |
| PO-004 | **false** (source-length sub-gate; no production mutation) |

**Total behavior-affecting obligations**: **0** — matches the controller directive "Behavior: false".

## Anti-Laundering Map

| Anti-pattern | Status |
|---|---|
| Vacuous Verus proof (no production binding) | N/A (no Verus obligation) |
| `assume`/`axiom`/`admit`/`external_body` in proof code | N/A (no proof code) |
| `cover!`-as-proof | N/A (no Kani obligation) |
| Copied harness without bridge row | N/A (no harness) |
| Generic waiver | N/A (all 8 non-applicable lanes have concrete evidence) |
| Behavior-affecting waiver | **none emitted** |
| Silent omission of demanded lane | **none** (every demanded lane has obligations; every non-demanded lane has explicit `not_applicable` row) |
| Prose-only source refs | **none** (all refs are `path::symbol` form) |
