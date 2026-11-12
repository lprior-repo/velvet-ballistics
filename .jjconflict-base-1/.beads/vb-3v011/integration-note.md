# vb-3v011 integration note (for LOCAL5090)

## Scope boundary

vb-3v011 changed **only** `crates/velvet_ballistics/src/{run,run_compiled,run_compiled_runtime,submit}.rs`
and `crates/velvet_ballistics/tests/admission_evidence_integration/{chunk_001,chunk_002}.rs`.

No public crate API changed. Every touched item is `pub(crate)` internal to the
`velvet-ballistics` CLI crate, which no other crate depends on:

- `run::submit_cli_compiled_artifact(journal, compiled, policy)` — signature gained a
  `policy` argument; body now delegates to `vb_storage::submit_artifact` (typed schema-v3
  evidence) instead of hand-building a legacy `VerificationProof::new` envelope.
- `run::store_workflow_artifacts` / `run_compiled_runtime::store_compiled_artifact` — thread `policy`.
- `submit::submission_policy` — visibility `fn` → `pub(crate)`.
- `run_compiled::runtime_config_for_durability` — `Journaled` now maps to
  `RuntimePolicy::Journaled` (previously fell through to `Strict` default).

## Effect on LOCAL5090 fixtures

`accepted_artifact_red_phase`, `core_yaml`, `qi37`, `lp2v`, `9agmk`, `ssei` are test
files in `vb_storage`, `vb_compile`, and `workspace_tests`; none import
`velvet_ballistics` internals, so this diff does not change their compilation or runtime
behavior. Their observed failures are driven by the pre-existing typed-evidence migration
already on the union (schema-v3 `ArtifactEvidence`, `VerificationProof::new` now emits
`ArtifactEvidence::Unverified` and zeroes claimed flags) — **not** by vb-3v011.

## Authoritative contract reference

MASTER §77.4 (line 5068) and §77.5: boolean proof flags are insufficient; every accepted
artifact must carry a schema-v3 verification ledger. Consumers must independently run
`verify_accepted_artifact`. Genuine compiler-produced artifacts execute; hostile forged
digests (placeholder `WorkflowDigest` ≠ canonical `BLAKE3(postcard(WorkflowParts{digest: ZERO}))`)
reject under **both** Relaxed and Strict via `JournalError::ArtifactChecksumMismatch`.
