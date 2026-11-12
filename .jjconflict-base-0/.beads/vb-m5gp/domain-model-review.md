# Domain Model Review: `vb-m5gp`

## Scope

- Bead: `vb-m5gp` — Split `vb_compile/src/lib.rs`.
- Lane: State 3 DDD/type-model architectural review only.
- Workspace: `/home/lewis/src/go-skill-vb-m5gp`.
- Production code/test/proof/config edits: none.

## Doctrine Read

- Read `/home/lewis/.claude/skills/scott-ddd-refactor/SKILL.md`.
- Read `/home/lewis/.agents/skills/scott-ddd-refactor/SKILL.md`.
- They match; no conflict. `/home/lewis/.agents/skills/scott-ddd-refactor/SKILL.md` is authoritative if future conflict appears.
- Applied cited doctrine: make invalid states unrepresentable, parse/constrain at boundaries, use types as executable specification, keep workflows explicit, keep expected failures as enumerable domain errors, avoid validation sprinkling and god modules.

## Architectural Decision

### Decision A: Requested names are actual Rust private module filenames

Use the requested names as actual private Rust module files under `crates/vb_compile/src/`:

- `mod_compile_core.rs`
- `mod_compile_errors.rs`
- `mod_compile_validation.rs`
- `mod_compile_lowering.rs`

`lib.rs` should become a thin stable facade declaring these modules privately and re-exporting the existing crate-root public API. Do **not** create public modules named `compile`, `lower`, or `validation` as part of this bead. Downstream evidence says callers depend on crate-root APIs, not internal paths.

Rationale: this bead is a pure structural refactor. Private modules keep the API stable while eliminating the god-module smell. Public module paths would be a new API design decision and create compatibility obligations not requested by the bead.

### Decision B: Existing unwired scaffolding must not be reused blindly

Existing directories/files:

- `crates/vb_compile/src/compile/mod.rs`
- `crates/vb_compile/src/lower/mod.rs`
- `crates/vb_compile/src/validation/mod.rs`

are currently unwired scaffolding. They should **not** be wired into active compilation for this bead unless implementation proves exact semantic parity with the active `lib.rs` code.

Recommended implementation policy:

1. Move the active implementation from `lib.rs` into the four new private modules.
2. Leave unwired scaffolding untouched during the first split if deletion would enlarge risk.
3. File a follow-up bead to delete or convert the stale scaffolding after the split passes gates.

Rationale: the codebase map reports those scaffolds are stale/unwired and may lack newer canonical primitive lowering. Reusing them would turn a pure refactor into a semantic rewrite.

## Target Module Boundaries

### `mod_compile_core`

Owns orchestration and stable facade implementation:

- `YamlLimits`
- `YamlCompiler`
- `compile_workflow`
- `compile_source`
- `compile_workflow_with_contracts`
- `build_slot_layout`
- `build_accessor_table`
- `build_constant_pool`
- `validate_ir`
- `compute_compiled_digest`
- `emit_compiled_artifact`
- `compile_to_generated_rust`
- idempotency gate helpers

DDD role: explicit compile workflow coordinator. It should compose parsed/validated source, lowering, artifact emission, and diagnostics without owning validation rules or primitive lowering internals.

### `mod_compile_errors`

Owns enumerable diagnostic model:

- `SourceMark`
- `CompileError`
- `CompileErrors`
- stable diagnostic code mapping
- `Display`/`Error` implementations

DDD role: error taxonomy. Expected compile failures must remain typed variants, not strings. `SourceMark` belongs here because it is diagnostic provenance, even if used by validation/lowering.

### `mod_compile_validation`

Owns source/document/shape validation:

- UTF-8/source/document limit checks
- duplicate-key scanning
- strict profile tree validation
- scalar/container limits
- workflow and trigger shape validation
- public-name validation
- primitive field extraction/validation helpers, only where those helpers decide source shape legality

DDD role: parse-don't-validate boundary. It constrains raw YAML/source shape before core and lowering consume it. It should not emit IR or artifacts.

### `mod_compile_lowering`

Owns AST-to-IR lowering:

- canonical AST layout and width calculation
- step-name expansion
- `lower_canonical_*` functions
- public `lower_*` primitive functions
- `WaitKind`
- `SlotCompiler`
- shared lowering helpers
- legacy dead-code lowering only if still needed for existing tests/compatibility

