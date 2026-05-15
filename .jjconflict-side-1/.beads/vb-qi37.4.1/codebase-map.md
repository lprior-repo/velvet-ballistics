# Codebase Map: vb-qi37.4.1 - runtime: Define accepted artifact envelope

## Scope

- This map covers accepted artifact envelopes, artifact admission, verification proof data, runtime admission, storage codec/envelope patterns, and the authoritative MASTER.md acceptance contract.
- No production or test code was modified during this State 2 retry.
- No `bd` writes were performed during this State 2 retry.

## Authoritative Contract Anchors

- `velvet-ballistics-MASTER.md:2974` defines Section 63, "Plan Verifier and Accepted Artifacts".
- `velvet-ballistics-MASTER.md:2978` states the core rule: AI may propose workflows, Velvet verifies them, and only accepted artifacts run.
- `velvet-ballistics-MASTER.md:2982` lists the 15 verification gates from YAML profile through observability evidence.
- `velvet-ballistics-MASTER.md:3005` states the runtime must not execute anything that is not an accepted artifact.
- `velvet-ballistics-MASTER.md:3007` sketches the intended `AcceptedArtifact` record with `artifact_version`, `workflow_name`, `workflow_version`, `workflow_digest`, `ir_digest`, `action_contract_digest`, `verified_at`, `resource_budget`, `capabilities`, `warnings`, and `verification`.
- `velvet-ballistics-MASTER.md:3026` sketches the intended `VerificationProof` shape: bounded, taint-safe, retry-safe, durable, replayable, keyed/attested idempotency lists.
- `velvet-ballistics-MASTER.md:3043` requires runs to bind to the accepted artifact by digest, not loose YAML or unverified `CompiledWorkflow`.
- `velvet-ballistics-MASTER.md:3045` says accepted artifacts are stored in `compiled_ir` keyed by `ir_digest`, and the artifact record wraps IR with verification metadata.
- `velvet-ballistics-MASTER.md:3285` defines Section 66, "Runtime Admission Gate".
- `velvet-ballistics-MASTER.md:3289` requires the runtime to accept only verified artifacts and treats `RunAccepted` persistence as the durability boundary.
- `velvet-ballistics-MASTER.md:3293` specifies admission flow: load artifact by digest, verify digest, validate input schema, bind workflow digest, check capabilities, check secrets, allocate frame, record `RunAccepted`, return run id.
- `velvet-ballistics-MASTER.md:3307` sketches the intended runtime `RunAdmission` record with run, artifact digest, input digest, granted capabilities, available secrets, and admission timestamp.
- `velvet-ballistics-MASTER.md:3326` requires `RunAccepted` to be durably recorded before execution begins, with strict mode requiring `SyncAll` before returning `run_id`.
- `velvet-ballistics-MASTER.md:3339` describes migration from `submit_direct` to admission-aware `submit_artifact` when accepted artifacts are required.

## Relevant Existing Files

- `crates/vb_storage/src/admission.rs`
  - Defines current storage-side `VerificationWarning`, `VerificationProof`, and `AcceptedArtifact`.
  - Current `AcceptedArtifact` is minimal: `digest`, serialized `ir`, `verification`, `accepted_at_seq`, and `required_capabilities`.
  - `submit_artifact` gates by `RuntimePolicy`: `Relaxed` persists without checks, `Journaled` and `Strict` perform structure and checksum checks, and `Strict` calls `journal.persist_strict()`.
  - `admit_compiled_artifact` is a lower-level path that validates structure/checksum and stores `CompiledIrRecord`.
  - Suspected touchpoint for the contract: either expand or clearly wrap this type to become the accepted artifact envelope required by MASTER.md.

- `crates/vb_storage/src/codec.rs`
  - Defines reusable 60-byte binary record envelope via `encode_record`, `decode_record`, `encode_record_header`, and `decode_record_header`.
  - Header fields are magic, schema version, record kind, header length, payload length, sequence, BLAKE3 payload digest, and CRC32C header checksum.
  - `validate_kind_family` binds `MAGIC_COMPILED_ARTIFACT` to `RecordKind::CompiledIr`.
  - Pattern to reuse: accepted artifact envelope should reuse the same encode/decode integrity rules instead of inventing an ad hoc envelope.

