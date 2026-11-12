# Codebase Map: vb-qi37.7.3 — ir: Validate symbol, action, and resource references

## Relevant crates/modules/files

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/workflow/mod.rs`
  - Canonical exported compiled-IR model via `vb_core::workflow::*`.
  - `WorkflowParts` carries `nodes`, `expressions`, `accessors`, `constants`, `slot_count`, `symbols_count`, `entry`, `resource_contract`, and `step_names`.
  - `CompiledWorkflow::try_from_parts` runs `validate_parts` and `validate_budget` before accepting IR.
  - Existing symbol checks: `WorkflowError::SymbolOutOfBounds`, `validate_accessor_paths`, `validate_constants_symbols`, `validate_build_object_symbols`, `validate_symbol`.
  - Existing resource checks: `validate_resource_contract`, `validate_contract_limit`, `validate_expr_stack_contract`, `validate_budget`.
  - Gap likely relevant to this bead: `CompiledNodeKind::Do { action, input }` currently validates only `input` in `validate_node_kind`; action IDs are not validated in core because no action-contract table is part of `WorkflowParts`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/workflow/tests.rs`
  - Dense acceptance/adversarial tests for `CompiledWorkflow::try_from_parts`.
  - Existing symbol coverage around lines ~1886-2825: accessor field/index acceptance, `SymbolOutOfBounds` for constants/accessors/build-object fields, and `symbols_count` roundtrip.
  - Existing resource coverage near top and around lines ~4413+ for `ResourceContractExceeded` / `ResourceContractTooLarge`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/action.rs`
  - Defines `ActionContract`, `ActionId`, `ActionTicket`, idempotency/retry/side-effect metadata, and action error types.
  - Contract fields likely needed when validating action references: `id`, slot counts, byte bounds, timeout, idempotency, side effect, retry safety, capabilities.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/action.rs`
  - `ActionRegistry` maps `ActionId` to `ActionContract`.
  - `resolve_compile_time(ActionId)` is the existing pattern for external action contract lookup.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/gates.rs`
  - Cold-path IR verifier gates.
  - Gate 8 currently validates accessor roots and rejects `u32::MAX` index segments, but treats field symbols as “any non-sentinel value is valid”; it does not use `parts.symbols_count`.
  - Gate 9 validates slot references in nodes and expressions.
  - Gate 12 validates action contract completeness by comparing `CompiledNodeKind::Do` action IDs to provided `ActionContract` IDs.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/shared.rs`
  - Shared validation pipeline.
  - `validate(parts)` runs gates 7,8,9,10,11,13,14,15 and intentionally skips Gate 12.
  - `validate_with_contracts(parts, action_contracts)` runs non-contract gates then Gate 12.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/lib.rs`
  - `ValidationError` includes `ActionContractMissing`, `ActionContractOrphan`, resource/slot/gate errors.
  - No `SymbolOutOfBounds` style validation error is currently exposed here; symbol bounds are primarily enforced in `vb_core::workflow::WorkflowError`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/diag_codes.rs`
  - Diagnostic code constants include action contract errors under E05xx, but no explicit symbol-reference code was found.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/references.rs`
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/ref_validate.rs`
  - YAML/string-level reference validation for `$input`, `$vars`, `$secrets`, `$steps`, `$runtime`.
  - Probably adjacent only; bead title says `ir`, so compiled numeric IR is the likely focus.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/workflow_build.rs`
  - Compiler constructs `WorkflowParts`; currently sets `symbols_count: 0` and `resource_contract: ResourceContract::DEFAULT` in the inspected build path.
  - If this bead requires compiler-produced IR to satisfy symbol/resource validation, this is a suspected producer touchpoint.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/tests.rs`
  - Compiler integration tests include action compilation (`CompiledNodeKind::Do { action, input }`) and default resource contract assertions.

## Current patterns to reuse

- Prefer a pure validation function over runtime mutation:
  - `CompiledWorkflow::try_from_parts(parts)` validates untrusted IR before constructing immutable `CompiledWorkflow`.
  - `vb_validate::shared::ValidationPipeline` runs explicit gates and returns first exact `ValidationError`.
- Reference bounds use small focused helpers:
  - `validate_step`, `validate_slot`, `validate_const`, `validate_expr`, `validate_accessor` in workflow validation.
  - `validate_symbol(SymbolId, symbols_count)` in `vb_core::workflow::mod.rs`.
- Action reference validation already has a cold-path pattern:
  - `validate_gate_12_action_contract_completeness(parts, action_contracts)` collects `Do` action IDs, checks every `Do` has a contract, then rejects orphan contracts.
  - `ActionRegistry::resolve_compile_time` is the runtime/compiler lookup equivalent.
- Resource validation already separates protocol hard limits from declared-contract coverage:
  - `ResourceContractTooLarge` when declared value exceeds hard limit.
  - `ResourceContractExceeded` when actual IR usage exceeds declared value.
- Tests use exact enum matching, not string-only assertions, for core invariants.

## Suspected touchpoints for implementation

