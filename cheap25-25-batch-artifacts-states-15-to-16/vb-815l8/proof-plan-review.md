reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: proof-plan-reviewer-vb-815l8-state4-attempt1
planner_invocation_id: proof-planner-vb-815l8-state4-attempt1
review_state: 4
reviewed_at: 2026-07-01T16:30:00Z

# Proof Plan Review: vb-815l8

## Review Metadata

- **Bead**: `vb-815l8` — Tests: replace tautological recovery fault-tolerance assertion (P1 bug)
- **Bead scope (controller directive)**: TEST-ONLY; repair at `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs`
- **Forbidden mutations**: `crates/vb_storage/src/recovery/types.rs:949-957` (`from_seed`) and `crates/vb_runtime/src/recovery.rs` (boundary)
- **Verifier lanes (controller directive)**: cargo-test + source-lint only
- **Obligation budget**: 4 obligations (verified)

## Reviewed Artifacts

| Artifact | Hash (sha256) | Status |
|----------|---------------|--------|
| `proof-strategy.md` | `f2fd23fa1f9fa9b34bfc20ac56a90cf645d96f4f44b245d817cb6c35d9387f1a` | reviewed |
| `verifier-lane-decisions.jsonl` (12 rows) | `ce6148ad2473c0ac779711305ac1a9b470f873e3da98ad568e3fe1c21c6f5d9e` | reviewed |
| `proof-obligations.planned.jsonl` (4 rows) | `68ef3c48a213833437bda034cb6307718a386154e4f97f1e36e6807b2e00dd2a` | reviewed |
| `trusted-base-plan.md` | `ded41c7805b50f86b2d05577693308f2c1609b09224958013ff3245a19dabd77` | reviewed |
| `waiver-candidates.jsonl` (8 rows) | `b4775d5f7ad93a79ad500d94ec9672de935e20006a5756289c256a794999e315` | reviewed |
| `contract.md` | `fbc5f6b31fff394ef708e5d88c58a99406b2529ff5c79b811a30513cc5db46af` | reviewed |
| `proof-seeds.jsonl` (7 rows) | `42adbf7043783a738ddd2c9cd5eada12df2033c65ee4bfc6730d4e7fa39642c0` | reviewed |
| `verifier-lane-matrix.md` | `0c34e868669ff2eebfcf98e88523fdc72827d451ca4049958a64239ad946ee19` | reviewed |
| `proof-coverage-matrix.md` | `3af4824b373f9ab7dcede2b34169006555cce26b3789ae67bf8ef3caf3ce1270` | reviewed |
| `verifier-lane-review.jsonl` (12 rows) | (this review's output) | written |

## Review Summary

### Lane Decision Coverage: PASS

- 12 lane decisions (`vld-vb815l8-001` through `vld-vb815l8-012`) across 7 proof seeds.
- **4 required lanes**: cargo-test (PO-001 focused + PO-002 no-regression), source-lint (PO-003 lint-src + PO-004 source-length sub-gate).
- **8 not_applicable lanes**: verus, kani, flux, proptest, loom, miri, tla+, cargo-fuzz.
- Every required lane has at least one obligation row. Every non-required lane has an explicit `not_applicable` row with concrete `non_applicability_evidence_refs`.
- No silent omissions.

### Non-Applicability Evidence: PASS

All 8 non-applicable lanes cite concrete evidence:

- **verus / kani / flux** (`vld-005/006/007`): Bead is TEST-ONLY; production code is forbidden to mutate. Existing 8 unit tests at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` lock the same `hydrate_run_frame` contract with typed `assert_eq!` against `Err(RuntimeError::InvalidRecoveryHydration)`. Verified at runtime by direct file inspection: all 8 sites use the canonical pattern A (`assert_eq!(boundary.hydrate_run_frame(), Err(RuntimeError::InvalidRecoveryHydration), ...)`).
- **proptest** (`vld-008`): No new proptest required; existing canonical coverage is sufficient per `proof-seeds.jsonl::ps-vb815l8-001`.
- **loom / miri / tla+ / fuzz**: Test is single-threaded, sync, `forbid(unsafe_code)`, no temporal surface, no parser/hostile input. All confirmed by direct file inspection (`#![forbid(unsafe_code)]` at `crates/vb_runtime/src/recovery.rs:1` and `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:1`).

### Obligation Schema: PASS (with minor findings)

- All 4 obligations use `schema_version: proof-obligation/v1` and have `id`, `requirement_id`, `contract_clause`, `target`, `command`, `expected_evidence`, `bounds`, `assumptions`, `mode`, `owner_state`, `rerun_from`, `status`, `behavior_affecting`, `required`.
- Commands are explicit with `-p velvet-ballistics-workspace-tests` package flag (matches the verified package name in `crates/workspace_tests/Cargo.toml:2`).
- All 4 obligations are `behavior_affecting: false` (TEST-ONLY scope; assertions lock existing production behavior, not change it).
- **FIND-001 (low)**: Legacy alias `layer` is present alongside canonical `target`. Not blocking because `target` is canonical and `layer` is informational redundancy. Normalize at materialization.
- **FIND-002 (low)**: Field naming uses `bounds`/`claim` rather than canonical `model_bounds`/`domain_claim`. Semantics match; rename at materialization.
- **FIND-003 (low)**: Optional fields `workdir`, `risk`, `risk_tags`, `artifact`, `tool_metadata`, `trusted_base_refs` not populated. Workdir can be inferred from the repository root (the test crate path is unambiguous); the rest can be filled at materialization. Not blocking.

### Production Binding Plan Validation: N/A (PASS)

- The four obligations have verifiers `cargo-test` (PO-001, PO-002) and `source-lint` (PO-003, PO-004).
- **Zero Verus obligations exist.** The production-binding gate is therefore N/A.
- `production_binding: null` is correct for non-Verus obligations.

### Forbidden-Mutation Compliance: PASS

- **FORBIDDEN**: `crates/vb_storage/src/recovery/types.rs:949-957` (`RecoveryCannotResumeState::from_seed` with `mark_missing_components(MissingRunStateComponents::ALL)`). Verified at `types.rs:949-957`: production code, contains `state = state.mark_missing_components(MissingRunStateComponents::ALL);`. The plan treats this as read-only trusted base and explicitly forbids mutation in `proof-strategy.md §1.3`, `trusted-base-plan.md §2.2`, `contract.md §3`, and `proof-strategy.md §6`.
- **FORBIDDEN**: `crates/vb_runtime/src/recovery.rs` (boundary). Verified at `recovery.rs:99-115`: production code, `hydrate_run_frame` and `reject_unsupported_live_frame_state`. The plan treats this as read-only trusted base.
- **TARGETED**: `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs`. Verified at line 79: contains the tautological assertion `assert!(result.is_ok() || result.is_err()); // boundary is permissive on empty seed`. The plan replaces this with a typed `assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...")` at line 79, adds `use vb_runtime::RuntimeError;` at lines 7-13, and replaces two false comments at lines 75-78. No other lines are modified per `contract.md §4`.

### Trusted Base Plan: PASS

- 13 trusted surfaces enumerated in `trusted-base-plan.md §2.2` with file:line refs and justifications.
- All production code is locked in by 8 existing unit tests at the cited line ranges (verified by grep against `crates/vb_runtime/src/recovery/tests.rs`).
- `RuntimeError::InvalidRecoveryHydration` is a unit variant (`crates/vb_runtime/src/error/mod.rs:72-73`) with `PartialEq` via unit-tag dispatch (`equality.rs:3-28`); equality is exact and discrimination-safe.
- `vb_runtime` is already a dev-dependency at `crates/workspace_tests/Cargo.toml:43`; verified by direct file inspection. The new `use vb_runtime::RuntimeError;` import is authorized.
- Source-length exception row at `.config/source-length-exceptions.txt:200` is preserved (`vb-jpq7.47|split-or-retire-before-release`). Verified by direct file inspection.
- Default test-file cap is 400 lines (`scripts/lib-source-length.sh`); the 359-line file (364 after edit) remains well under cap.

### Non-Vacuity: PASS

- PO-001 has a concrete seed-shape bound: one `RecoveryFrameSeed` fixture at `integration_runtime_storage_fault_tolerance.rs:50-72` and one assertion outcome `Err(InvalidRecoveryHydration)`.
- PO-002 broadens to package-wide regression detection (3 tests in the file).
- PO-003 specifies exact deny-by-default lints (`-D clippy::unwrap_used`, etc.) that must pass.
- PO-004 specifies exact line delta (`+1` import, `+5` multi-line `assert_eq!`, `-1` removed single-line assertion = `+5` net, 364 lines).
- No `cover!`-as-proof (no Kani harness).
- No vacuous Verus spec (no Verus obligation).
- No `assume`/`axiom`/`admit`/`external_body` in any obligation.

### Waiver Candidates: PASS

- 8 waiver candidates, all `not_applicable` with `behavior_affecting: false`.
- No behavior-affecting waiver (none are behavior-affecting at all).
- Each waiver links back to its lane decision by id (`wv-vb815l8-001` → `vld-vb815l8-005`, etc.) — clean cross-references.

### Bridge Planning: PASS

- `proof-coverage-matrix.md` maps every proof seed (`ps-vb815l8-001` through `ps-vb815l8-007`) to verifier lanes and obligations.
- `proof-strategy.md §7` describes handoff to State 5 (proof-writer), State 7 (proof-to-implementation), State 8 (test-writer), and State 12 (formal-verifier).
- No Verus/Kani/Flux proof artifacts required (no binding targets needed).

### Review Provenance: PASS

- **Reviewer invocation**: `proof-plan-reviewer-vb-815l8-state4-attempt1`
- **Planner invocation**: `proof-planner-vb-815l8-state4-attempt1`
- Independent, non-self-approved. Planner and reviewer invocation IDs differ on every `verifier-lane-review/v1` row.
- No reviewer fields present in any planner artifact (`proof-strategy.md`, `verifier-lane-decisions.jsonl`, `proof-obligations.planned.jsonl`, `trusted-base-plan.md`, `waiver-candidates.jsonl`).
- All 12 `verifier-lane-review/v1` rows include both `planner_invocation_id` and `reviewer_invocation_id`.

### `verifier-lane-review.jsonl`: PASS

- 12 review rows (`VLR-vb815l8-001` through `VLR-vb815l8-012`) written with `verifier-lane-review/v1` schema.
- All 12 lanes have `reviewer_disposition: accepted`.
- `owner_state: 4`, `status: reviewed`.
- `planner_invocation_id` and `reviewer_invocation_id` populated on every row and differ on every row (independent review).

## Findings

| ID | Code | Severity | Description | Disposition |
|----|------|----------|-------------|-------------|
| FIND-001 | `E_SCHEMA_ALIAS_FIELD` | low | All 4 obligations carry legacy `layer` field alongside canonical `target`. The canonical field is present and `layer` is informational; behavior-affecting classification, command, and evidence are unchanged. Fix at obligation materialization: remove `layer` from all 4 PO rows. | `owner_approved_no_action` |
| FIND-002 | `E_SCHEMA_ALIAS_FIELD` | low | All 4 obligations use `bounds`/`claim` rather than canonical `model_bounds`/`domain_claim`. Field semantics are identical; only naming diverges. Fix at materialization: rename to `model_bounds`/`domain_claim`. | `owner_approved_no_action` |
| FIND-003 | `E_SCHEMA_MISSING_FIELD` | low | All 4 obligations omit optional fields `workdir`, `risk`, `risk_tags`, `artifact`, `tool_metadata`, `trusted_base_refs`. Workdir is unambiguously the repo root for `cargo test -p velvet-ballistics-workspace-tests`; the remaining fields can be filled at materialization from the proof-strategy and trusted-base-plan. | `owner_approved_no_action` |

## Verdict

The proof plan is complete, precise, and implementation-bound for the test-only P1 bug fix. All 12 lane decisions are justified with concrete evidence (file:line refs, exact commands, expected outputs). All 4 obligations have exact commands, bounds, assumptions, and expected evidence, and are uniformly `behavior_affecting: false`. The trusted base correctly enumerates production code as read-only and lists both forbidden mutations explicitly. The non-applicable lanes cite concrete evidence (8 existing unit tests at `crates/vb_runtime/src/recovery/tests.rs` that already lock the same contract with typed assertions). No behavior-affecting waivers exist. No proof-theater: the plan is mechanically executable by `formal-verifier` and the assertions are not vacuous. The three schema-drift findings are non-blocking and target the materialization phase, not plan approval.

The plan is ready for `proof-writer` (State 5) and downstream states.

**STATUS: APPROVED**

## Next Steps

1. **State 5 (proof-writer)**: No proof/harness artifacts to author (no Verus/Kani/Flux/fuzz in scope). Skip per `proof-strategy.md §7`.
2. **State 7 (proof-to-implementation)**: Bridge the 4 obligations to the test file edit; map PO-001/PO-002 to the `assert_eq!` replacement, PO-003 to the import + comment cleanup, PO-004 to the source-length exception row.
3. **State 8 (test-writer)**: Implement the 4 edits in `contract.md §2` (import, comment cleanup, assertion replacement); run PO-001/PO-002/PO-003/PO-004 as evidence.
4. **State 12 (formal-verifier)**: Execute the cargo-test + source-lint commands, capture raw stdout, close the verification ledger.
5. **Materialization cleanup**: At or before State 12, remove `layer` alias and rename `bounds`→`model_bounds`, `claim`→`domain_claim`; populate optional fields (`workdir`, `risk`, `risk_tags`, `artifact`, `tool_metadata`, `trusted_base_refs`).