- `crates/vb_storage/src/constants.rs`
  - Defines keyspace names, key prefixes, magic constants, and max payload limits.
  - Relevant constants: `KEYSPACE_COMPILED_IR`, `PREFIX_COMPILED_IR`, `MAGIC_COMPILED_ARTIFACT`, `RECORD_HEADER_LEN`, `CURRENT_SCHEMA_VERSION`, `MAX_COMPILED_IR_BYTES`.
  - Suspected touchpoint if the envelope requires a distinct magic/kind/version for accepted artifact records rather than overloading compiled IR.

- `crates/vb_storage/src/records.rs`
  - Defines `RecordKind::CompiledIr = 2` and `CompiledIrRecord { digest, ir }`.
  - Defines `RecordKind::RunAccepted = 10` and `RecordKind::RunAdmission = 24`.
  - Suspected touchpoint if accepted artifacts need their own record kind or if `CompiledIrRecord.ir` becomes an encoded `AcceptedArtifact` payload.

- `crates/vb_storage/src/journal.rs`
  - `put_compiled_ir` encodes `CompiledIrRecord` with `MAGIC_COMPILED_ARTIFACT`, `RecordKind::CompiledIr`, sequence `0`, and `MAX_COMPILED_IR_BYTES`.
  - `compiled_ir` decodes the same envelope and returns `Option<CompiledIrRecord>`.
  - Pattern to reuse: storage callers should load accepted artifacts by digest through this path, then verify both storage envelope digest and internal artifact identity.

- `crates/vb_storage/src/artifacts.rs`
  - Adds list/remove/exists helpers for compiled IR artifacts.
  - `artifact_exists` is currently a digest-key existence check only; runtime admission currently depends on this general presence check through storage-backed `ArtifactStore`.

- `crates/vb_storage/src/batch.rs`
  - Batch `put_compiled_ir` uses the same `MAGIC_COMPILED_ARTIFACT`/`RecordKind::CompiledIr` envelope and staged atomic insert pattern.
  - Relevant if contract requires atomic persistence of accepted artifact plus run header/admission journal records.

- `crates/vb_storage/src/events.rs`
  - Defines durable `JournalEvent::RunAccepted { run, seq, workflow }`.
  - Defines `JournalEvent::RunAdmission { run, seq, artifact_digest, granted_capabilities, policy }`.
  - Current `RunAdmission` event is missing input digest, secrets availability, and admission timestamp from MASTER.md Section 66.

- `crates/vb_storage/src/types.rs`
  - Defines `RecordEnvelope` and `RecordHeader` used by the codec.
  - `RecordEnvelope` exposes only magic/schema/kind/sequence after decode; full digest/checksum are held in `RecordHeader` during validation.
  - If `rust-contract` needs a named accepted artifact envelope type, this file is the storage pattern to mirror.

- `crates/vb_runtime/src/admission.rs`
  - Defines runtime-side `RunAdmission` with private `artifact_digest`, `run_id`, granted capabilities, and policy.
  - Defines `ArtifactStore` trait with `compiled_ir_exists(digest) -> bool`.
  - `admit_run` currently accepts based on artifact existence under `Strict`/`Journaled`, while `Relaxed` always succeeds.
  - Gap: it does not load/decode an accepted artifact envelope, verify internal proof fields, validate inputs/secrets, or bind an `ir_digest` distinct from workflow digest.

- `crates/vb_runtime/src/shard/lifecycle.rs`
  - `handle_submit_with_inputs` checks duplicate/capacity, computes `digest = workflow.digest()`, calls `build_admission`, allocates/seeds a frame, appends runtime journal events, inserts `RunState`, then immediately drives the run.
  - `build_admission` calls `admit_run(self.artifact_store.as_ref(), self.policy, digest, run, caps)` and maps errors to runtime errors.
  - Suspected touchpoint: accepted-artifact enforcement must happen before frame allocation and before `drive_run`, and `RunAccepted`/`RunAdmission` durability ordering must be made explicit.