- Core IR structural validation:
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/workflow/mod.rs`
  - Ensure all `SymbolId` carriers are covered: accessor `PathSegment::Field`, `ConstValue::Symbol`, and `CompiledNodeKind::BuildObject` field keys are already covered in canonical module.
  - Determine whether `symbols_count == 0` with any symbol should continue to be rejected; existing tests imply yes.
- Cold verifier parity:
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/gates.rs`
  - Gate 8 likely drifts from core: it does not validate `PathSegment::Field(SymbolId)` against `parts.symbols_count` and does not inspect symbols in constants/build objects.
  - If bead acceptance means “IR validator validates symbol references,” add or adjust a gate rather than duplicating all logic in consumers.
- Action validation:
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/gates.rs` Gate 12 is the likely main implementation surface.
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/shared.rs` decides whether callers use `validate` or `validate_with_contracts`; contracts are external data, so rust-contract should specify when missing contract data is allowed.
  - Core `WorkflowParts` has no action-contract table; adding action checks to `vb_core::workflow::try_from_parts` would require a new API or embedding contracts, which is probably larger scope.
- Resource validation:
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/workflow/mod.rs` already enforces resource contract and whole-workflow budget.
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/gates.rs` Gate 7 covers expression stack vs `resource_contract.max_expr_stack`; other resource-contract parity may be missing from `vb_validate::shared::validate` unless callers also go through `CompiledWorkflow::try_from_parts`.
- Compiler producer:
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/workflow_build.rs` sets `symbols_count: 0` and default resources. If compiled workflows start producing symbols, this must change with tests.

## Test locations to inspect later

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/workflow/tests.rs`
  - Add/adjust tests around `SymbolOutOfBounds`, resource contract overflow/exceeded, and maybe action validation only if core API is expanded.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/gates.rs`
  - Existing Gate 12 tests near the lower half of the file cover missing/orphan action contracts.
  - Add exact tests for symbol IDs in accessor fields, `ConstValue::Symbol`, and build-object fields if validator parity is required.
  - Add tests for resource-contract parity if `vb_validate` must reject actual counts exceeding `ResourceContract` without relying on `vb_core` construction.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/shared.rs`
  - Pipeline tests should prove whether `validate` excludes Gate 12 and `validate_with_contracts` includes it.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/action.rs`
  - Registry tests validate register/resolve/duplicate behavior; useful if action-reference validation should consume registry output.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/tests.rs`
  - Compiler tests around `Do` nodes and default resource contracts are useful for end-to-end producer validation.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/vb-qi37-ws/tests/epic_coordination_test.rs`
  - Epic-level red/coordination tests mention action ABI and phase validation; inspect if this bead needs cross-bead evidence.

## Risks/dependencies

- There are parallel/split files under `crates/vb_core/src/compiled_workflow.rs`, `nodes.rs`, `accessors.rs`, and `validation/*`, but `vb_core/src/lib.rs` exports `workflow::*`. Treat `crates/vb_core/src/workflow/mod.rs` as canonical unless the build config proves otherwise.
- Action contracts are external to `WorkflowParts`; forcing action validation into `CompiledWorkflow::try_from_parts` may create an API boundary problem. Existing pattern is `vb_validate::shared::validate_with_contracts`.
- `vb_validate::shared::validate` intentionally skips Gate 12. Any caller using only `validate(parts)` will not catch missing action contracts.
- Symbol validation currently exists in core but appears incomplete in `vb_validate` Gate 8. If acceptance requires both core and verifier parity, contract should call this out explicitly.
- Resource validation may be split between core and validation gates. Avoid double-reporting with divergent error types unless the contract specifies a mapping.
- Repository rules forbid `unwrap`, `expect`, `panic`, unchecked indexing/casts/arithmetic; tests may currently contain older patterns, but new implementation should preserve zero-tolerance source lint.

## Next-state notes for rust-contract

- Define the exact boundary: “IR reference validation” should likely mean validation of numeric IDs inside `WorkflowParts` before accepted-artifact admission.
- Specify three contract groups:
  1. Symbol references: every `SymbolId` in accessors, constants, and build-object fields must be `< symbols_count`; `symbols_count == 0` rejects any symbol reference.
  2. Action references: every `Do.action` must be present in supplied `ActionContract` set/registry; decide whether orphan contracts are an error (current Gate 12 says yes).
  3. Resource references/contracts: actual nodes, slots, constants, accessors, expressions, and expression stack depth must not exceed `ResourceContract`; declared contract values must not exceed protocol hard limits.
- Decide whether the acceptance surface is `vb_core::CompiledWorkflow::try_from_parts`, `vb_validate::shared::validate_with_contracts`, or both. Current evidence suggests core already covers symbols/resources while `vb_validate` covers action contracts.
- Require exact error assertions:
  - Core: `WorkflowError::SymbolOutOfBounds`, `WorkflowError::ResourceContractExceeded`, `WorkflowError::ResourceContractTooLarge`.
  - Validator: `ValidationError::ActionContractMissing`, `ValidationError::ActionContractOrphan`, and any added symbol/resource validation errors.
- Avoid introducing runtime YAML/JSON/HTTP into the core; keep this as cold-path validation over numeric IR and external action-contract data.
