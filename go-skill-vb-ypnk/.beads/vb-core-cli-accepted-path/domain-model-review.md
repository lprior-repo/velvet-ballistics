# Domain Model Review: vb-core-cli-accepted-path

## Verdict

STATUS: CONTRACTABLE_WITH_BLOCKING_DEPENDENCIES

The domain model must make illegal strict admission states unrepresentable: a strict run cannot be constructed from YAML, `WorkflowParts`, or `CompiledWorkflow` alone. The only strict admission witness is a storage-backed accepted artifact receipt whose digest is bound into the run header and journal event.

## Model Strengths From State 2

- The repository already distinguishes YAML authoring, compiled IR, storage records, runtime policy, and accepted artifact stores.
- Runtime admission already rejects missing/malformed/invalid accepted artifacts when constructed with a real `AcceptedArtifactStore`.
- Runtime lifecycle admission failure is already before run state insertion according to State 2 mapping.

## Model Gaps To Close

- Current CLI persistence stores raw `CompiledIrRecord` instead of an `AcceptedArtifact` envelope for strict YAML `run`.
- Current `cmd_submit` can write source/header/`RunAccepted` without accepted artifact persistence.
- Current multi-shard runtime construction through `Runtime::new_with_journal` uses `AlwaysPresentArtifactStore`, making strict admission appear to pass without storage evidence.
- Current accepted artifact gate-count semantics are inconsistent between storage and runtime.

## Required Type Separations

- `YamlSource` is an authoring input, not runtime evidence.
- `CompiledWorkflow` is a compile output, not strict admission evidence.
- `WorkflowParts` is serialized IR, not strict admission evidence.
- `AcceptedArtifact` is the storage envelope carrying verification evidence.
- `AcceptedRunReceipt` is the durable acknowledgement after the accepted-run boundary.
- `StrictRuntime` must require a storage-backed artifact loader; relaxed runtime may use dummy loaders only outside strict/journaled policy.

## Illegal States

- Strict run with journal but no storage-backed artifact store.
- Strict run with `RunAccepted` before accepted artifact persistence.
- Run header digest different from accepted artifact digest.
- Stored compiled IR containing raw `WorkflowParts` where strict admission expects `AcceptedArtifact`.
- Direct compiled strict execution without accepted artifact receipt.

## Dependency Review

- `vb-core-accepted-artifact-format`: must settle schema, verification gate count, and malformed/legacy behavior.
- `vb-core-atomic-admission`: must supply or validate an atomic accepted-run durability boundary.
- `vb-core-storage-artifact-store`: must ensure production strict runtime construction cannot use `AlwaysPresentArtifactStore`.

## Handoff Constraints

- Implementation agents must preserve canonical product spelling `velvet-ballastics` in new user-facing text.
- No runtime YAML/JSON/HTTP interpretation may be introduced.
- Any new constructor or wrapper must encode policy/store constraints rather than relying on comments.
