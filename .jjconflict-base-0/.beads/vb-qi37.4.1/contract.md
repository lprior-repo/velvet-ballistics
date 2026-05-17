# Contract: vb-qi37.4.1 - runtime: Define accepted artifact envelope

## 1. Scope

Define the v1 contract for the accepted artifact envelope and the runtime admission boundary that consumes it. This bead specifies shape, identity, validation, errors, durability ordering, and test obligations only. It does not implement production code or tests.

Authoritative anchors:

- `velvet-ballistics-MASTER.md` Section 63: AI may propose workflows; Velvet verifies them; only accepted artifacts run.
- `velvet-ballistics-MASTER.md` Section 66: runtime admission loads an artifact by digest, verifies it, validates input, checks capabilities and secrets, records `RunAccepted`, then execution may begin.
- Existing storage patterns: `encode_record`/`decode_record`, `CompiledIrRecord`, `RecordKind::CompiledIr`, `MAGIC_COMPILED_ARTIFACT`, and bounded postcard payloads.

## 2. Domain terms

- Accepted artifact: a durable, binary, postcard-encoded record proving that a compiled workflow passed verification gates and may be submitted to runtime admission.
- Storage envelope: the existing 60-byte binary record header plus postcard payload validated by magic, schema version, record kind, payload length, BLAKE3 payload digest, and CRC32C header checksum.
- Artifact digest: the stable key used to load the accepted artifact. For v1 this MUST equal `ir_digest` and MUST be the key in the `compiled_ir` keyspace.
- Workflow digest: BLAKE3 identity of the source workflow definition. It is distinct from `ir_digest`.
- IR digest: BLAKE3 identity of the compiled IR bytes embedded in the artifact.
- Action contract digest: BLAKE3 identity of the action contract set verified for this artifact.
- Verification proof: bounded evidence that required gates passed, warnings are known and attached to gates, and execution-relevant properties are true.
- Run admission: runtime decision record binding a `RunId` to an accepted artifact, input digest, granted capabilities, available secret identifiers, and admission timestamp.

## 3. Design decision: v1 envelope identity

For this bead, v1 accepted artifacts are stored in the existing `compiled_ir` keyspace keyed by `ir_digest`, using the existing storage envelope family:

- `magic = MAGIC_COMPILED_ARTIFACT`
- `record_kind = RecordKind::CompiledIr`
- `payload = postcard(AcceptedArtifactV1)`
- `CompiledIrRecord.digest = artifact.ir_digest`
- `CompiledIrRecord.ir = postcard(AcceptedArtifactV1)`

This resolves the current mismatch where `CompiledIrRecord.ir` sometimes means raw workflow parts and red tests expect an accepted artifact. After this contract, the runtime-facing accepted artifact path MUST treat `CompiledIrRecord.ir` as an accepted artifact payload, not raw loose workflow parts, whenever accepted-artifact admission is required.

Legacy raw compiled workflow storage MAY remain behind relaxed/internal compatibility APIs, but MUST NOT satisfy accepted-artifact admission.

## 4. Contract data shapes

These are contract signatures and semantic shapes, not implementation code.

```rust
pub const ACCEPTED_ARTIFACT_VERSION_V1: &str = "velvet.artifact/v1";
pub const WORKFLOW_LANGUAGE_VERSION_V1: &str = "velvet-ballastics/v1";
pub const VERIFICATION_GATE_COUNT_V1: u8 = 15;

pub struct AcceptedArtifactV1 {
    pub artifact_version: ArtifactVersion,
    pub workflow_name: WorkflowName,
    pub workflow_version: WorkflowLanguageVersion,
    pub workflow_digest: WorkflowDigest,
    pub ir_digest: WorkflowDigest,
    pub ir_bytes: BoundedIrBytes,
    pub action_contract_digest: WorkflowDigest,
    pub verified_at_unix_ms: UnixMillis,
    pub resource_budget: WholeWorkflowBudget,
    pub capabilities: Box<[Capability]>,
    pub required_secrets: Box<[SymbolId]>,
    pub input_schema_digest: WorkflowDigest,
    pub warnings: Box<[VerificationWarningV1]>,
    pub verification: VerificationProofV1,
}

pub struct VerificationProofV1 {
    pub gate_count: GateCount,
    pub gate_statuses: Box<[VerificationGateStatus; 15]>,
    pub bounded: bool,
    pub taint_safe: bool,
    pub retry_safe: bool,
    pub durable: bool,
    pub replayable: bool,
    pub idempotency_keyed: Box<[ActionId]>,
    pub idempotency_attested: Box<[ActionId]>,
}

pub struct VerificationWarningV1 {
    pub code: VerificationWarningCode,
    pub message: BoundedWarningMessage,
    pub gate: VerificationGateNumber,
}

pub struct RunAdmissionV1 {
    pub run: RunId,
    pub artifact_digest: WorkflowDigest,
    pub workflow_digest: WorkflowDigest,
    pub input_digest: WorkflowDigest,
    pub capabilities_granted: Box<[Capability]>,
    pub secrets_available: Box<[SymbolId]>,
    pub admitted_at_unix_ms: UnixMillis,
}
```

