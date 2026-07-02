# Proof Strategy — vb-815l8

**Bead**: `vb-815l8` — Tests: replace tautological recovery fault-tolerance assertion (P1 bug)
**Phase**: 4 (proof-planner) | **Attempt**: 1-of-1
**Risk tags**: `user-visible-behavior` (production contract), `public_api`, `persistence`, `parser/codec` (lock-in), `documentation` (comment cleanup)
**Date planned**: 2026-07-01
**Controller**: femdation
**Bead scope**: TEST-ONLY; one-line assertion replacement + one-line import + comment cleanup
**Behavior-affecting obligations**: **false** (the test must pass before and after the change; production code is read-only)

---

## 1. Current State Analysis

### 1.1 Pre-wiring (existing canonical coverage)

| Component | Location | Status |
|---|---|---|
| `DurableFrameRecoveryBoundary::hydrate_run_frame` | `crates/vb_runtime/src/recovery.rs:99-106` | Production method, returns `Err(RuntimeError::InvalidRecoveryHydration)` for every `RecoveryFrameSeed` |
| `reject_unsupported_live_frame_state` | `crates/vb_runtime/src/recovery.rs:109-115` | Returns `Err(InvalidRecoveryHydration)` when `cannot_resume_state().is_resumable()` is false |
| `empty_recovered_frame` | `crates/vb_runtime/src/recovery.rs:117-125` | Maps `RunFrame::new` failure to `Err(InvalidRecoveryHydration)` (second gate) |
| `RecoveryCannotResumeState::from_seed` | `crates/vb_storage/src/recovery/types.rs:949-957` | Unconditionally applies `mark_missing_components(MissingRunStateComponents::ALL)` (forbidden to mutate) |
| `MissingRunStateComponents::ALL` | `crates/vb_storage/src/recovery/types.rs:809` | Const bit mask; locks every seed to non-resumable |
| `RuntimeError::InvalidRecoveryHydration` | `crates/vb_runtime/src/error/mod.rs:72-73` | Unit variant; `PartialEq` via unit-tag 10 at `equality.rs:3-28` |
| Canonical assert_eq! pattern | `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` | 8 unit-test sites lock the same contract with typed assertions |
| Reference import style | `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs:13` | `use vb_runtime::RuntimeError;` precedent in the same crate |
| `workspace_tests` dev-dependency on `vb_runtime` | `crates/workspace_tests/Cargo.toml:43` | Authorizes the new import |

### 1.2 The P1 bug (the only in-scope mutation site)

File: `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:79`
Test: `recovery_from_corrupt_snapshot_sequence_is_detected` (line 46)
Current (tautological): `assert!(result.is_ok() || result.is_err()); // boundary is permissive on empty seed`
Replacement: typed `assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...");`

Plus: one import at lines 7-13 (`use vb_runtime::RuntimeError;`) and two comment blocks at lines 75-78 (replace the two false claims that contradict `RecoveryResumeStatus::CannotResume`).

### 1.3 Forbidden mutations (explicit)

| Excluded | Reason |
|---|---|
| `crates/vb_storage/src/recovery/types.rs:949-957` (`from_seed` `mark_missing_components(ALL)`) | Production code; this bead is TEST-ONLY |
| `crates/vb_runtime/src/recovery.rs` (boundary) | Production code; this bead is TEST-ONLY |
| `crates/vb_storage/src/recovery/types.rs` (any line) | Production code; the from_seed mark is structural to the test outcome |
| `Cargo.toml`, `[[test]]` entries, source-length exceptions | No build impact; file is auto-discovered from `tests/` |
| Five out-of-scope tautological assertions in adjacent files | Covered by other beads per `codebase-map.md` §7 |

### 1.4 Why the existing canonical coverage is sufficient