DDD role: deterministic domain transformation from validated compile input to IR. It should accept already-constrained structures where practical and return typed `CompileError` failures for expected impossible/invalid transitions that cannot yet be encoded by types.

## Public API Stability Contract

The later implementation must preserve crate-root access to all existing public APIs listed in `codebase-map.md`, including:

- core types/functions: `YamlLimits`, `YamlCompiler`, `compile_workflow`, `compile_source`, `compile_workflow_with_contracts`
- diagnostics: `SourceMark`, `CompileError`, `CompileErrors`
- lowering: `lower_steps_to_ir`, all public `lower_*`, `WaitKind`, `SlotCompiler`
- artifact helpers: table builders, `validate_ir`, digest/artifact/codegen helpers
- idempotency helpers
- existing public modules/re-exports: `ast`, `expression`, `strict_yaml`, expression bytecode re-exports, `vb_validate::{ValidationError, ValidationResult}`, Kani cfg module

No new public internal module paths should be promised in this bead.

## Type-Driven Invariants to Preserve

### Pure refactor invariants

1. For every accepted input, emitted IR/artifacts/digests/codegen output remain byte-for-byte or semantically identical to baseline, depending on existing tests/contracts.
2. For every rejected input, `CompileError` variant and stable diagnostic code remain unchanged unless a later contract explicitly approves a diagnostic migration.
3. Crate-root public names and visibility remain stable.
4. Existing `#[cfg(kani)]` idempotency parity surface remains reachable.
5. No third validation implementation is created; validation moves, it does not fork.

### Domain invariants

1. Raw YAML/source strings are accepted only at boundary/facade functions; deeper core/lowering should prefer parsed AST/validated structures.
2. YAML limits remain constrained through `YamlLimits`; default limit constants remain authoritative.
3. Source provenance is carried with `SourceMark`/diagnostic context where currently available.
4. Workflow version acceptance remains explicit and unchanged.
5. Step names and public names remain validated before being used as IR identifiers or generated symbols.
6. Duplicate keys, strict profile violations, scalar/container limit violations, and document-shape violations remain validation failures, not lowering failures.
7. Lowering never constructs invalid branch/slot/action layouts silently; expected failures return `CompileError`.
8. Idempotency gate states remain explicit through accepted/rejected outcomes; do not encode gate lifecycle as booleans hidden in core control flow.

## Type-Model Opportunities for Later Implementation

Do not expand scope beyond the split, but prefer these type moves where they reduce visibility leakage without changing behavior:

- Keep `YamlLimits` as the constrained value object for parser/compiler limits.
- Treat `WaitKind` as the explicit mode enum instead of any boolean branch flag.
- Preserve `CompileError` as the closed/non-exhaustive domain failure taxonomy.
- If helper visibility must increase for cross-module calls, prefer narrow `pub(crate)` functions named by domain operation over broad re-export of implementation structs.
- Avoid `Option` fields as lifecycle state if new structs are introduced during extraction; use enums carrying only valid data.

## Dependency Direction

Preferred acyclic direction:

```text
lib.rs facade
  -> mod_compile_core
       -> mod_compile_validation
       -> mod_compile_lowering
       -> mod_compile_errors
  -> public re-exports

mod_compile_validation -> mod_compile_errors
mod_compile_lowering   -> mod_compile_errors
mod_compile_lowering   -> validated AST/domain inputs where available
```

Avoid validation depending on lowering. Avoid errors depending on core/validation/lowering.

## Test/Proof Implications for Later Lanes

Contract/proof/test/implementation should verify:

- crate-root public API still compiles for CLI, workspace tests, fuzz target, benches, and Kani harness
- diagnostic variants/codes/messages expected by tests remain stable
- canonical lowering outputs and compiled digest remain stable
- idempotency gate parity harness still targets the same behavior
- no source-length/god-module regression remains after split

## Follow-Up Beads Recommended

1. Remove or convert stale unwired `compile`, `lower`, and `validation` scaffolding after this split passes gates.
2. If product wants idiomatic public modules later, create a separate API design bead for `vb_compile::{compile, lower, validation}` compatibility surfaces.

## Review Status

APPROVED ARCHITECTURAL DIRECTION for a pure private-module split with stable crate-root API.

Artifact: `/home/lewis/src/go-skill-vb-m5gp/.beads/vb-m5gp/domain-model-review.md`