Type-first constraints:

- `ArtifactVersion` accepts only `velvet.artifact/v1`.
- `WorkflowLanguageVersion` accepts only `velvet-ballastics/v1`.
- `WorkflowName` is non-empty, bounded, and already name/scope validated.
- `BoundedIrBytes` is non-empty and no larger than `MAX_COMPILED_IR_BYTES` after accounting for the enclosing artifact payload bound.
- `GateCount` for v1 is exactly 15.
- `VerificationGateNumber` is in `1..=15`.
- `BoundedWarningMessage` is non-empty, bounded, and projection-only. Runtime core MUST NOT parse JSON/YAML/HTTP from it.
- `UnixMillis` is non-zero unless a deterministic test clock explicitly supplies zero through a test-only contract.
- Capability and secret arrays are bounded, deterministic-order, and duplicate-free.

## 5. Fallible contract signatures

All fallible operations return `Result<T, Error>` and must not panic.

```rust
pub fn encode_accepted_artifact_v1(
    artifact: &AcceptedArtifactV1,
) -> Result<Vec<u8>, ArtifactEnvelopeError>;

pub fn decode_accepted_artifact_v1(
    encoded_record: &[u8],
) -> Result<AcceptedArtifactV1, ArtifactEnvelopeError>;

pub fn validate_accepted_artifact_v1(
    artifact: &AcceptedArtifactV1,
    storage_key_digest: WorkflowDigest,
) -> Result<ValidatedAcceptedArtifact, ArtifactEnvelopeError>;

pub trait AcceptedArtifactStore: Send + Sync {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<ValidatedAcceptedArtifact, ArtifactEnvelopeError>;
}

pub fn admit_artifact_run_v1(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    run: RunId,
    artifact_digest: WorkflowDigest,
    input: &[u8],
    granted_capabilities: CapabilitySet,
    available_secrets: SecretPresenceSet,
    clock: &dyn AdmissionClock,
    frame_pool: &mut FramePool,
    journal: &mut RuntimeAdmissionJournal,
) -> Result<RunAdmissionV1, AdmissionError>;
```

## 6. Preconditions

### 6.1 Encoding preconditions

- Artifact version is exactly `velvet.artifact/v1`.
- Workflow version is exactly `velvet-ballastics/v1`.
- `workflow_name` is non-empty and bounded.
- `ir_bytes` is non-empty and within `MAX_COMPILED_IR_BYTES`/record payload limits.
- `ir_digest == blake3(ir_bytes)`.
- `workflow_digest`, `ir_digest`, and `action_contract_digest` are non-zero digests.
- `verification.gate_count == 15` and `verification.gate_statuses.len() == 15`.
- Every gate status is pass or warning-only pass; no failing gate is present.
- `bounded`, `taint_safe`, `retry_safe`, `durable`, and `replayable` are true for accepted artifacts.
- Every warning gate is in `1..=15`.
- Capabilities, required secrets, and idempotency action lists are duplicate-free and bounded.

### 6.2 Decoding/loading preconditions

- Encoded bytes are at least `RECORD_HEADER_LEN` bytes.
- Expected magic is `MAGIC_COMPILED_ARTIFACT`.
- Expected record kind is `RecordKind::CompiledIr`.
- Payload length does not exceed the accepted artifact payload bound.
- Payload is postcard bytes for `AcceptedArtifactV1`.
- Caller supplies the storage key digest used to retrieve the record.

### 6.3 Runtime admission preconditions

- If policy requires accepted artifacts, runtime submit path supplies an artifact digest, not raw `CompiledWorkflow`.
- Artifact store can load and validate the artifact by digest.
- Input bytes are bounded and validate against the artifact input schema.
- Granted capabilities cover every declared artifact capability.
- Available secret set contains every declared required secret identifier; secret values are never exposed.
- Run ID is not already active or durably accepted.
- Active run capacity and frame pool capacity are available.
- No frame allocation, run-state insert, or execution begins before all validation preconditions pass.

