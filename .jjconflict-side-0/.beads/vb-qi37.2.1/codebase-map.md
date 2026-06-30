# Codebase Map: vb-qi37.2.1

Bead: `vb-qi37.2.1`
Title: `runtime: Define aggregate resource budget model`
State: 2 artifact retry

## Relevant Files

- `crates/vb_core/src/budget.rs`: Existing core resource-budget value types and budget arithmetic are likely here. Treat this as the primary domain-language source for budget limits, resource dimensions, validation helpers, and any checked math patterns.
- `crates/vb_core/src/workflow/mod.rs`: Workflow model entrypoint. Inspect for workflow-level metadata or resource declarations that an aggregate resource budget model must consume.
- `crates/vb_core/src/validation/resource.rs`: Existing validation layer for resource-related constraints. Likely home for structural checks that reject invalid per-node or per-workflow resource budgets.
- `crates/vb_core/src/engine/tests/integration_budget.rs`: Existing behavioral coverage for budget enforcement. Reuse scenario style and fixtures for aggregate budget contract tests.
- `crates/vb_runtime/src/shard/types.rs`: Runtime shard resource/accounting types. Suspected touchpoint for shard-local budget totals and admission decisions.
- `crates/vb_runtime/src/runtime.rs`: Runtime orchestration entrypoint. Suspected touchpoint for propagating aggregate limits into admission, scheduling, or execution contexts.
- `crates/vb_runtime/src/admission.rs`: Admission-control logic. Primary runtime enforcement touchpoint for rejecting workloads that exceed aggregate resource budget.
- `velvet-ballistics-MASTER.md`: Authoritative architecture and acceptance contract. Use it to confirm naming, phase scope, constraints, and whether aggregate budget behavior belongs in core model, runtime admission, or both.

## Patterns To Reuse

- Keep budget semantics in `vb_core` as pure domain/data logic; keep runtime enforcement in `vb_runtime` admission/orchestration.
- Prefer checked arithmetic and explicit fallible constructors over implicit primitive math. Repository rules prohibit unchecked arithmetic, casts, indexing, and panic paths.
- Reuse existing resource validation error style from `crates/vb_core/src/validation/resource.rs` instead of introducing a parallel error vocabulary.
- Reuse existing budget test fixture style from `crates/vb_core/src/engine/tests/integration_budget.rs` for Given/When/Then-style aggregate scenarios.
- Keep generated/maxperf runtime constraints in mind: no YAML, JSON, or HTTP dependencies in runtime core; aggregate budget model should remain typed Rust data, not dynamic config parsing.
- Preserve crate naming conventions from `AGENTS.md`: package/product `velvet-ballistics`, crate/module `velvet_ballistics`; do not introduce `velvet-ballistics` names except where already externally fixed.

## Suspected Touchpoints

- `vb_core::budget`: Define or extend an aggregate resource budget type that can represent the sum/limit across workflow resources without runtime-specific dependencies.
- `vb_core::workflow`: Attach or expose aggregate budget requirements from workflow definitions if not already present.
- `vb_core::validation::resource`: Validate aggregate budget invariants, including non-zero/finite limits, no overflow when combining resources, and consistency between per-resource and aggregate constraints.
- `vb_runtime::admission`: Compare requested aggregate budget against runtime/shard capacity before execution starts.
- `vb_runtime::shard::types`: Represent available capacity, reserved capacity, or per-shard budget snapshots in terms compatible with the core aggregate model.
- `vb_runtime::runtime`: Wire validated aggregate budget data into admission without duplicating validation logic.

## Test Locations

- Add/extend core integration coverage in `crates/vb_core/src/engine/tests/integration_budget.rs` for aggregate construction, validation success, validation failure, and checked-combine behavior.
- Add runtime admission tests near `crates/vb_runtime/src/admission.rs` if existing inline/module tests are present, or in the runtime test directory if the crate already uses external integration tests.
- Cover edge cases: aggregate exactly equals capacity, aggregate exceeds capacity by one unit, multiple resource dimensions combine safely, overflow/invalid totals fail deterministically, and missing aggregate data follows the intended default from the contract.
- Use `moon ci` as the final canonical gate after implementation, with targeted Cargo tests only as faster local feedback.

## Risks And Dependencies

- The domain/runtime boundary can drift if runtime defines separate aggregate budget types. Prefer one core model plus runtime adapters/conversions where necessary.
- Overflow risk is central: aggregate budget summation must not use unchecked `+`, casts, indexing, or assumptions about resource vector shape.
- Admission behavior may depend on shard capacity definitions in `crates/vb_runtime/src/shard/types.rs`; contract should specify whether aggregate budget is global, per-shard, or both.
- Existing budget tests may validate per-task budgets only. Aggregate semantics need explicit scenarios to avoid silently accepting overcommitted workflows.
- The master document is authoritative; if inspected code and docs disagree, rust-contract should record the discrepancy before implementation.
- Repository rules ban `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, and `unsafe`; contract and later implementation should include these as acceptance constraints.

## Next-State Notes For rust-contract

- Define the aggregate resource budget contract before implementation: inputs, invariants, error cases, and where enforcement occurs.
- Clarify whether aggregate budget is computed from child budgets, declared explicitly, or both with parity validation.
- Specify capacity comparison semantics: `requested <= available` should admit, `requested > available` should reject with a typed error.
- Specify dimensionality rules: missing resource dimensions, duplicate dimensions, zero limits, and unknown dimensions need deterministic behavior.
- Specify overflow behavior: any aggregate sum overflow must return a validation/admission error, never wrap or saturate silently unless the master contract explicitly requires saturation.
- Include BDD scenarios for validation and runtime admission so later State 3/implementation can prove behavior end-to-end.
