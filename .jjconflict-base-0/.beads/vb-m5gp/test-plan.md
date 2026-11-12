# Test Plan: vb-m5gp — `vb_compile` Private Module Split

## Summary

- Bead: `vb-m5gp`
- Scope: derive executable test plan for splitting `crates/vb_compile/src/lib.rs` into private modules without behavior, dependency, feature, config, or public API change.
- Inputs: approved `contract-verification-review.md`, approved `proof-review.md`, `contract.md`, `domain-model-review.md`, `traceability-matrix.jsonl`, `proof-obligations.planned.jsonl`, and `proof-evidence.md`.
- Behaviors identified: 12
- Trophy allocation: 4 static/source/proof gates / 5 unit-characterization groups / 6 integration groups / 1 e2e/CI acceptance gate. Deviation from default trophy ratios is intentional: this bead is a pure structural refactor, so parity and compile-time integration evidence outrank new behavioral feature tests.
- Proptest invariants: 4 planned where pure lowering/digest/idempotency functions expose multiple-input invariants.
- Fuzz targets: 1 existing compiler/parser fuzz target remains in scope; no new fuzz target unless implementation changes parser/validation semantics.
- Kani harnesses: 1 required idempotency parity harness (`PO-014` / `KANI-001`) plus existing support harness compile reachability.
- Mutation threshold: scoped `cargo-mutants` kill rate target is >=90% for touched `vb_compile` behavior/error/lowering modules after implementation, or record a waiver if mutation runtime exceeds local budget.

## Authoritative Inputs And Approval Gates

- `contract-verification-review.md`: `STATUS: APPROVED`; confirms all contract clauses trace and repaired `KANI-001` / `PO-014` mapping is adequate.
- `proof-review.md`: `STATUS: APPROVED`; approves `PO-014` Kani idempotency parity, including the 45-case decision table and executable command.
- `contract.md`: pure refactor contract; no behavior, dependency, feature, public API, or runtime semantic change allowed.
- `domain-model-review.md`: approved architectural direction is private module files `mod_compile_core.rs`, `mod_compile_errors.rs`, `mod_compile_validation.rs`, `mod_compile_lowering.rs`; stale `compile/`, `lower/`, and `validation/` scaffolding must not be reused blindly.
- `traceability-matrix.jsonl`: maps every `PRE-*`, `POST-*`, `INV-*`, `ERR-*`, and waiver clause to tests/proofs/checks.
- `proof-obligations.planned.jsonl`: exact commands and owner states for API, behavior, source, lint, Kani, Miri, and CI obligations.
- `proof-evidence.md`: `PO-014` Kani pass, `cargo-kani 0.67.0`, final command exit 0.

## 1. Behavior Inventory

1. `vb_compile` preserves every crate-root public type/function/re-export when the private split lands.
2. `vb_compile` rejects accidental new public internal module paths when private modules are introduced.
3. `vb_compile` compiles accepted workflows to unchanged IR, artifacts, compiled digest, generated Rust, and idempotency gate decisions.
4. `vb_compile` returns unchanged typed `CompileError` / `CompileErrors` diagnostics, stable codes, observable messages, and `SourceMark` provenance for rejected inputs.
5. `vb_compile` keeps YAML/source/document/profile/name/shape validation behavior unchanged after validation code moves.
6. `vb_compile` keeps canonical and legacy public lowering behavior deterministic and parity-preserving after lowering code moves.
7. `vb_compile` keeps idempotency gate acceptance equivalent to the validation idempotency contract across the bounded decision table.
8. `vb_compile` keeps `lib.rs` as a thin facade over the four requested private modules.
9. `vb_compile` keeps module dependencies acyclic and visibility narrow after the split.
10. The split does not reuse stale unwired `compile/`, `lower/`, or `validation/` scaffolding without exact parity evidence.
11. The split does not introduce dependency, feature, Cargo, Moon, or workspace config changes.
12. The split satisfies file-length/source-governance, formatting, clippy, and `moon ci` gates.

## 2. Requirements → Tests / Proof Obligations Matrix