## 7. Postconditions

### 7.1 Encoding postconditions

- Output is an existing 60-byte storage envelope plus postcard payload.
- Header magic, schema, record kind, header length, payload length, BLAKE3 payload digest, and CRC32C validate with existing codec rules.
- Decoding the output returns an artifact semantically equal to the input.
- No JSON, YAML, or HTTP representation is introduced into runtime core storage.

### 7.2 Loading/validation postconditions

- Returned value is a `ValidatedAcceptedArtifact` newtype that can only be constructed after full envelope and semantic validation.
- `storage_key_digest == artifact.ir_digest`.
- `artifact.ir_digest == blake3(artifact.ir_bytes)`.
- All required v1 verification booleans are true and all 15 gates are accepted.
- Warning gates are in range and warnings never override failed verification.

### 7.3 Runtime admission postconditions

- On `Ok(RunAdmissionV1)`, the run is bound to `artifact_digest == artifact.ir_digest` and `workflow_digest == artifact.workflow_digest`.
- `input_digest == blake3(input)`.
- `capabilities_granted` is the admission-time bounded granted set.
- `secrets_available` includes only identifiers, never values.
- `RunAccepted` is durably recorded before execution begins.
- `RunAdmission` metadata is recorded in the same admission sequence immediately after or atomically with `RunAccepted` as defined by the implementation state. If both events exist, `RunAccepted.seq < RunAdmission.seq` for the same run.
- Under `RuntimePolicy::Strict`, `SyncAll` is completed before returning the run id/admission success.
- Under `RuntimePolicy::Journaled`, the write is queued before execution and the data-loss window is explicit.
- On `Err`, no frame is allocated permanently, no run is inserted, no execution occurs, and no `RunAccepted` event is recorded.

## 8. Invariants

- Runtime execution invariant: no workflow executes unless it is admitted from a validated accepted artifact when accepted-artifact policy is required.
- Digest separation invariant: `workflow_digest`, `ir_digest`, `action_contract_digest`, and `input_digest` are distinct semantic roles and MUST NOT be conflated even if byte values coincidentally match.
- Storage key invariant: accepted artifact records are keyed by `ir_digest`; load-time validation rejects records whose internal `ir_digest` differs from the key.
- Envelope invariant: accepted artifacts reuse the existing storage envelope validation path; no ad hoc runtime envelope is permitted.
- Proof invariant: accepted artifacts require all 15 verification gates for v1; the existing 2-gate storage proof is insufficient for runtime admission.
- Capability invariant: declared required capabilities are immutable after artifact acceptance; admission grants may be equal to or a superset of declared requirements.
- Secret invariant: artifacts and admission records contain only secret identifiers/presence, never secret values.
- Durability invariant: `RunAccepted` is the durability boundary for a run; storage artifact acceptance durability is separate from run admission durability.
- Bounded-resource invariant: all arrays, messages, IR bytes, inputs, and encoded records have explicit upper bounds checked before allocation or persistence.
- Compatibility invariant: relaxed/internal raw submit may exist only when accepted-artifact requirement is disabled; it cannot be mistaken for an accepted artifact path.

## 9. Error taxonomy

### 9.1 ArtifactEnvelopeError

