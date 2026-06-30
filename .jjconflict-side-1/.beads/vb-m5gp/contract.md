# Contract Specification: vb-m5gp

## Context

- Bead: `vb-m5gp` - split `crates/vb_compile/src/lib.rs` into private modules.
- Workspace: `/home/lewis/src/go-skill-vb-m5gp` only.
- Source scope: `vb_compile` crate structural refactor.
- Doctrine read: `/home/lewis/.claude/skills/rust-contract/SKILL.md` and `/home/lewis/.agents/skills/rust-contract/SKILL.md`; both are version `2.6.0`, and the agents copy is authoritative if conflict appears.
- DDD decision retained: `domain-model-review.md` approves actual private files `mod_compile_core.rs`, `mod_compile_errors.rs`, `mod_compile_validation.rs`, and `mod_compile_lowering.rs` with stable crate-root API.

## Assumptions

- This bead is a pure refactor: no behavior, dependency, feature, public API, or runtime semantic change is allowed.
- Existing unwired `compile/`, `lower/`, and `validation/` scaffolding is stale until parity is proven; this bead must not wire it blindly.
- Existing public behavior is defined by baseline `moon ci` success, current public API use sites, existing tests, fuzz/Kani harness paths, and `codebase-map.md`.

## Preconditions

- PRE-001: Implementation starts from isolated workspace `/home/lewis/src/go-skill-vb-m5gp`, not the forbidden source checkout.
- PRE-002: The only production crate in scope is `vb_compile`; no dependency, feature, or config changes are in scope.
- PRE-003: The active implementation is moved from `crates/vb_compile/src/lib.rs`; unwired `compile/`, `lower/`, and `validation/` scaffolding may not be reused unless exact parity is independently proven.
- PRE-004: Public crate-root APIs listed in `codebase-map.md` are the compatibility surface to preserve.

## Postconditions

- POST-001: `lib.rs` is a thin facade declaring private modules `mod_compile_core`, `mod_compile_errors`, `mod_compile_validation`, and `mod_compile_lowering`.
- POST-002: Existing crate-root public names remain reachable with unchanged signatures, visibility, cfg gates, error variants, and re-exports.
- POST-003: Accepted inputs produce unchanged compile results: IR, artifacts, compiled digest, generated Rust, and idempotency gate outcomes remain equivalent to baseline.
- POST-004: Rejected inputs produce unchanged `CompileError` variants, stable diagnostic codes, and observable messages unless an existing test already permits formatting variation.
- POST-005: No new public module paths are promised for `compile`, `lower`, `validation`, or the new private modules.
- POST-006: File-length governance improves: `lib.rs` drops below the repository threshold; every new module is below threshold or has bead-linked follow-up justification.

## Invariants

- INV-001: Public API compatibility is stable at crate root for all public types/functions/modules/re-exports enumerated in `codebase-map.md`.
- INV-002: Module dependency direction remains acyclic: errors are leaf diagnostics; validation and lowering may depend on errors; core composes validation/lowering/errors; facade re-exports only.
- INV-003: Validation remains validation: source/document/profile/name/shape checks do not fork into a third implementation and do not move semantic lowering into validation.
- INV-004: Lowering remains deterministic and behavior-preserving for canonical and legacy public lowering functions.
- INV-005: Diagnostic taxonomy remains typed: expected failures are `CompileError`/`CompileErrors`, not strings or panics.
- INV-006: No new `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked casts, or unchecked arithmetic is introduced by the split.
- INV-007: Tests moved by concern may gain only the minimum visibility required; production helpers must not be made broadly public to compensate for test location.

## Error Taxonomy

- ERR-001: Existing `CompileError` variants and stable diagnostic code mapping remain unchanged.
- ERR-002: Source provenance via `SourceMark` remains available wherever existing diagnostics carried it.
- ERR-003: Multi-error collection behavior through `CompileErrors` remains unchanged.
- ERR-004: Any scaffolding/parity uncertainty is a follow-up or proof blocker, not a behavior rewrite hidden in this bead.

## Contract Signatures

The split must preserve the existing signatures at crate root, including but not limited to:

- `YamlLimits`, `YamlCompiler`, `SourceMark`, `CompileError`, `CompileErrors`.
- `compile_workflow`, `compile_source`, `compile_workflow_with_contracts`, `compile_to_generated_rust`.
- `build_slot_layout`, `build_accessor_table`, `build_constant_pool`, `validate_ir`, `compute_compiled_digest`, `emit_compiled_artifact`.
- `lower_steps_to_ir`, all public `lower_*` functions, `WaitKind`, `SlotCompiler`.
- `is_compile_idempotency_gate_accepted`, `check_idempotency_gates`.
- Existing public modules/re-exports: `ast`, `expression`, `strict_yaml`, expression bytecode re-exports, `vb_validate::{ValidationError, ValidationResult}`, and `#[cfg(kani)]` surface.

## Module Ownership Contract

- `mod_compile_core`: orchestration, facade implementation, table builders, artifact/digest/codegen helpers, idempotency helpers.
- `mod_compile_errors`: `SourceMark`, `CompileError`, `CompileErrors`, diagnostic code mapping, `Display`/`Error` implementations.
- `mod_compile_validation`: UTF-8/source/document/profile/name/shape validation and field extraction helpers that decide legality.
- `mod_compile_lowering`: canonical layout, primitive lowering, `WaitKind`, `SlotCompiler`, shared lowering helpers, legacy dead-code lowering only if existing compatibility requires it.

## Verification Ownership

- TLA+: not applicable; this is not workflow/protocol/concurrent/lifecycle behavior. See `tla-spec.md`.
- Verus: waived for this bead because no new pure algorithm or data invariant is being designed; parity is better proven by compile, Kani, Miri, API, static scan, and source-structure checks. Any later semantic extraction must create Verus obligations.
- Lean/Aeneas/Hax: not applicable; no tiny theorem kernel is introduced. See `lean-contract.md`.

## Non-goals

- Do not redesign validation, lowering, diagnostics, idempotency, codegen, or public module paths.
- Do not add dependencies, features, config, benchmarks, generated Rust behavior, or new public compatibility shims.
- Do not wire stale scaffolding without separate parity proof.