| Contract clauses | Required evidence | Test/proof/check command or scenario | Assertion strength |
|---|---|---|---|
| `PRE-001` | Isolated workspace only | `PO-001`: `pwd -P` guard in `/home/lewis/src/go-skill-vb-m5gp` | Exact workspace path; forbidden checkout rejected. |
| `PRE-002` | No dependency/feature/config change | `PO-002`: `git diff --exit-code -- Cargo.toml Cargo.lock crates/vb_compile/Cargo.toml .moon/` | No diff allowed. Any diff blocks or returns to contract review. |
| `PRE-003`, `INV-003`, `ERR-004` | Active implementation moved; stale scaffolding not silently wired | `PO-009`: source review in `formal-verification-report.md`; characterization tests below | Must state old scaffolding remains unwired or exact parity was proven. |
| `PRE-004`, `POST-002`, `INV-001` | Public API parity | `PO-004`: `cargo +nightly test -p vb_compile --all-targets --all-features`; `PO-005`: selected `workspace_tests` integration tests | Compile of all public use sites, cfg gates, signatures, visibility, re-exports. |
| `POST-001` | Thin facade and four private modules | `PO-003`: `cargo +nightly check -p vb_compile --all-targets --all-features`; `PO-011`: rustfmt; source review | `lib.rs` declares private `mod_compile_*`; public modules/re-exports unchanged. |
| `POST-003`, `INV-004` | Accepted input behavior parity | Characterization tests for compile/lowering/codegen/digest; `PO-006`: `moon ci`; `PO-004`, `PO-005` | Exact IR/artifact/digest/generated Rust/idempotency outcomes, not only success. |
| `POST-004`, `ERR-001..003`, `INV-005` | Rejected input diagnostic parity | Error variant unit tests; `integration_compile_error_message_quality`; `PO-007` | Exact `CompileError` variant, stable code, `SourceMark`, collection count/order where observable. |
| `POST-005` | No public internal paths | `PO-013`: `rtk grep -n '^pub mod (compile|lower|validation|mod_compile_)|^pub use (compile|lower|validation|mod_compile_)' crates/vb_compile/src || true` plus compile-fail/API review | Any new public internal path is a failure unless separately contracted. |
| `POST-006` | File-length governance | `PO-012`: line-count command for `lib.rs` and `mod_compile_*.rs` | Every checked file `<300` lines or reviewer-approved bead-linked follow-up. |
| `INV-002`, `INV-007` | Acyclic dependency and minimal visibility | `PO-008`: source review; `cargo +nightly check` | Errors leaf; validation not depending on lowering; facade re-exports only; no broad visibility leak. |
| `INV-006` | Forbidden constructs / lint / UB guard | `PO-010`: clippy; `PO-015`: Miri optional/deep; source scan | No new forbidden production constructs; Miri pass or approved waiver. |
| `POST-003` | Idempotency Kani parity | `PO-014`: `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet` | Exit 0; 45-case decision table remains non-vacuous and bound to real APIs. |
| `TLA/VERUS/THM waivers` | Pure-refactor assumptions remain true | Contract review + diff review | If semantic algorithm/temporal behavior is introduced, return to State 3. |

## 3. Trophy Allocation

| Layer | Planned coverage | Why |
|---|---:|---|
| Static / proof / source review | 4 gates | Pure refactor risk is API/source/config drift, not a new user story. |
| Unit / calc characterization | 5 groups | Needed for exact error variants, lowering outputs, digest/codegen helpers, idempotency predicates, and validation boundaries. |
| Integration | 6 groups | Highest value: compile real `vb_compile` plus real workspace consumers and fuzz/Kani surfaces. |
| E2E / acceptance | 1 gate | `moon ci` is canonical full-repo acceptance gate. |

## 4. BDD Scenarios

### Behavior 1: Crate-root public API remains reachable

- Test name: `crate_root_api_compiles_when_vb_compile_is_split`
- Given: downstream crates/tests import the public surface listed in `contract.md` and `codebase-map.md`.
- When: `cargo +nightly test -p vb_compile --all-targets --all-features` and selected `workspace_tests` compile.
- Then: every crate-root type/function/re-export resolves with unchanged signatures, visibility, cfg gates, and names.
- And: failures must identify the missing/changed public item, not be accepted as generic compile failure.
- Obligation mapping: `PO-004`, `PO-005`; clauses `PRE-004`, `POST-002`, `INV-001`.

### Behavior 2: New private modules do not become public API

- Test name: `private_compile_modules_are_not_public_when_split_is_complete`
- Given: private files `mod_compile_core.rs`, `mod_compile_errors.rs`, `mod_compile_validation.rs`, and `mod_compile_lowering.rs` exist.
- When: source scan searches for public `compile`, `lower`, `validation`, or `mod_compile_*` module declarations/re-exports added by this bead.
- Then: scan records no new public internal module path.
- And: any proposed public path requires a separate API design bead.
- Obligation mapping: `PO-013`; clauses `POST-005`, `INV-001`.

