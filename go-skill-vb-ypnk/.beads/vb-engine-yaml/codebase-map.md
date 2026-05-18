# vb-engine-yaml Codebase Map

State 2 canonical artifact repair for bead `vb-engine-yaml` in isolated workspace `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`.

## Bead Source

- Command: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-engine-yaml --json`
- Title: `engine: Durable YAML runtime acceptance without UI or generated Rust`
- Status: `in_progress`
- Type: `feature`
- Labels: `admission`, `boundedness`, `cli`, `durability`, `engine`, `no-codegen`, `no-ui`, `quality`, `recovery`, `release-plan`, `runtime`, `yaml`

## Delivery Scope

The bead is the engine-only acceptance root for a durable workflow engine from strict YAML authoring through compiled numeric IR, accepted artifact admission, bounded deterministic runtime execution, Fjall/Postcard durability, replay/recovery, direct API, IPC, CLI operator evidence, and engine-scoped quality evidence.

Explicit exclusions: UI delivery, generated Rust/codegen parity, and maxperf generated-mode completion are out of scope for this bead.

## Relevant Crate Areas

- `crates/vb_yaml/**`: strict cold-path YAML authoring parser, AST, source maps, trigger/step/value shape checks.
- `crates/vb_validate/**`: shared validation pipeline, diagnostics, gate inputs, references, type/taint/idempotency/capability checks.
- `crates/vb_compile/**`: YAML AST handoff, strict compiler path, numeric IR lowering, validation integration, artifact digest inputs.
- `crates/vb_core/**`: numeric identifiers, workflow parts, values, artifacts, contracts, budgets, runtime-independent model types.
- `crates/vb_runtime/**`: deterministic runtime, admission, bounded primitives, lifecycle, events, recovery, idempotency and capability enforcement seams.
- `crates/vb_storage/**`: Fjall/Postcard persistence, accepted artifacts, run headers, journal, snapshots, indexes, hydration/recovery, fail-closed errors.
- `crates/vb_ipc/**`: local binary IPC frames, bounded ingress, commands, payloads, backpressure, server/client dispatch.
- `crates/velvet_ballastics/**`: CLI/operator entrypoints, diagnostics, YAML validation/compile/admission/runtime/storage integration.
- `fuzz/**`, `kani/**`, `verification/**`, `tests/**`, `xtask/**`, `.moon/**`: evidence and gate surfaces for fuzz, formal/model checks, integration tests, release evidence, and canonical `moon ci` orchestration.

## Public API Seams

- `vb_yaml::parse_workflow_source`
- `vb_yaml::ast::WorkflowSource`
- `vb_compile::YamlCompiler`
- `vb_compile::compile_source`
- `vb_validate::shared::validate`
- `vb_validate::shared::validate_with_contracts`
- `vb_validate::shared::ValidationPipeline`
- `vb_runtime::admission`
- `vb_storage::artifacts`
- `vb_storage::admission`
- `vb_storage::headers`
- `vb_storage::journal`
- `vb_storage::recovery`
- `vb_ipc::frame`
- `vb_ipc::payloads`
- `vb_ipc::commands`
- `vb_ipc::ingress`

## Boundary Rules

- Runtime-core crates `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc` must not parse YAML, JSON, or HTTP.
- YAML stays in cold authoring, validation, and compile surfaces.
- Strict runtime admission must bind accepted artifacts and digests, not loose YAML or raw unchecked IR.
- Fjall and Postcard are required persistence mechanisms for the durable engine path.
- Recovery and replay must not reparse YAML and must fail closed on corruption, mismatch, or incomplete durable evidence.

## Risk Tags

- `runtime-yaml-leak`
- `raw-ir-bypass`
- `durability-before-ack`
- `recovery-reparse`
- `unbounded-runtime`
- `dummy-proof`
- `operator-diagnostic-drift`
- `dependency-boundary-drift`

## Required Verifier Modes

- `moon ci` as the canonical workspace gate.
- Focused `cargo fmt --check`, `cargo check`, `cargo test` or nextest, and strict clippy for touched crates where required by the repository.
- Miri, fuzz smoke, Kani/Verus/TLA/Loom/property checks where the dependency closure requires proof for admission, boundedness, capability/idempotency, recovery, or concurrency ordering.
- Coverage, mutation smoke, dependency-boundary, banned-token, unsafe, supply-chain, and performance gates only where engine-scope acceptance requires them.

## State 2 Repair Evidence

- Canonical artifact path repaired: `.beads/vb-engine-yaml/codebase-map.md`.
- Machine-readable scope guard repaired: `.beads/vb-engine-yaml/delivery-scope.jsonl`.
- State log appended: `.beads/vb-engine-yaml/STATE.md`.
- No production code, tests, or proofs were modified by this repair.
