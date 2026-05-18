# Codebase Map: vb-m5gp Split `vb_compile/src/lib.rs`

## Bead

- Bead: `vb-m5gp`
- Title: Split `vb_compile/src/lib.rs` (observed 6139 lines in isolated workspace)
- Workspace: `/home/lewis/src/go-skill-vb-m5gp`
- Source file: `/home/lewis/src/go-skill-vb-m5gp/crates/vb_compile/src/lib.rs`
- Baseline: `/home/lewis/src/go-skill-vb-m5gp/.beads/vb-m5gp/baseline-report.md` says shared-parent `moon ci` passed.

## Explore Evidence

- Read `/home/lewis/src/go-skill-vb-m5gp/.beads/vb-m5gp/STATE.md` and `baseline-report.md`.
- Mapped `/home/lewis/src/go-skill-vb-m5gp/crates/vb_compile/src/lib.rs` with `grep` for public items, private functions, and test modules.
- Read representative line ranges from `lib.rs`: `1-360`, `360-1219`, `1220-2769`, `2770-3769`, `3770-4769`, `4770-6139`.
- Read existing sibling/stub files: `/home/lewis/src/go-skill-vb-m5gp/crates/vb_compile/src/compile/mod.rs`, `/home/lewis/src/go-skill-vb-m5gp/crates/vb_compile/src/lower/mod.rs`, `/home/lewis/src/go-skill-vb-m5gp/crates/vb_compile/src/validation/mod.rs`, and `/home/lewis/src/go-skill-vb-m5gp/crates/vb_compile/Cargo.toml`.
- Searched downstream Rust usage of public `vb_compile` APIs, private module paths, source-length policy, and architecture docs.

## Current `vb_compile` Module Surface

`lib.rs` currently declares:

- Public modules: `ast`, `expression`, `strict_yaml`, and `kani_idempotency_parity` under `cfg(kani)`.
- Private modules: `control_flow`, `expression_bytecode`, `references`, `schema`, `type_taint`.
- Public re-exports: `compile_expr_to_bytecode`, `compile_expr_to_bytecode_with_accessors`, `vb_validate::{ValidationError, ValidationResult}`.

Existing files `/crates/vb_compile/src/compile/mod.rs`, `/lower/mod.rs`, and `/validation/mod.rs` are not declared from active `lib.rs`; grep found no `mod compile`, `mod lower`, or `mod validation` declaration in `crates/vb_compile/src/*.rs`. Treat them as stale/unwired scaffolding unless implementation state intentionally wires or replaces them.

## Public API / Export Constraints

External callers depend on the crate-root API, not new internal module paths. Preserve these names at `vb_compile::...`:

- Types: `YamlLimits`, `YamlCompiler`, `SourceMark`, `WaitKind`, `SlotCompiler`, `CompileError`, `CompileErrors`.
- Compile/facade functions: `compile_workflow`, `compile_source`, `compile_workflow_with_contracts`, `compile_to_generated_rust`.
- Artifact/validation helpers: `build_slot_layout`, `build_accessor_table`, `build_constant_pool`, `validate_ir`, `compute_compiled_digest`, `emit_compiled_artifact`.
- Lowering functions: `lower_steps_to_ir`, `lower_set`, `lower_do`, `lower_choose`, `lower_for_each`, `lower_together`, `lower_collect`, `lower_reduce`, `lower_repeat`, `lower_wait`, `lower_ask`, `lower_finish`.
- Idempotency gate: `is_compile_idempotency_gate_accepted`, `check_idempotency_gates`.
- Expression bytecode re-exports and validation type re-exports must remain unchanged.

Downstream grep found direct usage in CLI, workspace tests, fuzz targets, benches, Kani harnesses, and vb_compile integration tests. No active external usage of `vb_compile::compile::*`, `vb_compile::lower::*`, or `vb_compile::validation::*` was found.

## Current `lib.rs` Structure by Concern

### Facade / Compile Core

- Lines `61-67`: default YAML limits and workflow version constants.
- Lines `70-180`: `YamlLimits`, `YamlCompiler`, `SourceMark`, `YamlCompiler::compile`, `YamlCompiler::parse_ast`.
- Lines `183-225`: canonical YAML error adaptation.
- Lines `237-288`: `compile_workflow` and `compile_source` orchestration.
- Lines `1222-1289`: `compile_workflow_with_contracts`, table builders, `lower_steps_to_ir` facade.
- Lines `1710-1748`: `validate_ir`, digest/artifact/codegen helpers.
- Lines `1763-1832`: idempotency gate.

### Canonical Lowering

- Lines `360-1211`: canonical AST layout, width calculation, step-name expansion, `lower_canonical_*` functions, slot/text helpers, canonical digest.
- Lines `1291-1700`: public per-primitive lowering functions plus `WaitKind`.
- Lines `1834-1956`: `SlotCompiler`, branch-route validation, shared lowering helper.
- Lines `3791-4535`: older `WorkflowBuilder`, `StepPrimitive`, `compile_step`, `compile_*` legacy lowering path marked mostly `#[allow(dead_code)]`.
- Lines `4545-4806`: low-level conversion helpers used by validation/lowering.
- Lines `4808-4838`: constant value lowering helpers.

### Errors / Diagnostics

- Lines `1958-2484`: public non-exhaustive `CompileError` enum.
- Lines `2486-2680`: stable diagnostic code mapping helpers.
- Lines `2682-2742`: public `CompileErrors` collection and `Display`/`Error` impls.
- `SourceMark` currently lives near facade lines `105-146` but is an error/diagnostic type and is used by validation helpers.

### YAML / Source Validation