### Behavior 3: Accepted workflows compile to unchanged products

- Test name: `compile_outputs_match_baseline_when_workflow_is_accepted`
- Given: accepted representative workflows from existing `vb_compile` and workspace integration fixtures, including canonical primitives and codegen pipeline cases.
- When: public compile APIs run before and after the split through crate-root APIs.
- Then: IR, slot layout, accessor table, constant pool, compiled artifact, compiled digest, generated Rust, and idempotency gate outcome equal the baseline expected values.
- And: assertions compare concrete values/snapshots/hashes; no `is_ok()`-only assertion is acceptable.
- Obligation mapping: `PO-004`, `PO-005`, `PO-006`; clauses `POST-003`, `INV-004`.

### Behavior 4: Rejected workflows preserve typed diagnostics

- Test name: `compile_errors_match_baseline_when_workflow_is_rejected`
- Given: invalid YAML/source/document/profile/name/shape/primitive inputs already covered by error tests and integration error-message fixtures.
- When: public compile APIs reject the inputs after the split.
- Then: exact `CompileError` variant, stable diagnostic code, observable message, `SourceMark` provenance, and `CompileErrors` collection behavior match baseline.
- And: `result.is_err()` without exact variant/code/provenance assertion is rejected.
- Obligation mapping: `PO-007`; clauses `POST-004`, `ERR-001`, `ERR-002`, `ERR-003`, `INV-005`.

### Behavior 5: Validation remains validation after move

- Test name: `validation_failures_stay_validation_failures_when_validation_code_moves`
- Given: duplicate keys, strict profile violations, scalar/container limit violations, document-shape violations, workflow version errors, invalid public names, and invalid trigger/step shapes.
- When: `YamlCompiler::parse_ast`, `compile_source`, or `compile_workflow` processes those inputs.
- Then: each input returns the same validation-owned `CompileError` variant/code as baseline.
- And: no case is silently accepted, lowered, or converted to an unrelated lowering/core error.
- Obligation mapping: `PO-007`, `PO-009`; clauses `INV-003`, `ERR-004`.

### Behavior 6: Lowering remains deterministic and behavior-preserving

- Test name: `lowering_outputs_match_baseline_when_public_lower_functions_are_called`
- Given: representative valid AST steps for every public `lower_*` function and `lower_steps_to_ir`.
- When: public lowering functions run after extraction.
- Then: lowered IR/layout/branch/action/slot results match exact baseline expected structures.
- And: repeated calls with the same input return the same output.
- Obligation mapping: `PO-004`, `PO-005`, `PO-006`; clauses `POST-003`, `INV-004`.

### Behavior 7: Idempotency gate parity holds over decision table

- Test name: `idempotency_gate_matches_validation_contract_for_all_bounded_cases`
- Given: all 5 `SideEffect` variants × 3 `RetrySafety` variants × 3 `Idempotency` variants in the approved Kani domain.
- When: `is_compile_idempotency_gate_accepted` and `vb_validate::idempotency_contract::is_statically_idempotent_contract` are compared by `idempotency_gate_parity`.
- Then: both APIs agree with the independent expected decision-table acceptance result.
- And: the harness calls real crate APIs and exits 0.
- Obligation mapping: `PO-014` / `KANI-001`; clause `POST-003`.

### Behavior 8: `lib.rs` becomes a thin stable facade

- Test name: `lib_rs_declares_only_facade_and_private_split_modules_when_refactor_completes`
- Given: implementation split is complete.
- When: source review inspects `crates/vb_compile/src/lib.rs`.
- Then: it declares the four private modules and preserves existing public module/re-export surface.
- And: orchestration/error/validation/lowering implementation bodies are moved out of the facade except for intentional facade wiring.
- Obligation mapping: `PO-003`, `PO-012`; clauses `POST-001`, `POST-006`.

### Behavior 9: Module dependencies remain acyclic and narrow

- Test name: `module_dependency_direction_remains_acyclic_when_private_modules_are_added`
- Given: the four private modules compile.
- When: source review examines module imports and visibility changes.
- Then: errors remain leaf diagnostics; validation and lowering may depend on errors; core composes validation/lowering/errors; facade re-exports only.
- And: tests do not force broad public visibility for helpers.
- Obligation mapping: `PO-008`; clauses `INV-002`, `INV-007`.

