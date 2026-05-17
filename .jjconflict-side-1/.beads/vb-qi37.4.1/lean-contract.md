# Lean Contract Projection: vb-qi37.4.1 - Accepted Artifact Envelope

## Boundary

- **Lean-owned kernel**: Envelope encoding/decoding validation, digest computation, artifact structure validation, gate count validation, proof flag validation, idempotency list deduplication, secret/capability list deduplication.
- **Rust/runtime shell**: Journal persistence, `RunAccepted`/`RunAdmission` durability, clock access, frame pool allocation, capability/secret runtime checking, policy enforcement.
- **External systems excluded from Lean proof**: Fjall storage backend, network, external processes, clock hardware.

## Lean-Owned Clauses

### THM-ENV-001: Envelope Roundtrip Integrity
- **Contract clause**: Section 7.1 (Encoding postconditions)
- **Rust/spec target**: `vb_storage::admission::encode_accepted_artifact_v1` / `decode_accepted_artifact_v1`
- **Lean module**: `VelvetBallistics.ArtifactEnvelope`
- **Theorem shape**: `roundtrip_preserves_artifact : ∀ (a : Artifact), encode a >>= decode = some a`
- **Model**: Abstract `Artifact` with fields: `artifact_version`, `workflow_name`, `workflow_version`, `workflow_digest`, `ir_digest`, `ir_bytes`, `action_contract_digest`, `verified_at`, `resource_budget`, `capabilities`, `required_secrets`, `input_schema_digest`, `warnings`, `verification`
- **Refinement**: Rust `AcceptedArtifactV1` encodes to postcard bytes; Lean model is a pure data structure with equivalent fields; decode validates envelope header integrity (magic, schema, kind, checksum) and reconstructs the artifact
- **Shell exclusions**: No I/O, no storage, no wall-clock time, no dynamic allocation after initial capacity check
- **Evidence command**: `lake build` on `VelvetBallistics.ArtifactEnvelope`

### THM-DIGEST-001: IR Digest Correctness
- **Contract clause**: Section 6.1 (ir_digest == blake3(ir_bytes))
- **Rust/spec target**: `blake3::hash` on `ir_bytes`
- **Lean module**: `VelvetBallistics.Digest`
- **Theorem shape**: `ir_digest_correct : ∀ (artifact : Artifact), artifact.ir_digest = blake3(artifact.ir_bytes)`
- **Model**: BLAKE3 hash function modeled as pure function from byte vector to 32-byte digest
- **Refinement**: Rust `blake3::hash` call is deterministic; Lean BLAKE3 model is bit-exact for the first 32 bytes
- **Shell exclusions**: No I/O, no storage, no side effects
- **Evidence command**: `lake build` on `VelvetBallistics.Digest`

### THM-GATE-001: Gate Count Invariant
- **Contract clause**: Section 6.1 (verification.gate_count == 15)
- **Rust/spec target**: `submit_artifact` gate count validation
- **Lean module**: `VelvetBallistics.Verification`
- **Theorem shape**: `gate_count_positive : ∀ (proof : VerificationProof), proof.gate_count > 0`
- **Theorem shape**: `gate_count_matches_statuses : ∀ (proof : VerificationProof), proof.gate_count = list.length proof.gate_statuses`
- **Model**: `VerificationProof` with `gate_count : nat` and `gate_statuses : list GateStatus`
- **Refinement**: v1 requires exactly 15 gates; Rust validation checks `ADMISSION_GATE_COUNT == 15`
- **Shell exclusions**: No I/O, no storage, no runtime state
- **Evidence command**: `lake build` on `VelvetBallistics.Verification`

### THM-PROOF-001: Proof Flags Invariant
- **Contract clause**: Section 6.1 (bounded, taint_safe, retry_safe, durable, replayable == true)
- **Rust/spec target**: `submit_artifact` proof flag validation
- **Lean module**: `VelvetBallistics.Verification`
- **Theorem shape**: `proof_flags_imply_accepted : ∀ (proof : VerificationProof), proof.bounded ∧ proof.taint_safe ∧ proof.retry_safe ∧ proof.durable ∧ proof.replayable → proof.gate_count = 15`
- **Model**: `VerificationProof` with boolean proof flags
- **Refinement**: For an artifact to be accepted, all five proof flags must be true; this is enforced by Rust validation
- **Shell exclusions**: No I/O, no storage, no runtime state
- **Evidence command**: `lake build` on `VelvetBallistics.Verification`

### THM-DEDUP-001: Idempotency List Deduplication
- **Contract clause**: Section 6.1 (idempotency lists are duplicate-free)
- **Rust/spec target**: `validate_accepted_artifact_v1` idempotency check
- **Lean module**: `VelvetBallistics.Idempotency`
- **Theorem shape**: `no_duplicates_keyed : ∀ (ids : list ActionId), duplicate_free ids → no_duplicate_in_list ids`
- **Theorem shape**: `no_duplicates_attested : ∀ (ids : list ActionId), duplicate_free ids → no_duplicate_in_list ids`
- **Model**: `list ActionId` with duplicate-free constraint
- **Refinement**: Rust validation checks for duplicates before accepting artifact
- **Shell exclusions**: No I/O, no storage
- **Evidence command**: `lake build` on `VelvetBallistics.Idempotency`

