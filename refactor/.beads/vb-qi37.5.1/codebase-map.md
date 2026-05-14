# Codebase Map: vb-qi37.5.1 — verifier: Define idempotency contract model

## Relevant crates/modules/files

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/action.rs`
  - Owns the core idempotency model today: `Idempotency`, `SideEffect`, `RetrySafety`, `IdempotencyViolation`, `ActionContract`, `ActionTicket`, `verify_idempotency`, `validate_idempotency_key_ingredients`, `issue_action_ticket`.
  - Current runtime contract rules: `SideEffect::None` always passes; `RetrySafety::Safe` passes; `RetrySafety::KeyRequired` requires non-empty clean key slots; `RetrySafety::Unsafe` fails.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/slot.rs`
  - Contains `check_idempotency_gates`, a compile-time static action-contract gate.
  - Rules currently reject side-effecting `RetrySafety::Unsafe` and side-effecting `Idempotency::AtLeastOnceExternal`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/api_validation.rs`
  - Duplicates the public `check_idempotency_gates` implementation/pattern.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/compile.rs`
  - `compile_workflow_with_contracts` calls `validate_with_contracts` then `check_idempotency_gates`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/lib.rs`
  - Also exposes `compile_workflow_with_contracts` and `check_idempotency_gates`; likely generated/aggregated module surface mirrors split files.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/shared.rs`
  - Validation pipeline entry point. Gate 12 checks action-contract completeness; no dedicated idempotency contract gate here yet.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/gate_12_14_15.rs`
  - Gate 12 implementation validates every Do action has matching `ActionContract` and no orphan contracts.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/lib.rs`
  - `ValidationError` currently has `ActionContractMissing`, `ActionContractOrphan`, determinism/type/slot errors; no verifier-specific idempotency contract error variant.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/commands_verify.rs`
  - CLI `verify` pipeline uses `vb_validate::shared::validate(&parts)` only, so contract-aware Gate 12/idempotency checks are not included by default.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs`
  - CLI action registry/spec display uses three sample `CliActionSpec`s and string renderers for idempotency/retry/side-effect names and idempotency rules.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_ipc/src/server/handlers.rs`
  - Certificate/gate handler manually invokes gates and currently calls Gate 12 with `&[]`, which will fail for workflows with Do nodes unless contract data is wired in.

## Current patterns to reuse

- Keep the canonical domain types in `vb_core::action`; downstream crates import those instead of defining parallel enums.
- Gate-style validation returns typed errors, short-circuits in `vb_validate::shared`, and uses explicit `ValidationError` variants for certificate/diagnostic rendering.
- Compile-time multi-error accumulation is already used by `check_idempotency_gates`: collect `CompileError::IdempotencyViolation`, return `CompileErrors` only if non-empty.
- Tests use local fixture constructors such as `make_contract(...)`, `make_parts(...)`, `do_node(...)`, and explicit match assertions on typed error variants.
- Repository style forbids `unsafe`, unchecked indexing, `unwrap`/`expect` in production; production loops prefer checked increments and `get`/`Option` handling.

## Suspected touchpoints for next states

- Decide whether the new verifier idempotency contract model belongs in:
  - `vb_core::action` as reusable domain model, or
  - `vb_validate` as a verifier-only contract checker over `ActionContract` + `WorkflowParts`.
- If this is a verifier gate, likely add a new `ValidationError` variant and a new gate/export in `vb_validate::shared` rather than only relying on `vb_compile::check_idempotency_gates`.
- `commands_verify::run_verification` may need a contract-aware path if `verify` must prove idempotency contracts. Today it has no registered `ActionContract` input.
- IPC verification certificate code in `vb_ipc/src/server/handlers.rs` likely needs real contract data, not `&[]`, before a contract model can be meaningful in UI certificates.
- CLI action contract presentation in `velvet_ballastics/src/main.rs` may need to display any new model fields/rules if public UX changes.
- Avoid diverging duplicate logic between `vb_compile/src/slot.rs`, `vb_compile/src/api_validation.rs`, and `vb_compile/src/lib.rs`.

## Test locations to inspect later

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/action.rs` unit tests around lines 948+ cover idempotency violations, `verify_idempotency`, key taint rejection, and ticket key preservation.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/tests/test_20.rs` covers boundary behavior for `check_idempotency_gates`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/tests/test_21.rs` covers additional idempotency gate cases and multiple violation accumulation.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/gate_12_14_15.rs` unit tests cover action contract completeness and are the closest verifier-gate pattern.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/tests/cli_integration.rs` covers CLI action list/inspect JSON/text fields and taint propagation helpers.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/tests/cross_crate_adversarial.rs` has cross-crate action/idempotency/taint assertions.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/fuzz/src/bin/verifier_gates.rs` may need later fuzz coverage if a verifier gate is added.

## Risks/dependencies

- Contract source ambiguity: normal `verify <workflow.yaml>` currently compiles YAML and validates IR only; it does not know registered external action contracts.
- Duplicated idempotency gate implementations in `vb_compile` files raise drift risk; rust-contract should specify a single source of truth.
- Existing `ActionTicket.idempotency_key` allows `0`; model must distinguish “no key supplied” from a valid numeric key if runtime dispatch checks are expanded.
- `IdempotencyViolation::RandomInKey` and `TimeInKey` exist but comments say the metadata is not modeled in `SlotValue`; contract must not require impossible checks without adding metadata.
- Gate 12 currently rejects orphan contracts. If verifier receives a global registry rather than workflow-specific contracts, this may conflict with practical usage.
- IPC certificate code currently passes empty contracts to Gate 12; adding stricter idempotency verifier checks could make certificate generation fail until contract registry plumbing exists.

## Next-state notes for `rust-contract`

- Define the contract boundary first: static compile/deploy contract over `ActionContract`, runtime dispatch contract over key slots/frame taint, or verifier certificate contract over `WorkflowParts + ActionContract`.
- Specify invariants for each combination of `SideEffect`, `RetrySafety`, and `Idempotency`; include whether `DeterministicPure + side_effect != None` is legal or rejected.
- Specify how key presence is represented: `key_slots` non-empty, `ActionTicket.idempotency_key != 0`, or a new typed `Option`/newtype model.
- Specify whether verifier accumulates all idempotency contract violations or short-circuits like `ValidationPipeline`.
- Specify exact error surface and stable diagnostic text before implementation because CLI, IPC certificate, and tests may assert messages.
- Preserve current clean-key taint invariant: `Secret` and `DerivedFromSecret` cannot participate in idempotency key ingredients.