### Behavior 10: Stale scaffolding is not silently activated

- Test name: `stale_scaffolding_remains_unwired_unless_exact_parity_is_recorded`
- Given: existing unwired `crates/vb_compile/src/{compile,lower,validation}` scaffolding exists.
- When: source review checks active module declarations and call paths.
- Then: those paths remain unwired or formal evidence records exact parity before reuse.
- And: any uncertainty blocks rather than becoming a hidden rewrite.
- Obligation mapping: `PO-009`; clauses `PRE-003`, `INV-003`, `ERR-004`.

### Behavior 11: Dependencies and config do not change

- Test name: `dependency_and_config_files_are_unchanged_when_split_is_pure_refactor`
- Given: the split is declared pure refactor.
- When: `git diff --exit-code -- Cargo.toml Cargo.lock crates/vb_compile/Cargo.toml .moon/` runs.
- Then: command exits 0 with no dependency, feature, workspace, or Moon config diff.
- Obligation mapping: `PO-002`; clause `PRE-002`.

### Behavior 12: Canonical CI accepts the split

- Test name: `moon_ci_passes_when_split_preserves_behavior_and_structure`
- Given: local scoped tests/proofs pass or have approved waivers.
- When: `moon ci` runs from isolated workspace.
- Then: canonical repository gate exits 0.
- And: any failure in scoped compile/codegen/error/yaml surface is treated as local/regression unless explicitly proven environmental.
- Obligation mapping: `PO-006`; clause `POST-003`.

## 5. Public API Parity Test Plan

The test writer must cover this public surface through compile tests or integration test imports from crate root:

- Types: `YamlLimits`, `YamlCompiler`, `SourceMark`, `CompileError`, `CompileErrors`, `WaitKind`, `SlotCompiler`.
- Compile/facade functions: `compile_workflow`, `compile_source`, `compile_workflow_with_contracts`, `compile_to_generated_rust`.
- Artifact/validation helpers: `build_slot_layout`, `build_accessor_table`, `build_constant_pool`, `validate_ir`, `compute_compiled_digest`, `emit_compiled_artifact`.
- Lowering functions: `lower_steps_to_ir`, `lower_set`, `lower_do`, `lower_choose`, `lower_for_each`, `lower_together`, `lower_collect`, `lower_reduce`, `lower_repeat`, `lower_wait`, `lower_ask`, `lower_finish`.
- Idempotency helpers: `is_compile_idempotency_gate_accepted`, `check_idempotency_gates`.
- Existing public modules/re-exports: `ast`, `expression`, `strict_yaml`, expression bytecode re-exports, `vb_validate::{ValidationError, ValidationResult}`, and `#[cfg(kani)]` surface.

Acceptance requirements:

- Tests must import from `vb_compile::...`, not private module paths.
- Tests must assert exact output/error when executing APIs.
- Compile-only parity is acceptable for pure type/signature reachability checks, but behavioral APIs still need concrete output/error assertions.

## 6. Compile / Lowering / Validation Characterization

### Compile characterization

Required input classes:

| Scenario | Input class | Expected output | Layer |
|---|---|---|---|
| valid minimal workflow | accepted source | exact IR/artifact/digest/codegen baseline | integration |
| valid workflow with contracts | accepted source + contracts | exact compile result and gate result baseline | integration |
| valid primitive mix | accepted source with set/do/choose/for_each/together/collect/reduce/repeat/wait/ask/finish | exact IR/action/branch/slot layout baseline | integration |
| invalid YAML/UTF-8/document | rejected source | exact `CompileError` variant/code/source mark | unit/integration |
| invalid limits | source over configured `YamlLimits` | exact limit error variant/code | unit |
| multi-error document | source with multiple expected failures | exact `CompileErrors` count/order/codes where observable | unit |

### Lowering characterization

Required input classes:

| Scenario | Input class | Expected output | Layer |
|---|---|---|---|
| each public `lower_*` happy path | valid AST primitive shape | exact lowered IR/layout fragment | unit |
| each public `lower_*` expected failure | missing/invalid field for primitive | exact `CompileError` variant/code | unit |
| branch/slot route validation | valid and invalid branch/slot shapes | exact route result or exact error | unit |
| deterministic repeat | same valid lowering input twice | equal lowered output | proptest/unit |
| legacy compatibility path | any still-reachable legacy lowering fixture | exact pre-split output | integration |

### Validation characterization