### THM-CAP-001: Capability List Deduplication
- **Contract clause**: Section 6.1 (capabilities are duplicate-free)
- **Rust/spec target**: `validate_accepted_artifact_v1` capability check
- **Lean module**: `VelvetBallistics.Capability`
- **Theorem shape**: `no_duplicate_capabilities : ∀ (caps : list Capability), duplicate_free caps`
- **Model**: `list Capability` with duplicate-free constraint
- **Refinement**: Rust validation checks for duplicates before accepting artifact
- **Shell exclusions**: No I/O, no storage
- **Evidence command**: `lake build` on `VelvetBallistics.Capability`

### THM-SECRET-001: Secret List Deduplication
- **Contract clause**: Section 6.1 (required secrets are duplicate-free)
- **Rust/spec target**: `validate_accepted_artifact_v1` secret check
- **Lean module**: `VelvetBallistics.Secret`
- **Theorem shape**: `no_duplicate_secrets : ∀ (secrets : list SymbolId), duplicate_free secrets`
- **Model**: `list SymbolId` with duplicate-free constraint
- **Refinement**: Rust validation checks for duplicates before accepting artifact
- **Shell exclusions**: No I/O, no storage
- **Evidence command**: `lake build` on `VelvetBallistics.Secret`

### THM-WARNING-001: Warning Gate Range
- **Contract clause**: Section 6.1 (warning gates in 1..=15)
- **Rust/spec target**: `validate_accepted_artifact_v1` warning gate validation
- **Lean module**: `VelvetBallistics.Verification`
- **Theorem shape**: `warning_gate_in_range : ∀ (warnings : list VerificationWarning), ∀ (w ∈ warnings), 1 ≤ w.gate ∧ w.gate ≤ 15`
- **Model**: `list VerificationWarning` where each warning has `gate : nat`
- **Refinement**: Rust validation rejects any warning with gate outside 1..=15
- **Shell exclusions**: No I/O, no storage
- **Evidence command**: `lake build` on `VelvetBallistics.Verification`

### THM-STORAGE-001: Storage Key Matches Artifact IR Digest
- **Contract clause**: Section 7.2 (storage_key_digest == artifact.ir_digest)
- **Rust/spec target**: `validate_accepted_artifact_v1` storage key check
- **Lean module**: `VelvetBallistics.Storage`
- **Theorem shape**: `storage_key_matches_digest : ∀ (artifact : Artifact) (key : Digest), key = artifact.ir_digest`
- **Model**: Storage key is exactly the artifact's `ir_digest`
- **Refinement**: Load-time validation rejects records where internal ir_digest differs from the storage key
- **Shell exclusions**: No I/O, no storage backend, only validation logic
- **Evidence command**: `lake build` on `VelvetBallistics.Storage`

### THM-ADMISSION-001: Digest Separation Invariant
- **Contract clause**: Section 8 (workflow_digest, ir_digest, action_contract_digest, input_digest are distinct)
- **Rust/spec target**: Runtime admission digest binding
- **Lean module**: `VelvetBallistics.Digest`
- **Theorem shape**: `digestes_distinct : ∀ (a : Artifact) (input : bytes), a.workflow_digest ≠ a.ir_digest ∧ a.ir_digest ≠ a.action_contract_digest ∧ input_digest ≠ a.ir_digest`
- **Model**: Four distinct digest roles in the system
- **Refinement**: The contract requires these to be semantically distinct even if byte values coincidentally match
- **Shell exclusions**: No I/O, no storage, no runtime state
- **Evidence command**: `lake build` on `VelvetBallistics.Digest`

## Waivers

- **WAIVER-ADMISSION-001**: Runtime admission preconditions (capability coverage, secret presence, frame allocation, run capacity) require runtime shell access and cannot be Lean-proven in isolation. Compensating evidence: Kani bounded model check (`kani::verify_admission_preconditions`) plus integration tests proving correct rejection behavior.
- **WAIVER-ADMISSION-002**: Durability ordering (`RunAccepted.seq < RunAdmission.seq`) and strict/journaled policy behavior require storage backend semantics. Compensating evidence: storage integration tests plus `cargo-fuzz` for adversarial journal interleavings.
- **WAIVER-ADMISSION-003**: Input schema validation against `input_schema_digest` requires the workflow IR's input schema parser. Compensating evidence: proptest for schema validation plus integration tests.
- **WAIVER-ADMISSION-004**: CRC32C header checksum and BLAKE3 payload digest validation are third-party library functions (byteorder, blake3). Compensating evidence: library unit tests, integration tests, and fuzzing of the envelope codec path.

## Non-goals

- Proving Fjall storage backend durability or crash recovery
- Proving clock behavior or time-based ordering
- Proving async/scheduling behavior in the runtime shell
- Proving external process IPC behavior
- Proving generated Rust workflow code equivalence (that is a separate contract)
