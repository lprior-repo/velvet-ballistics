# Domain Model Review: vb-engine-yaml

## Review Status

STATUS: CONTRACT_AUTHOR_REVIEW_ONLY

This is not an independent approval. The rust-contract role cannot approve its own artifacts. A separate contract-verification-reviewer must approve or reject this contract before downstream test planning or implementation consumes it.

## Aggregate Boundaries

- Authoring aggregate: `WorkflowSource`, strict YAML profile, v1 triggers, step primitive contract, source map, typed diagnostics.
- Validation aggregate: shared validation pipeline, references, type/taint checks, resource contracts, idempotency metadata, capability declarations.
- Compile aggregate: numeric IR lowering, structural validation, deterministic digest inputs, accepted artifact candidate construction.
- Artifact aggregate: accepted artifact schema, digest/proof envelope, gate evidence, capability/idempotency certificates, policy/action ABI digest binding.
- Runtime aggregate: shard-owned run frame, numeric slots/steps/actions, deterministic drive loop, bounded queues/frame pools/timers/trace rings.
- Storage aggregate: Fjall keyspaces, binary envelopes, Postcard payloads, run headers, journals, snapshots, blobs, indexes, recovery reports.
- Ingress/operator aggregate: direct API, binary IPC, CLI diagnostics, inspect/replay/incident evidence.

## Illegal States to Make Unrepresentable

- Runtime submission from YAML source text.
- Runtime-core dependencies on YAML, JSON, HTTP, or text command protocols.
- Strict admission from raw `WorkflowParts`, unchecked `CompiledWorkflow`, or dummy proof stores.
- Accepted runs without immutable source/artifact/policy/action ABI digest binding.
- Strict acknowledgement without durable source/artifact/header/RunAccepted/index evidence.
- Replay/recovery that reparses YAML for an existing run.
- Missing capability/idempotency/taint/durability/replay proof gates represented as success.
- Unbounded queue/fanout/retry/payload/journal allocation in hot paths.
- String reference resolution in runtime execution.

## Type-Driven Design Requirements

- Separate cold authoring types from accepted runtime types; no shared type should allow YAML source to masquerade as accepted artifact.
- AcceptedArtifact must encode proof-gate completeness as data required by strict admission, not optional metadata.
- ResourceContract must be present before runtime allocation or IPC payload admission.
- Digest-bound records must carry enough typed identity to prove source/artifact/header/journal/snapshot consistency.
- Operator diagnostics must render typed errors from domain errors, not reverse-parse logs or text commands.

## Model Risks

- If artifact and storage schemas drift, strict runtime may admit artifacts that cannot recover.
- If dummy proof stores remain constructible in production strict paths, accepted artifact verification becomes decorative.
- If recovery treats missing object/list/blob slot hydration as empty state, data loss becomes silent success.
- If direct API and IPC use different admission paths, one path can bypass strict accepted-artifact checks.
- If CLI `run` accepts YAML and runtime admission in one loose step, cold/hot boundaries become unverifiable.

## Required Reviewer Questions

- Can a type checker distinguish `WorkflowSource`, `CompiledWorkflow`, `AcceptedArtifact`, and `RunAcceptedEvidence` at every strict boundary?
- Are all fallible operations expressed as `Result<T, EngineYamlError>` or an equivalent typed domain error?
- Does every accepted run have a durable, immutable artifact digest and proof envelope before acknowledgement?
- Does recovery have any fallback that reconstructs from YAML or default/empty frames?
- Can dependency-boundary checks prove runtime-core crates do not depend on YAML/JSON/HTTP crates?

## Finding Summary

- APPROACH OK: the domain split matches master contract hot/cold boundaries.
- BLOCKER FOR DOWNSTREAM: exact production type names and proof targets must be confirmed during implementation/proof planning; State 3 does not authorize writing production code, tests, or proofs.