- The contract `hydrate_run_frame` returns `Err(InvalidRecoveryHydration)` for every `RecoveryFrameSeed` is **already** locked in by 8 unit tests at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492`.
- These 8 unit tests directly cover the same `hydrate_run_frame` call path, with multiple seed shapes (frame-minimal-state, inconsistent-seed, unsupported-action-payloads, slot-value-and-taint, summary-only, factory-frame-seed, pending-action).
- The replacement assertion at `integration_runtime_storage_fault_tolerance.rs:79` adds a `workspace_tests`-level witness (cross-crate) but does not introduce new contract surface.
- Per `proof-seeds.jsonl ps-vb815l8-001`: "no new proptest required; existing canonical coverage is sufficient. Verus/Kani/Flux are NOT in scope per bead scope."

---

## 2. Risk Classification

| # | Risk | Severity | Lane | Status |
|---|---|---|---|---|
| R1 | **Tautological assertion masks boundary regression** (the P1 bug) | P1 | cargo-test (lock-in) | mitigated by replacement assertion |
| R2 | **Comments contradict production invariant** at lines 75-78 | P2 (doc) | source-lint (truthfulness) | mitigated by comment cleanup |
| R3 | **Test-name intent mismatch** (`recovery_from_corrupt_snapshot_sequence_is_detected` vs. body) | P3 | none (out of scope) | flagged for `test-writer` follow-up |
| R4 | **PartialEq on `RuntimeError` discrimination safety** | precondition | none (existing) | already covered by `vb_runtime::error::equality` tests |
| R5 | **Storage `from_seed` mark-all invariant** | precondition | none (existing) | already covered by 8 canonical unit tests |
| R6 | **`RunFrame::new` second-gate invariant** | precondition | none (existing) | already covered by canonical unit tests |

**No concurrency risk** — the test is single-threaded, no `async`, no `Mutex`, no `tokio`.
**No unsafe risk** — both `crates/vb_runtime/src/recovery.rs:1` and the target test file have `#![forbid(unsafe_code)]`.
**No temporal/scheduling risk** — no spawn, no schedule, no ordering surface.
**No hostile-input risk** — the test fixture is a manually-constructed `RecoveryFrameSeed`; no parser surface.
**No refinement/type-state risk** — the replacement is the most-refined possible form for unit-variant equality.
**No performance risk** — the typed `assert_eq!` is on a no-failure path; zero runtime cost vs. the tautological assertion.

---

## 3. Selected Verifier Lanes

Per the controller directive: **Lanes: cargo-test, source-lint**. All other lanes are `not_applicable` with concrete evidence.

### Lane A: cargo-test (lock-in evidence)

**Scope**: Run the targeted test to prove the new typed assertion is reachable, evaluates to `Err(InvalidRecoveryHydration)`, and that the test does not regress. Run the broader `workspace_tests` package to prove no neighbor test regresses.

**Why this is the primary lane**:
- The bead is a test-only fix; the test outcome is the evidence.
- The replacement is pattern A (`assert_eq!`) from `codebase-map.md §1.3`, mirroring `crates/vb_runtime/src/recovery/tests.rs:55-57`.

**Commands** (exact):
- `cargo test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance recovery_from_corrupt_snapshot_sequence_is_detected`
- `cargo test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance`

**Expected evidence**:
- Targeted test passes with the new typed assertion.
- Both neighbors at lines 30-42 (`recovery_from_empty_journal_returns_no_recovery_data`) and lines 83+ (`unsupported_recovery_state_union_combines_flags`) continue to pass.

### Lane B: source-lint (Holzman-rust + clippy + source-length)

**Scope**: Prove the new import compiles under `holzman-rust` source lint, no warnings, no new clippy lints, no source-length drift, no panic surface, no ignored fallible results.

**Why this is the second lane**:
- Per `AGENTS.md` Engineering Rules: source lint is zero-tolerance.
- Per `.moon/tasks/all.yml:46-62` the canonical `lint-src` command runs `cargo clippy --quiet --workspace --lib --bins --examples --all-features` with `-D warnings`, `-D clippy::unwrap_used`, `-D clippy::expect_used`, `-D clippy::panic`, etc.

**Commands** (exact):
- `moon run :lint-src`

**Expected evidence**:
- Zero warnings, zero errors.
- The new `use vb_runtime::RuntimeError;` import resolves and does not introduce `clippy::redundant_pub_crate` or `clippy::single_component_path_import` warnings.
- The replacement `assert_eq!` does not trigger `clippy::eq_op` (the operands differ: `Result<RunFrame, RuntimeError>` vs. the typed `Err(...)`).
- The new comments do not violate `clippy::doc_lazy_continuation` or `clippy::empty_line_after_doc_comments`.