- `crates/vb_runtime/src/shard/types.rs`
  - `ShardCommand::Submit` and `SubmitWithInputs` both carry raw `CompiledWorkflow` plus capabilities.
  - `RunState` stores `admission: Option<crate::admission::RunAdmission>`.
  - `ShardConfig` carries `policy: vb_core::policy::RuntimePolicy`.
  - Gap: command shape still supports raw workflow submission; no artifact digest/input envelope command exists yet.

- `crates/vb_runtime/src/runtime.rs`
  - Public `submit_direct` and `submit_compiled_with_inputs` enqueue raw compiled workflow commands.
  - MASTER.md calls out this migration path and says accepted artifact mode should reject direct submit when required.

## Existing Tests And Evidence Locations

- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`
  - Red/contract-style test file for durability gate behavior.
  - Covers `submit_artifact`, `admit_compiled_artifact`, duplicate admission, record envelope invariants, and BDD scenarios.
  - Important existing mismatch: one test at lines 856-857 attempts to decode `record.ir` as `AcceptedArtifact`, while current production `submit_artifact` stores postcard-encoded `WorkflowParts` bytes in `CompiledIrRecord.ir`.

- `crates/vb_storage/src/tests.rs`
  - Storage codec and journal tests.
  - Relevant tests include compiled IR envelope roundtrip near lines 2130-2153, record envelope header field checks near lines 3563+, and many `RunAccepted` journal event envelope tests.

- `crates/vb_storage/src/security_tests.rs`
  - Adversarial storage envelope and digest validation tests; useful for corruption/forgery cases.

- `crates/vb_storage/src/proptests.rs`
  - Property tests for storage encoding/key/admission invariants.

- `crates/vb_runtime/src/shard/tests.rs`
  - Runtime shard submission behavior tests.
  - Relevant area: `SubmitWithInputs` tests near lines 3270+, duplicate/capacity rejection, and black-hat tests around arbitrary input slot writes near lines 4187 and 4395.

- `crates/vb_runtime/src/admission.rs` unit tests
  - Cover runtime `RunAdmission`, artifact-not-found, capability checks, and `admit_run` behavior.

- `crates/velvet_ballastics/tests/admission_evidence_integration.rs`
  - Cross-crate integration tests for `submit_artifact` plus runtime execution.
  - Current happy path persists an artifact but still runs via `runtime.submit_direct`, so it proves storage plus raw runtime execution, not full accepted-artifact runtime admission.

- `crates/velvet_ballastics/tests/cross_crate_adversarial.rs`
  - Contains cross-crate `RunAccepted` and raw `submit_direct` coverage.

- `crates/velvet_ballastics/tests/cli_integration.rs`
  - Contains CLI flows that assert `RunAccepted` appears in event output and may need update once accepted artifact admission is surfaced in CLI.

## Patterns To Reuse

- Reuse `encode_record`/`decode_record` and the existing 60-byte envelope rules for any accepted artifact payload.
- Reuse `MAGIC_COMPILED_ARTIFACT` and `RecordKind::CompiledIr` only if the contract explicitly says accepted artifact is a compiled-IR-family record; otherwise define a distinct family/kind in the contract before implementation.
- Reuse `WorkflowDigest`/BLAKE3 conventions, but distinguish clearly between source workflow digest and compiled IR/artifact digest because MASTER.md lists both.
- Reuse `RuntimePolicy` tiering already present in `submit_artifact` and runtime shard config.
- Reuse `RunAdmission` as the runtime binding point, but the contract should decide whether runtime uses storage-side proof fields directly or a normalized runtime admission record.
- Reuse `JournalEvent::RunAdmission` as the evidence event, but the contract should specify missing fields and ordering relative to `RunAccepted`.

## Suspected Implementation Touchpoints For Later States

- Storage accepted artifact envelope type: likely `crates/vb_storage/src/admission.rs` and possibly `records.rs`/`constants.rs` if a new record kind or version tag is needed.
- Storage load/verify API: likely near `FjallJournal::compiled_ir` or a new helper beside `crates/vb_storage/src/artifacts.rs` to return a decoded/validated accepted artifact, not just a `CompiledIrRecord`.
- Runtime artifact store trait: `crates/vb_runtime/src/admission.rs` probably needs more than `compiled_ir_exists`; it may need `load_accepted_artifact` or `verify_accepted_artifact` semantics.
- Runtime submit path: `crates/vb_runtime/src/runtime.rs`, `crates/vb_runtime/src/shard/types.rs`, and `crates/vb_runtime/src/shard/lifecycle.rs` are the path from public submission to frame allocation/drive.
- Durable evidence: `crates/vb_storage/src/events.rs`, runtime journal adapter code, and CLI event projections may need alignment if `RunAdmission` gains fields.
- CLI/runtime integration: `crates/velvet_ballastics/src/main.rs`, `commands_journal.rs`, `commands_diff.rs`, `commands_ai_context.rs`, and related tests may be affected if accepted artifact envelopes become externally visible.

## Risks And Dependencies

- MASTER.md intended `AcceptedArtifact` has 15 verification gates and rich proof fields, while current storage code only has a 2-gate proof and a minimal artifact. `rust-contract` must either scope this bead to defining the envelope contract only or explicitly stage the migration.
- Current `submit_artifact` stores serialized workflow parts in `CompiledIrRecord.ir`; some red tests appear to expect `record.ir` to decode as `AcceptedArtifact`. This mismatch must be resolved in the contract before implementation.
- Runtime admission currently checks only digest presence for strict/journaled policies. That is insufficient for the MASTER.md rule that only verified artifacts run.
- Runtime `RunAccepted`/`RunAdmission` durability boundary is not the same as storage `submit_artifact` strict durability. The contract must separate artifact acceptance durability from run admission durability.
- `Relaxed` policy is intentionally permissive today. The contract must state whether accepted artifact envelopes are mandatory in relaxed mode or only in strict/journaled mode.
- `ShardCommand::Submit` and `SubmitWithInputs` carry raw compiled workflows; adding artifact-digest submission could be a behavior migration affecting many tests.
- `RunAdmission` event currently lacks input digest, available secret IDs, and admitted timestamp from MASTER.md Section 66.
- Any new record kind/magic affects storage compatibility, record family validation, CLI projections, and fuzz/proptest targets.
- Governance rules forbid runtime-core JSON/YAML/HTTP and require no unsafe, unwrap, panic, unchecked casts/indexing, or post-admission allocation in hot paths.

## Next-State Notes For rust-contract

- Define the exact envelope identity: is the accepted artifact payload stored inside `CompiledIrRecord.ir`, adjacent to it, or as a new record family?
- Define digest semantics precisely: source `workflow_digest`, compiled `ir_digest`, `action_contract_digest`, and storage key digest must not be conflated.
- Define minimum required fields for `AcceptedArtifact` v1 and whether `artifact_version = "velvet.artifact/v1"` and `workflow_version = "velvet-ballastics/v1"` are encoded as strings, enums, or constants.
- Define `VerificationProof` v1 as either the full MASTER.md proof or an explicitly staged subset; include a rule for gate count/status consistency.
- Define load-time validation: storage envelope header validation, postcard decode, internal digest checks, version check, proof consistency, capabilities shape, and resource budget bounds.
- Define runtime admission preconditions: artifact must load and validate, input digest/schema must be checked, capabilities must cover required capabilities, secrets must be present, frame capacity must be reserved, and no run state mutates before all preconditions pass.
- Define durable ordering: if `RunAccepted` and `RunAdmission` both exist, specify event order and strict/journaled acknowledgement semantics.
- Define migration behavior for `submit_direct` and `SubmitWithInputs` under `RuntimePolicy::Relaxed`, `Journaled`, and `Strict`.
- Define test obligations before State 5: storage roundtrip/corruption tests, runtime rejection tests for missing/malformed artifacts, integration test proving accepted artifact admission path, and a regression that raw submit is rejected when accepted artifacts are required.

STATUS: COMPLETE