Required input classes:

| Scenario | Input class | Expected output | Layer |
|---|---|---|---|
| duplicate key | YAML duplicate key | exact duplicate-key diagnostic | unit |
| strict profile violation | unsupported strict profile shape | exact strict-profile diagnostic | unit/integration |
| scalar/container limits | boundary below/at/above limits | exact accepted result or exact limit error | unit/proptest |
| invalid public names | empty/reserved/invalid symbols | exact public-name diagnostic | unit |
| malformed workflow/trigger/step | invalid document shape | exact shape diagnostic | unit/integration |
| provenance carrying error | invalid source with location | exact `SourceMark` fields preserved | unit |

## 7. Proptest Invariants

### Proptest: deterministic public lowering

- Invariant: for any generated valid bounded step/primitive fixture accepted by public lowering, two invocations produce equal IR fragments and equal diagnostics absence.
- Strategy: generate valid bounded AST/step structures using existing domain constructors/fixtures; constrain sizes to local unit-test budget.
- Anti-invariant: invalid primitive shapes must return exact expected `CompileError`, not panic or unrelated error.

### Proptest: compiled digest stability

- Invariant: for any generated valid bounded compiled artifact/IR accepted by `compute_compiled_digest`, digest is deterministic and changes when a semantically significant field is changed.
- Strategy: generate small valid IR/artifact structures through existing builders or fixtures.
- Anti-invariant: malformed/invalid IR must be rejected by `validate_ir` with exact error before digest assumptions are made.

### Proptest: YAML limit boundary classification

- Invariant: generated sources at or below `YamlLimits` accepted by parser boundaries do not fail with limit errors; generated sources above one limit fail with the exact corresponding limit diagnostic.
- Strategy: sizes around min/default/max supported fixture boundaries.
- Anti-invariant: over-limit sources must not lower or produce codegen artifacts.

### Proptest: idempotency predicate consistency

- Invariant: generated bounded `ActionContract` idempotency-relevant fields classify consistently between compile gate and validation contract.
- Strategy: enum product over side effect, retry safety, and idempotency; arbitrary irrelevant witness fields only if safe generators exist.
- Anti-invariant: any disagreement is a failing counterexample; do not weaken the contract.
- Formal companion: `PO-014` Kani harness is authoritative for the 45-case bounded decision table.

## 8. Fuzz Targets

### Fuzz Target: `fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs`

- Input type: bytes / source text accepted by the YAML compiler boundary.
- Risk: parser/compiler panics, OOM-style pathological inputs, diagnostic misclassification, source provenance loss.
- Corpus seeds: empty input, invalid UTF-8, duplicate keys, maximum-size scalar/container boundaries, malformed strict profile, malformed workflow shape, each primitive family, idempotency-contract variants.
- Required for this bead: compile/reachability must remain intact through `moon ci` / relevant fuzz target build surface if present.
- Execution rule: full fuzz execution is waived for pure move per `PO-020`; if implementation changes parser/validator semantics, add scoped fuzz execution for this target before State 8/12 acceptance.

## 9. Kani Harnesses

### Kani Harness: `idempotency_gate_parity`

- Property: `vb_compile::is_compile_idempotency_gate_accepted` agrees with `vb_validate::idempotency_contract::is_statically_idempotent_contract` and an independent expected decision table.
- Command: `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet`
- Bound: approved 45-case decision table: 5 `SideEffect` × 3 `RetrySafety` × 3 `Idempotency`; `#[kani::unwind(8)]` per `proof-evidence.md`.
- Rationale: this is semantic parity proof for a critical gate that must survive extraction.
- Rejection rule: do not hardcode a single structural input as proof of parity; do not add assumptions in the `vb_compile` parity harness to hide disagreements.

## 10. Mutation Checkpoints

Target: >=90% scoped mutation kill rate for touched `vb_compile` modules after implementation, or explicit waiver if runtime/budget blocks mutation execution.

Critical mutations that tests must kill:

- Remove or change a crate-root public re-export → public API parity compile tests fail.
- Make `mod_compile_*` public or expose `compile/lower/validation` path → source scan/API review fails.
- Change any `CompileError` diagnostic code mapping → error variant/code tests fail.
- Drop `SourceMark` propagation from a diagnostic path → source provenance tests fail.
- Alter duplicate-key / limit / strict-profile branch conditions → validation characterization tests fail.
- Alter a public `lower_*` branch/action/slot output field → lowering characterization tests fail.
- Change compiled digest inputs/order → digest characterization/proptest fails.
- Flip idempotency acceptance for one enum combination → Kani `PO-014` fails.
- Add dependency/config changes to Cargo/Moon files → `PO-002` fails.
- Leave `lib.rs` or new modules above governance threshold → `PO-012` fails.