- Lines `2744-3041`: error collector, UTF-8/source/document checks, duplicate-key scan, strict profile tree validation, scalar/container limits.
- Lines `3154-3790`: document shape validation, trigger validation, public name validation, top-level/step validation.
- Lines `4591-4806`: primitive field validation helpers, slot/action/branch extraction.
- Public AST validation pipeline in `YamlCompiler::parse_ast` also delegates to existing private modules `schema`, `references`, `type_taint`, and `control_flow`.

### Tests

- Lines `4840-6139`: large inline `#[cfg(test)] mod tests`, including `mod error_variant_tests;` and many tests that access private helpers across all concerns.
- Additional external tests under `/home/lewis/src/go-skill-vb-m5gp/crates/vb_compile/tests/` and `/home/lewis/src/go-skill-vb-m5gp/crates/workspace_tests/tests/` use crate-root public APIs.

## Architecture Constraints

- `/home/lewis/src/go-skill-vb-m5gp/velvet-ballistics-MASTER.md` section 28 defines `vb_compile` mandatory surface: entry points, lowering, validation, expression bytecode, diagnostics, artifact emission.
- Master line `117` requires hot functions <=25 logical lines and says complex cold validation functions must be decomposed or carry bead-linked justification and stay out of hot paths.
- Master lines `1193-1204` require the current `vb_compile` surface to remain present.
- DRIFT-5 lines `3322-3338` document duplicate validation between `vb_validate` and `vb_compile` as partially resolved; do not deepen duplication during the split.
- `vb_compile` is cold compiler code; `HashMap` and `format!` are allowed in cold parser/compiler/diagnostic paths per master cold-path notes.

## Smallest Safe Split Plan

Architectural decision needed: make `lib.rs` a stable facade and move implementation behind four private modules named as requested. Do not expose new public module paths unless a separate compatibility decision demands it.

1. Add private modules from crate root:
   - `mod mod_compile_core;`
   - `mod mod_compile_errors;`
   - `mod mod_compile_validation;`
   - `mod mod_compile_lowering;`
2. Keep existing public modules/re-exports unchanged (`ast`, `expression`, `strict_yaml`, expression bytecode, validation re-exports, Kani cfg module).
3. Move error/diagnostic surface into `mod_compile_errors`:
   - `SourceMark`, `CompileError`, `CompileErrors`, code mapping helpers.
   - Re-export from `lib.rs`: `pub use mod_compile_errors::{CompileError, CompileErrors, SourceMark};`.
4. Move validation helpers into `mod_compile_validation`:
   - strict YAML profile, duplicate key validation, document shape validation, public-name helpers, primitive field extraction helpers.
   - Expose only `pub(crate)` functions needed by core/lowering/tests.
   - Keep calls to existing `schema`, `references`, `type_taint`, `control_flow` unchanged from `YamlCompiler::parse_ast` or core wrapper.
5. Move lowering into `mod_compile_lowering`:
   - canonical layout/lowering, public `lower_*` functions, `WaitKind`, `SlotCompiler`, legacy dead-code `WorkflowBuilder` path only if tests or future code still require it.
   - Re-export crate-root public lower APIs from `lib.rs`.
6. Move facade/orchestration into `mod_compile_core`:
   - `YamlLimits`, `YamlCompiler`, `compile_workflow`, `compile_source`, `compile_workflow_with_contracts`, artifact/codegen/idempotency helpers, table builders.
   - Re-export crate-root public core APIs from `lib.rs`.
7. Move or split inline tests by concern. Lowest-risk option is concern-local test modules so private helper access remains local. If tests remain in `lib.rs`, helper visibility will need broad `pub(crate)` leakage.
8. Decide what to do with unwired `/compile`, `/lower`, `/validation` scaffolding. Smallest safe choice is not to reuse stale duplicate implementations; either delete in implementation state if unused or convert to thin compatibility wrappers only after explicit architectural approval.

## Primary Risks

- Public API break if crate-root re-exports are missed.
- Private helper visibility churn because tests currently rely on `super::*` inside `lib.rs`.
- Stale `/compile/mod.rs` duplicates active implementation and can mislead implementers; blindly wiring it would regress semantics because it lacks newer canonical primitive lowering.
- Validation/lowering helpers are interleaved; unknown dependency direction should escalate to stricter local verifier mode.
- DRIFT-5: moving validation code must not create a third validation implementation.

## Verification Scope Recommendation

Because this is a structural split of a pure compile crate with public API stability risk, use stricter local verification even though intended semantics are unchanged:

- `cargo +nightly fmt --all --check`
- `cargo +nightly clippy -p vb_compile --all-targets --all-features -- -D warnings`
- `cargo +nightly test -p vb_compile --all-targets --all-features`
- `cargo +nightly test -p workspace_tests --test integration_compile_codegen_pipeline --test integration_compile_codegen_runtime_e2e --test integration_compile_error_message_quality --test integration_validate_yaml_parsing` or the closest supported workspace-test invocation.
- `cargo +nightly miri test -p vb_compile` if local runtime/time budget allows; master lists `vb_compile` as a pure Miri crate.
- Existing Kani harness touching `vb_compile::check_idempotency_gates`: `/home/lewis/src/go-skill-vb-m5gp/kani/idempotency_gate_parity.rs` should remain in scope if Kani lane is available.

## Blockers / Decisions for State 3

- Architecture decision required: module names requested are `mod_compile_core`, `mod_compile_errors`, `mod_compile_validation`, `mod_compile_lowering`; confirm whether these should be actual file/module names or conceptual names with idiomatic Rust module paths.
- Architecture decision required: remove/replace stale unwired `compile`, `lower`, and `validation` directories, or leave untouched for a follow-up bead.