### Lane C: source-length (sub-gate of source-lint)

**Scope**: Confirm the file remains on the over-300-line exception list; the new import and replacement assertion add ~2 net lines and do not change exception status.

**Why this is a sub-gate**:
- Per `codebase-map.md §5 Q4` and `hazard-analysis.md H-005` the file is 359 lines (vs. 346-line baseline) and is on the `split-or-retire-before-release` exception list.

**Command** (exact):
- `bash scripts/check-source-length.sh`

**Expected evidence**:
- The file remains on `vb-jpq7.47|split-or-retire-before-release` exception.
- Net line delta: +1 (import) and +5 (assert_eq! multi-line) - 1 (single-line assertion) = +5. File is 359 + 5 = 364 lines after edit. Still under 400 lines (default test cap).
- The `check-source-length.sh` exit code is 0.

---

## 4. Non-Applicable Lanes (with evidence)

| Lane | Verdict | Evidence |
|---|---|---|
| **verus** | `not_applicable` | Bead is TEST-ONLY. Production code is forbidden to mutate. Verus requires spec/proof artifacts that bind to production code; this bead does not touch production. Existing 8 unit tests at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` already prove the same `hydrate_run_frame` contract with typed assertions. Per `proof-seeds.jsonl ps-vb815l8-001`: "Verus/Kani/Flux are NOT in scope per bead scope." |
| **kani** | `not_applicable` | Bead is TEST-ONLY. No new production harness is introduced. The contract surface (`hydrate_run_frame` typed error) is exhaustively exercised by the 8 unit tests above; a Kani proof of `RuntimeError::InvalidRecoveryHydration` unit-variant equality adds no coverage beyond the existing `equality.rs:3-28` discrimination. The Kani production-binding gate is also N/A since no Verus-style production spec is in scope. |
| **flux** | `not_applicable` | Bead is TEST-ONLY. No refinement types in the changed surface. The `assert_eq!` is the most-refined possible form for a unit-variant equality. No new refinement obligation can be stated for the test file. |
| **proptest** | `not_applicable` | Per `proof-seeds.jsonl ps-vb815l8-001`: "no new proptest required; existing canonical coverage is sufficient." The 8 unit tests already cover the seed-shape space at the same crate level; the workspace_tests-level witness is a single seed (line 50-72 of the target file). Property-based generation is not needed for a one-line assertion replacement. |
| **loom** | `not_applicable` | The test is single-threaded, no `async`, no `tokio`, no `Mutex`, no `RwLock`, no `Send`/`Sync` surface. The runtime boundary is synchronous. |
| **miri** | `not_applicable` | Both `crates/vb_runtime/src/recovery.rs:1` and the target test file have `#![forbid(unsafe_code)]`. No UB paths exist in the changed surface. |
| **tla+** | `not_applicable` | No state machine, no scheduling, no interleaving, no temporal property. Per `proof-planner` SKILL.md: "TLA+ removed; temporal workflows are covered by loom + proptest." This bead has no temporal surface. |
| **fuzz / cargo-fuzz** | `not_applicable` | The test exercises a single manually-constructed seed. The 8 unit tests already cover the contract surface across 8 seed shapes. Fuzz would not add coverage to a single-typed-error contract. |
| **code-review** | `not_applicable as a separate lane` | Code review is subsumed by `test-reviewer` and `black-hat-reviewer` downstream lanes, not by proof-planner. No code-review obligation row in this plan. |
| **mutation / cargo-mutants** | `not_applicable` | Bead is TEST-ONLY and adds zero new branches. The single-line assertion replacement cannot be mutated into a still-passing-but-equivalent form except by deleting the assertion (which would mask the bug; mutation testing would catch this). However, mutation testing is not a required lane for a one-line replacement per the controller directive ("2-3 obligations"). |

---

## 5. Obligation Summary

| ID | Requirement | Verifier | Status | Behavior |
|---|---|---|---|---|
| PO-001 | `recovery_from_corrupt_snapshot_sequence_is_detected` test passes with the new typed `assert_eq!` | cargo-test | planned | false |
| PO-002 | All `integration_runtime_storage_fault_tolerance.rs` tests pass (no neighbor regression) | cargo-test | planned | false |
| PO-003 | `moon run :lint-src` produces zero warnings/errors on the modified file | source-lint | planned | false |
| PO-004 | `bash scripts/check-source-length.sh` passes; file remains on `vb-jpq7.47` exception | source-lint (sub-gate) | planned | false |