- `BadMagic { found }`: storage envelope magic is not `MAGIC_COMPILED_ARTIFACT`.
- `UnsupportedSchemaVersion { version }`: envelope schema is newer than supported.
- `MigrationRequired { from, to }`: envelope schema is older than supported.
- `BadRecordKind { found }`: record kind is not `RecordKind::CompiledIr` for v1 accepted artifact storage.
- `HeaderLengthMismatch { found }`: header length is not `RECORD_HEADER_LEN`.
- `HeaderChecksumMismatch`: CRC32C header checksum validation fails.
- `PayloadDigestMismatch`: envelope BLAKE3 payload digest validation fails.
- `PayloadTooLarge { len, max }`: encoded payload exceeds configured bound.
- `UnexpectedEof`: header or payload bytes are truncated.
- `PostcardDecodeFailed`: payload is not a valid `AcceptedArtifactV1` postcard value.
- `UnsupportedArtifactVersion { version }`: artifact version is not `velvet.artifact/v1`.
- `UnsupportedWorkflowVersion { version }`: workflow version is not `velvet-ballastics/v1`.
- `EmptyWorkflowName`: workflow name is empty after decoding.
- `InvalidWorkflowName`: workflow name violates name/scope contract.
- `EmptyIr`: compiled IR bytes are empty.
- `IrDigestMismatch { expected, computed }`: internal IR digest does not match `blake3(ir_bytes)`.
- `StorageKeyDigestMismatch { key, artifact }`: loaded storage key digest differs from `artifact.ir_digest`.
- `ZeroDigest { field }`: a required digest is all zeroes.
- `InvalidGateCount { found }`: verification gate count is not 15.
- `VerificationGateFailed { gate }`: a required verification gate did not pass.
- `MissingRequiredProofFlag { flag }`: bounded/taint/retry/durable/replayable flag is false.
- `InvalidWarningGate { gate }`: warning gate is outside `1..=15`.
- `DuplicateCapability { capability }`: artifact capability list contains a duplicate.
- `DuplicateSecret { secret }`: required secret list contains a duplicate.
- `DuplicateActionId { list, action }`: idempotency proof list contains a duplicate.
- `BoundExceeded { field, len, max }`: any bounded collection or string exceeds its maximum.

### 9.2 AdmissionError

- `AdmissionRequired`: raw submit was attempted while accepted-artifact admission is required.
- `ArtifactNotFound { digest }`: store has no record for requested artifact digest.
- `ArtifactInvalid { digest, source }`: store returned an envelope or artifact that failed validation.
- `InputTooLarge { len, max }`: input bytes exceed runtime bound.
- `InputSchemaMismatch { schema_digest }`: input does not validate against artifact schema.
- `CapabilityDenied { action, required, granted }`: declared capability is not covered by grants.
- `SecretUnavailable { secret }`: required secret identifier is absent from runtime secret store.
- `RunAlreadyExists { run }`: run is already active or durably accepted.
- `ActiveRunCapacityExceeded { capacity }`: no active run capacity remains.
- `FrameAllocationFailed`: frame pool cannot allocate a frame within bounds.
- `AdmissionJournalFailed { source }`: `RunAccepted`/`RunAdmission` could not be recorded.
- `StrictDurabilityFailed { source }`: strict admission could not complete `SyncAll`.
- `ClockUnavailable`: admission timestamp could not be obtained from the provided clock.

## 10. Acceptance criteria

- Contract defines one v1 accepted artifact envelope stored in `compiled_ir` keyed by `ir_digest`.
- Contract explicitly distinguishes `workflow_digest`, `ir_digest`, `action_contract_digest`, and `input_digest`.
- Contract requires full 15-gate verification proof for runtime-admissible artifacts.
- Contract requires existing binary envelope validation; runtime core does not add JSON/YAML/HTTP parsing.
- Contract requires railway-oriented `Result<T, Error>` for all fallible operations.
- Contract defines typed error variants for envelope corruption, semantic invalidity, and admission rejection.
- Contract defines `RunAccepted` durability ordering before execution and strict `SyncAll` behavior.
- Contract states raw `submit_direct` is rejected with `AdmissionRequired` whenever accepted artifacts are required.
- Contract includes Martin Fowler Given/When/Then scenarios for happy, error, edge, and invariant cases.

## 11. Martin Fowler Given/When/Then scenarios

### Scenario 1: Valid accepted artifact roundtrips through the storage envelope

Given a valid `AcceptedArtifactV1` with all 15 gates passing and `ir_digest == blake3(ir_bytes)`
When it is encoded with `MAGIC_COMPILED_ARTIFACT` and decoded as `RecordKind::CompiledIr`
Then decoding succeeds
And the decoded artifact has the same semantic fields
And the storage envelope payload digest and header checksum are valid.

### Scenario 2: Runtime admits a run from a valid accepted artifact

Given accepted-artifact admission is required
And the artifact store returns a validated artifact for digest `D`
And input validates against the artifact schema
And all required capabilities are granted
And all required secrets are present
When the runtime admits run `R` for artifact digest `D`
Then admission returns `RunAdmissionV1`
And `RunAccepted` is recorded before execution
And `RunAdmission` binds `R`, `D`, workflow digest, input digest, capabilities, secret identifiers, and timestamp.

### Scenario 3: Raw submit is rejected when accepted artifacts are required
Given runtime policy requires accepted artifacts
When a caller submits a raw `CompiledWorkflow` through `submit_direct` or equivalent shard command
Then admission fails with `AdmissionRequired`
And no frame is allocated
And no run state is inserted
And no `RunAccepted` event is recorded.

