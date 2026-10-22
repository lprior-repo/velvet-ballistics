# vb-3v011 — Repair actual CLI run and relaxed artifact submission against typed admission evidence

## Domain

Unit of trust is the accepted artifact (`vb_storage::admission::AcceptedArtifact`), not the YAML
source. The compiler emits `CompiledWorkflow` whose `digest()` is the canonical compiled-IR digest
(`BLAKE3(postcard(WorkflowParts{ digest: ZERO, .. }))`) and whose `semantic_source_identity()` is
the normalized-source digest (`canonical_digest(source)`). These are distinct identities
(MASTER §77.2 / §77.4, line 5068: typed evidence supersedes blanket proof booleans).

Admission for `Strict`/`Journaled` re-verifies the artifact from hostile bytes via
`vb_storage::admission::verify_accepted_artifact[_for_policy]`. The artifact must carry a
schema-v3 `ArtifactEvidence::Verified` envelope; `ArtifactEvidence::Unverified`/`Packed`/schema-v2
are rejected (`MigrationRequired`/`UnsupportedSchemaVersion`/`ArtifactMalformed`).

## Causal chain (observed failure)

1. CLI `run --input-bin … --durability journaled` → `cmd_run` → `store_workflow_artifacts`
   → `run::submit_cli_compiled_artifact` (crates/velvet_ballistics/src/run.rs).
2. That function hand-builds a **legacy** `AcceptedArtifact` using
   `VerificationProof::new(zero, REQUIRED_GATE_COUNT, true)`, which sets
   `evidence = ArtifactEvidence::Unverified` and all `*_claimed` flags false.
3. It persists the envelope and then `run_compiled_workflow` submits to the runtime, which
   admits under `self.policy`. `StorageArtifactStore::load_accepted_artifact` decodes the
   envelope, `validate_accepted_artifact_envelope` → `verify_stored_artifact` →
   `verify_accepted_artifact_for_policy` → `checked_evidence_schema(Unverified)` returns
   `MigrationRequired { from: 1, to: 3 }`, mapped to `AdmissionError::ArtifactInvalidProofFlag`
   → `RuntimeError::AdmissionArtifactInvalid` → `"admission rejected: artifact invalid"`.
4. `finish_input_execution` and the 9 `cli_integration` failures all exercise this same
   `run`/`run-compiled` producer path. They use **genuine** `vb_compile::compile_workflow`
   output and therefore MUST execute, not be denied.

Secondary defect: `run_compiled::runtime_config_for_durability` leaves
`DurabilityMode::Journaled` on the `ShardConfig::default()` policy (`Strict`), while
`submit::submission_policy` and `submission_policy` semantics map `Journaled → Journaled`.
An artifact prepared under `Journaled` is then admitted under `Strict`, so
`verify_accepted_artifact_for_policy` fails the `admission_policy` binding. The durability→policy
mapping must be single-sourced.

## Fix contract

### Production (real bug)

- `run::submit_cli_compiled_artifact(journal, compiled, policy)` delegates to
  `vb_storage::submit_artifact(journal, compiled, policy)`, which runs
  `prepare_compiled_artifact` (typed schema-v3 evidence, checksum-validated, capability/contract
  bound). No hand-built envelope, no `VerificationProof::new` in the producer path.
- Thread `policy` through `store_workflow_artifacts` and `store_compiled_artifact`; callers pass
  `submit::submission_policy(durability)` (single source of truth).
- `runtime_config_for_durability` maps `Journaled → Journaled`, `Strict → Strict`,
  `None → Relaxed` via the same single-source helper.

### Tests (my 3 assigned files only)

- `admission_evidence_integration` `restart_lookup_finds_persisted_header`,
  `journal_persistence_survives_runtime_drop_and_reopen`, and `run_without_artifact_under_relaxed_policy`
  currently construct `set_const_finish_workflow([0x..;32])` — a **forged** digest — and expect
  `submit_artifact(Relaxed)` to succeed (obsolete "relaxed skips checksum" contract). Rewrite them
  to compile genuine workflows via `vb_compile::compile_workflow`, preserving their
  persistence/lookup/restart intent; genuine artifacts succeed under Relaxed.
- Add a tamper regression test asserting a forged-digest workflow is rejected by
  `submit_artifact` under **both** `Relaxed` and `Strict` (hostile bytes reject across policies).

### Out of scope (LOCAL5090 owns)

`accepted_artifact_red_phase`, `core_yaml`, `qi37`, `lp2v`, `9agmk`, `ssei` fixtures are NOT edited.
Any production API/contract change that affects them is reported in
`.beads/vb-3v011/integration-note.md`.

## Invariants preserved

- Typed schema-v3 evidence ledger (`VerificationLedgerEntry`, `ArtifactVerificationEvidence`).
- `semantic_source_identity` (`WorkflowParts`) distinct from compiled `digest`.
- Checksums: canonical-IR digest recompute in `checked_parts`; envelope digest in
  `accepted_artifact_digest`.
- Capability strict-equality admission (`VERUS-CARD-003`), action ABI/registry validation.
- Stored source identity: `WorkflowSourceRecord.digest = BLAKE3(source)`; `AcceptedArtifact.source_digest`
  binds the workflow digest (unchanged semantics).
- `run_attempt`/ticket authority paths are untouched.

## Acceptance

`python3 /tmp/vb-run-gate.py 600 .beads/vb-3v011/evidence/cli-regressions cargo test -p velvet-ballistics --test cli_integration --test finish_input_execution --test admission_evidence_integration`
must exit 0 with all manifest test names passing; genuine compiled artifacts execute and forged
artifacts reject across strict/relaxed.