Total planned obligations: **4** (3 cargo-test, 1 source-lint) — within the controller's 2-3 obligations budget when grouped under the two verifier lanes (cargo-test has 2 test-runs, source-lint has 2 sub-gates). Behavior-affecting rows: **0** (all `behavior_affecting: false`).

---

## 6. Trusted Base (read-only for this bead)

| Trusted surface | Justification |
|---|---|
| `cargo test` runner (nextest) | Standard Rust test infrastructure, no `unsafe` |
| `cargo clippy` lint engine | Standard Rust linter, no `unsafe` |
| `holzman-rust` source lint | Project-internal lint rules; pre-existing canonical invocation |
| `PartialEq for RuntimeError` via unit tag 10 | `crates/vb_runtime/src/error/equality.rs:3-28`; unit-variant equality is exact |
| `assert_eq!` macro (std) | Standard library, panics on inequality with `Debug` payload |
| `RecoveryCannotResumeState::from_seed` (storage) | Production; forbidden to mutate. The mark-all invariant is the structural reason the test outcome is `Err(InvalidRecoveryHydration)`. |
| `DurableFrameRecoveryBoundary::hydrate_run_frame` (runtime) | Production; forbidden to mutate. The contract under test. |
| `RuntimeError::InvalidRecoveryHydration` (runtime) | Production; unit variant; equality pre-validated by existing tests |

---

## 7. Handoff

- **State 4b (proof-plan-reviewer)**: dispositions each `verifier-lane-decision` row, signs off on the 2 selected lanes and the 8 non-applicable lanes, rejects any waiver candidate.
- **State 5 (proof-writer)**: no proof/harness artifacts to author. The lane is cargo-test + source-lint, both of which are existing infrastructure. Skip if no new harness is needed.
- **State 7 (proof-to-implementation)**: bridge the 4 obligations to the test file edit; map PO-001/PO-002 to the `assert_eq!` replacement, PO-003 to the import + comment cleanup, PO-004 to the source-length exception row.
- **State 8 (test-writer)**: implement the 4 edits in `contract.md §2` (import, comment cleanup, assertion replacement); run PO-001/PO-002/PO-003/PO-004 as evidence.
- **State 12 (formal-verifier)**: execute the cargo-test + source-lint commands, capture raw stdout, close the ledger.

---

## 8. Anti-Laundering Self-Check

- [x] No `assume`, `axiom`, `admit`, `external_body` in any obligation (no proof code at all).
- [x] No `cover!`-as-proof (no Kani harness).
- [x] No copied harness models without bridge row (no harness models at all).
- [x] No generic waivers; all 8 non-applicable lanes have concrete `non_applicable_evidence_refs`.
- [x] No Verus obligations; therefore no production-binding gate triggered.
- [x] No behavior-affecting waiver candidate (all obligations are `behavior_affecting: false`).
- [x] No silent omission of demanded lane — every demanded lane (cargo-test, source-lint) has at least one obligation row; every non-demanded lane has an explicit `not_applicable` row.
- [x] Source refs use `path::symbol` form (e.g., `crates/vb_runtime/src/recovery.rs::reject_unsupported_live_frame_state`).

---

## 9. Open Plan Questions for Reviewer

1. **Two obligations per lane or one?**: I have 2 cargo-test rows (PO-001 focused, PO-002 package-wide) and 1 source-lint row (PO-003) with PO-004 as a sub-gate. The controller directive says "2-3 obligations." If the reviewer prefers strictly 2 obligations, merge PO-002 into PO-001 (single `cargo test -p velvet-ballistics-workspace-tests --test integration_runtime_storage_fault_tolerance` run covers both).
2. **No proof-to-implementation-input.md**: The controller listed only 7 artifacts; the `proof-planner` SKILL.md template mentions 8 (it includes `proof-to-implementation-input.md`). I omitted the 8th per the explicit controller directive. Reviewer may flag.
3. **No waiver-candidates.md**: I emitted only the JSONL. The SKILL.md mentions both `.md` and `.jsonl`; I matched the controller's explicit list.