### Scenario 4: Artifact payload corruption is rejected before runtime mutation
Given a stored accepted artifact record has a corrupted payload byte
When runtime loads the artifact by digest
Then envelope validation fails with `PayloadDigestMismatch` or `PostcardDecodeFailed`
And admission maps the failure to `ArtifactInvalid`
And no execution-visible state changes occur.

### Scenario 5: Internal digest mismatch is rejected
Given an accepted artifact is stored under key digest `K`
And the decoded artifact contains `ir_digest = D` where `D != K`
When load-time validation runs
Then validation fails with `StorageKeyDigestMismatch`
And the artifact cannot be used for runtime admission.

### Scenario 6: Two-gate legacy proof is not runtime-admissible
Given a legacy artifact has `gate_count = 2`
When validation requires `AcceptedArtifactV1`
Then validation fails with `InvalidGateCount { found: 2 }`
And runtime admission rejects it as `ArtifactInvalid`.

### Scenario 7: Missing capability rejects admission
Given a valid accepted artifact declares capability `network.github` for action `A`
And the granted capability set does not grant it
When runtime admission checks capabilities
Then admission fails with `CapabilityDenied`
And no `RunAccepted` event is recorded.

### Scenario 8: Missing secret rejects admission without exposing values
Given a valid accepted artifact declares required secret identifier `github_token`
And the runtime secret store reports it absent
When runtime admission checks secrets
Then admission fails with `SecretUnavailable`
And neither the artifact nor the error includes any secret value.

### Scenario 9: Strict admission syncs before returning
Given runtime policy is `Strict`
And all admission validations pass
When `RunAccepted` and `RunAdmission` are appended
Then `SyncAll` completes before admission returns success
And if `SyncAll` fails, admission returns `StrictDurabilityFailed` and execution does not begin.

### Scenario 10: Journaled admission documents the data-loss window
Given runtime policy is `Journaled`
And all admission validations pass
When `RunAccepted` is queued
Then admission may return before disk sync
And execution may begin only after the journal append accepts the event into its queue.

### Scenario 11: Oversized artifact payload is rejected
Given encoded accepted artifact bytes exceed the configured maximum payload length
When decoding validates the storage envelope
Then decoding fails with `PayloadTooLarge`
And no postcard decode or runtime admission is attempted.

### Scenario 12: Warning gate bounds are enforced
Given an accepted artifact contains a warning with gate `16`
When semantic validation runs
Then validation fails with `InvalidWarningGate { gate: 16 }`.

## 12. Proof obligations for later implementation states

- Unit tests prove storage envelope roundtrip, bad magic, bad kind, bad schema, payload digest mismatch, header checksum mismatch, EOF, and payload-too-large behavior.
- Unit tests prove semantic artifact validation for version, workflow version, zero digests, IR digest mismatch, storage key mismatch, gate count, proof flags, warning gates, duplicates, and bounds.
- Runtime tests prove accepted-artifact admission success and every `AdmissionError` variant.
- Integration tests prove `submit_artifact` followed by admission-aware runtime submit executes only after `RunAccepted`.
- Regression tests prove raw `submit_direct` is rejected when accepted-artifact policy is required.
- Property tests generate bounded artifacts and corrupted envelopes to verify no panics and precise error classification.
- CI proof uses `moon ci` as canonical gate.

## 13. Out of scope

- Implementing the verifier pipeline gates.
- Implementing production code or tests for this bead.
- Changing CLI output formats.
- Designing JSON/YAML/HTTP projection formats.
- Proving external idempotency behavior beyond attestation and key shape.
- Benchmark or performance claims without measured evidence.

## 14. Risk notes

- Existing storage code currently persists raw workflow parts in `CompiledIrRecord.ir`; migration must avoid silently treating legacy payloads as accepted artifacts.
- Existing runtime admission checks only digest existence; this contract requires load and full validation.
- Existing `VerificationWarning` gate range is `1..=13`; v1 accepted artifacts require `1..=15` to match the MASTER document.
- Existing `RunAdmission` lacks input digest, secrets, workflow digest, and timestamp; event schema changes may affect projections and compatibility.
- Reusing `RecordKind::CompiledIr` minimizes storage family churn but requires clear semantic versioning inside the payload.
- Strict durability for artifact acceptance and strict durability for run admission are separate boundaries and must not be conflated.
