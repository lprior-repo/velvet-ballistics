# vb-my3b3 — qi37 checked admission fixtures

## Objective

Repair the 15 remaining failures in
`crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs` by
migrating hand-built legacy `AcceptedArtifact` fixtures (manual `gate_count`,
`durable`, `*_claimed` flags, `ArtifactEvidence::Unverified`) to the real
checked producer and typed evidence contract. No production code changes.

## Contract

- Artifacts are built only through `vb_storage::admission::prepare_compiled_artifact`
  (and `submit_artifact` for storage round-trip, already covered by `vb_9agmk`).
  Legacy `VerificationProof::new` / hand-built `AcceptedArtifact` fields are not
  admission authority and are not used as fixtures.
- Legacy display fields are not admission authority: a checked artifact has
  `verification.gate_count == 0` and `verification.durable == false`; the
  obsolete `REQUIRED_GATE_COUNT == 15` oracle is removed from assertions.
- Typed evidence (`ArtifactEvidence::Verified { schema_version: 3 }`) is the
  authority. `Unverified`, schema `2`, and unknown schema versions fail closed.
- The runtime verifier (`verify_accepted_artifact`) rejects tampered IR,
  rehashed envelopes, and policy upgrades; all such rejections surface through
  `AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" | "admission_policy" }`.
- Declaration-only `ObligationClosure::RuntimeValidated` marks checks the
  verifier actually ran. `OperatorAttested` is a distinct external attestation
  and remains intentionally absent from a checked ledger (empty operator
  attestation is intentional).
- Rejection payloads are preserved exactly: `CapabilityDenied { action, required,
  granted }`, `CapabilityCountMismatch { required_count, granted_count }`,
  `ArtifactDigestMismatch { requested, found }`, `ArtifactCertificateStale`,
  `ResourceCapacityExceeded`, `ArtifactNotFound`, `ArtifactEnvelopeDecodeFailed`.
- Rejected admissions allocate no run frame, insert no run state, emit no
  admission journal event, and consume the submit command.

## Scope

- `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs` (rewritten).
- `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.proptest-regressions`
  (stale seed file deleted; old parameter names no longer exist).
- `.beads/vb-my3b3/` metadata only. No other artifact fixture files touched.
