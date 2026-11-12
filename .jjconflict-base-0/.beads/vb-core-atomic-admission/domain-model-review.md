# Domain Model Review: vb-core-atomic-admission

## Verdict

State 2 maps a real domain gap: strict accepted-run admission currently spans separate storage operations and mixed artifact representations. The model must introduce an explicit accepted-run aggregate boundary instead of treating source, compiled IR, run header, acceptance event, and indexes as independent facts.

## Aggregate Boundary

- Aggregate root: `AcceptedRunCommit`.
- Identity: `RunId` plus accepted artifact digest.
- Required children: `WorkflowSourceRecord`, `CompiledIrRecord(AcceptedArtifact)`, `RunHeaderRecord`, `JournalEvent::RunAccepted`, status/workflow/action index entries.
- Transaction boundary: one strict Fjall batch commit.
- External acknowledgement boundary: receipt returned only after the transaction is durable.

## Illegal States To Make Unrepresentable

- Accepted run with source but no accepted artifact.
- Accepted run with artifact but no header.
- Accepted run with header/event but missing indexes.
- Accepted artifact whose `accepted_at_seq` is zero/sentinel or differs from `RunAccepted.seq`.
- Strict compiled IR record containing raw `WorkflowParts`.
- Runtime-admitted run derived from loose YAML/raw compiled parts instead of storage-backed `AcceptedArtifact`.
- Index entry pointing to a missing or partially staged accepted run.

## Type Model Recommendations For Downstream Implementation

- Use a validated `AcceptedRunCommitInput` constructor to enforce source/artifact/header/proof/digest coherence before batch staging.
- Use a distinct `StrictCompiledIrPayload::AcceptedArtifact` representation or equivalent discriminator so raw `WorkflowParts` cannot satisfy strict paths.
- Use an `AcceptedRunCommitReceipt` containing run id, workflow id, artifact digest, real `accepted_at_seq`, committed record kinds, and index keys.
- Separate `BatchStaged` from `BatchCommitted` typestates; acknowledgement requires `BatchCommitted`.
- Keep relaxed/backward-compatible read paths explicit and outside strict admission APIs.

## Cross-Bead Dependencies

- `vb-core-accepted-artifact-format`: must supply final accepted artifact schema and raw/legacy rejection rules.
- `vb-core-proof-15-gate`: must supply final 15-gate proof semantics used by strict admission.
- `vb-core-strict-ack-ordering`: broader before-ack proof must refine this bead's accepted-run boundary.
- `vb-qi37.12.2`: typed persistence error propagation must preserve this bead's error taxonomy.

## Review Risks

- If implementation only wraps existing split writes in higher-level orchestration, atomicity remains false.
- If `accepted_at_seq` is assigned before the real journal sequence is known, sequence truth remains false.
- If readback silently accepts raw `WorkflowParts`, strict admission can be bypassed.
- If failure injection only targets commit failure and not stage/codec/index derivation failures, partial construction errors remain unproven.

## Acceptance Of Model

The correct domain model is an accepted-run aggregate with a single durable commit and a fail-closed receipt. Any design that exposes acknowledgement before the receipt, or allows strict raw compiled payloads, violates this contract.
