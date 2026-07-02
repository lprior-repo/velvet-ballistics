# Domain Model Review: vb-core-yaml-e2e-chain

## Verdict

STATUS: CONTRACT_MODEL_REPAIRED_WITH_EXPLICIT_DOWNSTREAM_BLOCKERS

The domain split is usable for downstream proof/test/implementation, but two risks are release-critical: raw compiled IR storage must not bypass accepted-artifact admission, and storage/runtime gate-count parity must be resolved.

State 6 repair note: those risks are now explicit contract blockers, not implicit open questions. Downstream may proceed only after the repaired `proof-obligations.jsonl` exact commands pass or the named blocker/waiver fields are independently approved.

## Bounded Contexts

- Authoring/compile context: strict YAML bytes, parser profile, validation, lowering, `CompiledWorkflow`/artifact production.
- Artifact context: source digest, artifact digest, accepted artifact envelope, verification proof flags, required capabilities, accepted sequence.
- Runtime admission context: strict policy, artifact store, accepted artifact validation, capability grant checks, run header and RunAccepted/RunAdmission evidence.
- Journal/recovery context: Fjall journal, source/IR records, event stream, snapshots, replay summary, recovery frame seed, typed recovery errors.
- Operator evidence context: CLI `run`, `submit`, `events`, `inspect`, and recovery/doctor-adjacent outputs.

## Aggregate Roots

- `YamlOriginRun`: identity `RunId`; owns lifecycle from source accepted to terminal/recovered state.
- `AcceptedWorkflowArtifact`: identity `artifact_digest`; owns serialized IR, proof flags, required capabilities, accepted sequence, and source binding.
- `DurableJournal`: identity `store/run_id`; owns ordered durable events and recovery source of truth.

## Value Objects

- `YamlSourceBytes`, `SourceDigest`, `ArtifactDigest`, `AcceptedSequence`, `CapabilitySet`, `RunHeader`, `JournalSequence`, `RecoveryFrameSeed`.

## Illegal States To Make Unrepresentable Or Fail Closed

- Runtime admission from YAML bytes.
- Strict runtime admission from raw `WorkflowParts` when an accepted artifact envelope is required.
- Accepted artifact with gate count/proof flags below runtime's strict threshold.
- Source digest used as artifact digest without role evidence.
- Acknowledged runtime state whose source/artifact/header/journal evidence was not persisted.
- Recovery that reparses YAML or silently hydrates an empty frame for a non-empty run.
- Inspect/events success projection without corresponding durable journal prefix.

## Typed Error Completeness Check

- Strict YAML invalid: covered by `StrictYamlRejected`-class compile error.
- Source digest mismatch: covered by `WorkflowSourceDigestMismatch`/payload mismatch.
- Artifact digest mismatch: covered by `CompiledIrDigestMismatch`/admission mismatch.
- Missing/invalid accepted artifact: must be explicit admission error, not generic I/O.
- Gate-count/proof mismatch: must be explicit invalid-artifact error.
- Capability mismatch: must be explicit capability error.
- Durability failure before ack: must be explicit durability/admission error with no runnable state.
- Replay divergence/corruption/no data: covered by recovery typed errors.
- YAML reparse during recovery: must be executable evidence failure; if no runtime error variant exists, downstream must add one or prove impossible via dependency/static boundary.

## State 6 Contract Repair Requirements

- Strict YAML rejection now has a focused obligation (`STRICT-YAML-012`) separate from recovery corruption.
- Every fail-closed error variant now has an exact scenario obligation (`ERR-*`) with executable Cargo command evidence.
- Verus digest-role obligations now use the existing executable Verus command and carry a shell-linkage waiver instead of `BLOCKED_DISCOVERY` placeholders.
- Kani admission matrix remains a valid downstream blocker (`KANI-ADMIT-023`) because the command is exact but the harness is not discoverable until source/proof integration is allowed.
- Miri codec evidence is required for this release-critical parser/codec bead (`MIRI-CODEC-024`), not optional.

## Contract Parity Findings

- Storage accepted artifact proof count and runtime proof count are not obviously equal from State 2 map. This is a blocking downstream invariant: strict runtime contract must consume the same proof shape storage emits.
- CLI strict run/submit must prove it stores the accepted artifact envelope, not just source plus raw compiled IR, before runtime admission.
- Existing recovery code appears storage/event based and YAML-free; the missing evidence is end-to-end chain proof from YAML-origin accepted artifact into recovery.

## Review Requirements Before Implementation Consumption

- Independent contract-verification-reviewer must approve or reject this contract before test planning or implementation consumes it.
- Any downstream waiver must name the exact clause, owner, expiry, limitation, and compensating evidence.
