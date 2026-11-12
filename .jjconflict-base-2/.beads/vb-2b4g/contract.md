# Contract: vb-2b4g — Repeat/Reduce/Together/Collect generated parity

## Authority

- Rust-contract startup read: `/home/lewis/.claude/skills/rust-contract/SKILL.md` lines 8, 17-18, 25: contract-first artifacts only; no implementation/tests/proof code; no invented proof targets.
- Winning startup source read: `/home/lewis/.agents/skills/rust-contract/SKILL.md` lines 8, 17-18, 25: same rules; this file wins on conflict.
- Parent contract: `.beads/vb-qi37.10/contract.md` remains binding and is narrowed here to executable support for `Repeat*`, `Reduce*`, `Together*`, and `Collect*`.

## Scope

In scope: `vb_codegen` generated Rust support for `RepeatStart/Attempt/Check/Finish`, `ReduceStart/Next/Finish`, `TogetherStart/Branch/Join`, and `CollectStart/Page/Next/Finish`, plus parity tests and static gates for generated code.

Out of scope: production/test/proof implementation by this specialist; formal TLA+/Verus/Kani claims; using `vb_core::run_until_blocked` as oracle for these families.

## Oracle

- Runtime oracle: `vb_runtime::engine::drive::drive_deterministic_full`.
- Forbidden oracle for target families: `vb_core::run_until_blocked`, because baseline exploration says it returns `UnsupportedPrimitive { primitive: "not_yet_implemented" }` for these families.
- A test that treats `not_yet_implemented` as pass is contract failure.

## Preconditions

- PRE-001: Workflows are validated `CompiledWorkflow` instances with identical workflow digest, constants, slots, inputs, limits, deterministic seeds, and action IDs for generated and runtime oracle execution.
- PRE-002: Generated-family admission is decided before source emission by the active `crates/vb_codegen/src/lib.rs` path, not by duplicate or inactive code.
- PRE-003: Resource limits for attempts, reducer inputs, together branch count, collect pages/items, side stores, and journal observations are finite and explicit.
- PRE-004: Parity harness invokes `vb_runtime::drive_deterministic_full` for target families and fails if the compared oracle result is `not_yet_implemented`.

## Postconditions

- POST-001 Repeat parity: generated `Repeat*` execution matches runtime oracle on terminal result or typed error variant/fields, pc, slots, taints, step states, attempt counters, and normalized journal signature.
- POST-002 Reduce parity: generated `Reduce*` execution matches runtime oracle on empty/single/multi input, accumulator initialization, item binding, iteration state, materialized output, typed errors, pc, slots, taints, step states, and journal signature.
- POST-003 Together parity: generated `Together*` execution matches runtime oracle on deterministic branch routing, branch result order, aggregation/join state, failure policy, typed errors, pc, slots, taints, step states, and journal signature.
- POST-004 Collect parity: generated `Collect*` execution matches runtime oracle on single-page, multi-page, duplicate page, stale page, page materialization order, page state, capacity exhaustion, typed errors, pc, slots, taints, step states, and journal signature.
- POST-005 No fail-open support: if any target family remains unsupported, validation must fail closed with exact typed unsupported family; bead acceptance cannot claim that family complete.
- POST-006 Static generated-source contract: emitted target-family source contains no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, runtime YAML/JSON/HTTP, or string action/reference lookup.

## Invariants

- INV-001: Generated and runtime executions compare the same observable dimensions: terminal result/error, pc, slots, taints, step states, family counters/state, collect page state, and normalized journal signature.
- INV-002: Taint is preserved exactly; generated code must not drop, weaken, or invent taint through repeat finish, reducer accumulator/output, together join, collect materialization, or final result.
- INV-003: Counters and capacities use checked arithmetic and typed errors; wrapping/saturating behavior is allowed only if it matches runtime oracle exactly.
- INV-004: Collect page handling rejects duplicate, stale, and out-of-order pages exactly like runtime and preserves multi-page materialization order.
- INV-005: Journal parity is normalized semantic parity only: event kind/order, step id, slot id, value kind, taint, target-family state/counter essentials, and terminal event.

## Error taxonomy

- Unsupported target family: exact typed generated unsupported error naming `Repeat*`, `Reduce*`, `Together*`, or `Collect*` before emission.
- Runtime-compatible typed errors: generated and oracle must expose the same variant and fields for type mismatch, capacity exceeded, stale/duplicate collect page, attempt/counter overflow, index/bounds, and semantic mismatch.
- Journal mismatch: reports exact mismatching dimension, expected signature, and actual signature.
- Semantic mismatch: reports exact dimension among result, error, pc, slots, taints, step state, attempts/counters, collect page state/materialization, or journal.

## Acceptance commands

Required focused gates:

```bash
cargo test -p vb_codegen repeat_generated_parity -- --nocapture
cargo test -p vb_codegen reduce_generated_parity -- --nocapture
cargo test -p vb_codegen together_generated_parity -- --nocapture
cargo test -p vb_codegen collect_generated_parity -- --nocapture
cargo test -p vb_codegen generated_source_contract -- --nocapture
cargo test -p vb_codegen --test trybuild_tests
cargo check -p vb_codegen --all-targets
cargo fmt --all -- --check
```

Final integration gate remains `moon ci` if this bead reaches landing.