## 11. Combinatorial Coverage Matrix

| Group | Scenario | Input class | Expected output | Layer | Clauses / POs |
|---|---|---|---|---|---|
| API parity | crate-root compile imports | every listed public API | compile success with unchanged names/signatures | integration/static | `PRE-004`, `POST-002`, `INV-001`; `PO-004`, `PO-005` |
| API privacy | internal module path scan | `compile/lower/validation/mod_compile_*` public paths | no new public path | static/source | `POST-005`; `PO-013` |
| Compile happy path | valid workflows | accepted fixtures | exact IR/artifact/digest/generated Rust | integration | `POST-003`; `PO-004..006` |
| Compile errors | rejected workflows | invalid fixtures | exact variant/code/message/mark | unit/integration | `POST-004`, `ERR-001..003`; `PO-007` |
| Validation | shape/name/limit/profile classes | boundary invalid/valid source | exact validation accept/error classification | unit/integration/proptest | `INV-003`; `PO-007`, `PO-009` |
| Lowering | primitive families | valid/invalid AST steps | exact IR fragment or exact error | unit/proptest | `INV-004`; `PO-004..006` |
| Idempotency | enum decision table | 45 bounded cases | Kani proof success | formal | `POST-003`; `PO-014` |
| Structure | source files/modules | `lib.rs`, `mod_compile_*.rs` | private facade, acyclic imports, file lengths | static/source | `POST-001`, `POST-006`, `INV-002`, `INV-007`; `PO-003`, `PO-008`, `PO-012` |
| Config | Cargo/Moon files | dependency/config diff | no diff | static | `PRE-002`; `PO-002` |
| Full acceptance | whole repo gate | implemented split | `moon ci` exit 0 | e2e/CI | `POST-003`; `PO-006` |

## 12. Required Command Set For Test Writer / Formal Execution

Minimum scoped gates:

```bash
cargo +nightly fmt --all --check
cargo +nightly check -p vb_compile --all-targets --all-features
cargo +nightly clippy -p vb_compile --all-targets --all-features -- -D warnings
cargo +nightly test -p vb_compile --all-targets --all-features
cargo +nightly test -p workspace_tests --test integration_compile_codegen_pipeline --test integration_compile_codegen_runtime_e2e --test integration_compile_error_message_quality --test integration_validate_yaml_parsing
cargo kani --package vb_compile --harness idempotency_gate_parity --quiet
moon ci
```

Source/config gates from planned obligations:

```bash
git diff --exit-code -- Cargo.toml Cargo.lock crates/vb_compile/Cargo.toml .moon/
python -c 'from pathlib import Path; files=[Path("crates/vb_compile/src/lib.rs")]+sorted(Path("crates/vb_compile/src").glob("mod_compile_*.rs")); counts={str(p): sum(1 for _ in p.open()) for p in files}; print(counts); bad={k:v for k,v in counts.items() if v>=300}; raise SystemExit(1 if bad else 0)'
rtk grep -n '^pub mod (compile|lower|validation|mod_compile_)|^pub use (compile|lower|validation|mod_compile_)' crates/vb_compile/src || true
```

Optional/deep gate:

```bash
cargo +nightly miri test -p vb_compile
```

## 13. Assertion Rules For Test Writer

- No test may assert only `is_ok()` or `is_err()` for behavior parity; assert exact value, exact error variant, exact diagnostic code, exact relevant message/provenance, or exact snapshot/hash.
- Prefer public crate-root APIs for all parity tests.
- Do not assert private module layout except in source-structure tests explicitly tied to `POST-001`, `POST-005`, `POST-006`, `INV-002`, and `INV-007`.
- Prefer real downstream integration tests over mocks.
- If a test must use a fixture, the fixture must represent a named contract behavior and not simply exercise a moved function.
- If implementation changes semantics rather than moving code, stop and return to contract/proof planning; do not update tests to bless the change.

## Open Questions

- None blocking. This plan assumes the implementation remains a pure private-module split. Any semantic change to validation, lowering, digest, artifact, idempotency, parser behavior, public API, dependencies, config, or concurrency invalidates this plan and requires returning to earlier states.
